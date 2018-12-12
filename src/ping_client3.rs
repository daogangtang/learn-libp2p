extern crate env_logger;
extern crate futures;
extern crate libp2p;
extern crate tokio;

use futures::prelude::*;
use libp2p::{
    NetworkBehaviour, Transport,
    core::upgrade::{self, OutboundUpgradeExt},
    secio,
    mplex,
    yamux,
    tokio_codec::{FramedRead, LinesCodec}
};
use libp2p::InboundUpgradeExt;

fn main() {
    env_logger::init();

    // Create a random PeerId
    let local_key = secio::SecioKeyPair::ed25519_generated().unwrap();
    let local_pub_key = local_key.to_public_key();
    println!("Local peer id: {:?}", local_pub_key.clone().into_peer_id());

    // Set up a an encrypted DNS-enabled TCP Transport over the Mplex protocol
    let transport = libp2p::CommonTransport::new()
        .with_upgrade(secio::SecioConfig::new(local_key))
        .and_then(move |out, cp| {
            let peer_id1 = out.remote_key.into_peer_id();
            let peer_id2 = peer_id1.clone();
            let upgrade = libp2p::yamux::Config::default()
                .map_inbound(move |muxer| (peer_id1, muxer))
                .map_outbound(move |muxer| (peer_id2, muxer));
            upgrade::apply(out.stream, upgrade, cp).map_err(|e| e.into_io_error())
        });

    // Create a Swarm to manage peers and events
    let mut swarm = {
        let behaviour = libp2p::ping::PeriodicPing::new();

        libp2p::Swarm::new(transport, behaviour, libp2p::core::topology::MemoryTopology::empty(), local_pub_key)
    };

    // Listen on all interfaces and whatever port the OS assigns
    let addr = libp2p::Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();
    println!("Listening on {:?}", addr);

    // Reach out to another node if specified
    if let Some(to_dial) = std::env::args().nth(1) {
        let dialing = to_dial.clone();
        match to_dial.parse() {
            Ok(to_dial) => {
                match libp2p::Swarm::dial_addr(&mut swarm, to_dial) {
                    Ok(_) => {
                        println!("Dialed {:?}", dialing);
                        //swarm.ping.ping(());
                    },
                    Err(e) => println!("Dial {:?} failed: {:?}", dialing, e)
                }
            },
            Err(err) => println!("Failed to parse address to dial: {:?}", err),
        }
    }

    // Kick it off
    let task = futures::future::poll_fn(move || -> Result<_, ()> {
        loop {
            match swarm.poll().expect("Error while polling swarm") {
                Async::Ready(Some(x)) => {
                    println!("{:?}", x);
                },
                Async::Ready(None) | Async::NotReady => break,
            }
        }

        Ok(Async::NotReady)
    });

    tokio::run(task);

}
