mod rekordbox;
mod utils;
mod component;

extern crate rand;
extern crate pnet;

use std::thread;
use std::io;
use std::time::{Duration};
use crate::rekordbox::player::{PlayerCollection};
use crate::rekordbox::message as Message;
use crate::utils::network::{
    find_interface,
};
use crate::rekordbox::client::random_broadcast_socket;

fn main() -> Result<(), io::Error> {
    let mut app = component::App {
        network: None,
        players: PlayerCollection::new(),
    };

    app.run();

    eprintln!("{:?}", app.players);
    app.network = find_interface(app.players[0].address());

    if let Some(network) = app.network {
        let thread_sleep = Duration::from_millis(50);
        for sequence in 0x01 ..= 0x03 {
            random_broadcast_socket(&network, Message::DiscoveryInitial::new(&network, sequence).into());
            thread::sleep(thread_sleep);
        }
        for sequence in 0x01..=0x06 {
            for index in 1..=6 {
                random_broadcast_socket(&network, Message::DiscoverySequence::new(&network, sequence, index).into());
                thread::sleep(thread_sleep);
            }
        }
        random_broadcast_socket(&network, Message::ApplicationBroadcast::new(&network).into());

        thread::spawn(move || {
            let threehoundred_millis = Duration::from_millis(300);
            loop {
                thread::sleep(threehoundred_millis);
            }
        }).join().expect("Failed to blaha");
    }

    Ok(())
}
