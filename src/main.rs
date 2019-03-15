mod rekordbox;
mod utils;
mod component;

extern crate rand;
extern crate pnet;

use std::thread;
use std::net::{UdpSocket, ToSocketAddrs};
use std::io;
use std::time::{Duration};
use rand::Rng;
use crate::rekordbox::player::{PlayerCollection};
use crate::rekordbox::message as Message;
use crate::utils::network::{
    find_interface,
    PioneerNetwork,
};
use crate::rekordbox::client::Client;

pub fn send_data<A: ToSocketAddrs>(
    socket: &UdpSocket,
    addr: A,
    data: Message::RekordboxMessageType
) {
    match socket.send_to(&data.as_ref(), addr) {
        Ok(number_of_bytes) => {
            eprintln!("{:?}", number_of_bytes);
        }
        Err(err) => {
            eprintln!("{:?}", err.to_string());
        }
    }
}

fn random_broadcast_socket(address: &PioneerNetwork, data: Message::RekordboxMessageType) {
    let port = rand::thread_rng().gen_range(45000, 55000);
    let socket = UdpSocket::bind((address.ip(), port)).unwrap();
    socket.set_broadcast(true).unwrap();
    send_data(&socket, (address.broadcast(), 50000), data);
}

fn main() -> Result<(), io::Error> {
    let mut app = component::App {
        network: None,
        players: PlayerCollection::new(),
    };

    let threehoundred_millis = Duration::from_millis(300);

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
