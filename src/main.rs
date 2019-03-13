mod rekordbox;
mod utils;
mod player;

extern crate rand;
extern crate pnet;

use pnet::datalink::interfaces;
use std::thread;
use std::net::{UdpSocket, ToSocketAddrs, IpAddr};
use std::io;
use std::time::{Duration};
use rand::Rng;
use std::str;
use crate::rekordbox::{
    SOFTWARE_IDENTIFICATION,
    APPLICATION_NAME,
    RekordboxEventHandler,
    RekordboxMessage,
};
use crate::player::{PlayerCollection};
use crate::utils::network::{
    find_interface,
    PioneerNetwork,
};

pub fn send_data<A: ToSocketAddrs>(
    socket: &UdpSocket,
    addr: A,
    data: PioneerMessage
) {
    match socket.send_to(&to_socket_package(data).as_ref(), addr) {
        Ok(number_of_bytes) => {
            eprintln!("{:?}", number_of_bytes);
        }
        Err(err) => {
            eprintln!("{:?}", err.to_string());
        }
    }
}

fn to_socket_package(item: Vec<Vec<u8>>) -> Vec<u8> {
    item.into_iter().flatten().collect::<Vec<u8>>()
}

fn random_broadcast_socket(address: &PioneerNetwork, data: PioneerMessage) {
    let port = rand::thread_rng().gen_range(45000, 55000);
    let socket = UdpSocket::bind((address.ip(), port)).unwrap();
    socket.set_broadcast(true).unwrap();
    send_data(&socket, (address.broadcast(), 50000), data);
}

struct App<'a> {
    listening_address: Option<&'a PioneerNetwork>,
    players: PlayerCollection,
}

fn main() -> Result<(), io::Error> {
    let mut app = App {
        listening_address: None,
        players: PlayerCollection::new(),
    };

    let threehoundred_millis = Duration::from_millis(300);
    let socket = UdpSocket::bind(("0.0.0.0", 50000)).unwrap();
    socket.set_broadcast(true).unwrap();

    loop {
        let mut buffer = [0u8; 512];
        match socket.recv_from(&mut buffer) {
            Ok(metadata) => {
                let buffer = &buffer[..metadata.0];
                match RekordboxEventHandler::parse(buffer, metadata) {
                    Ok(RekordboxMessage::Player(player)) => {
                        app.players.add_or_update(player);
                    }
                    Err(error) => eprintln!("{:?}", error),
                    _ => ()
                }
            }
            Err(error) => {
                eprintln!("Failed reading broadcast socket: #{:?}", error);
            }
        }
        if app.players.len() >= 2 {
            break;
        }
        thread::sleep(threehoundred_millis);
    }

    eprintln!("{:?}", app.players);
    let network = find_interface(app.players[0].address());

    if let Some(network) = network {
        let thread_sleep = Duration::from_millis(50);
        for sequence in 0x01 ..= 0x03 {
            random_broadcast_socket(&network, Message::phase1(sequence, &network));
            thread::sleep(thread_sleep);
        }
        for sequence in 0x01..=0x06 {
            for index in 1..=6 {
                random_broadcast_socket(&network, Message::phase2(sequence, index, &network));
                thread::sleep(thread_sleep);
            }
        }
        random_broadcast_socket(&network, Message::broadcast(&network));

        thread::spawn(move || {
            let threehoundred_millis = Duration::from_millis(300);
            loop {
                thread::sleep(threehoundred_millis);
            }
        }).join().expect("Failed to blaha");
    }

    Ok(())
}

type PioneerMessage = Vec<Vec<u8>>;

struct Message;
impl Message {
    pub fn broadcast(network: &PioneerNetwork) -> PioneerMessage {
        vec![
            SOFTWARE_IDENTIFICATION.to_vec(),
            vec![0x06, 0x00],
            APPLICATION_NAME.to_vec(),
            vec![0x01,0x03,0x00],
            vec![0x36,0x11,0x01],
            <[u8; 6]>::from(network.mac_address()).to_vec(),
            match network.ip() {
                IpAddr::V4(ip) => ip.octets().to_vec(),
                IpAddr::V6(ip) => vec![],
            },
            vec![0x01,0x01,0x00,0x00,0x04,0x08]
        ]
    }

    pub fn phase1(sequence: u8, network: &PioneerNetwork) -> PioneerMessage {
        vec![
            SOFTWARE_IDENTIFICATION.to_vec(),
            vec![0x00,0x00],
            APPLICATION_NAME.to_vec(),
            vec![0x01, 0x03, 0x00, 0x2c, sequence, 0x04],
            <[u8; 6]>::from(network.mac_address()).to_vec(),
        ]
    }

    pub fn phase2(sequence: u8, index: i32, network: &PioneerNetwork) -> PioneerMessage {
        vec![
            SOFTWARE_IDENTIFICATION.to_vec(),
            vec![0x02, 0x00],
            APPLICATION_NAME.to_vec(),
            vec![0x01, 0x03, 0x00, 0x32],
            match network.ip() {
                IpAddr::V4(ip) => ip.octets().to_vec(),
                IpAddr::V6(ip) => vec![],
            },
            <[u8; 6]>::from(network.mac_address()).to_vec(),
            vec![Self::map_sequence_byte(index), sequence],
            vec![0x04, 0x01]
        ]
    }

    fn map_sequence_byte(index: i32) -> u8 {
        if index == 1 {
            0x11
        } else if index == 2 {
            0x12
        } else if index == 3 {
            0x29
        } else if index == 4 {
            0x2a
        } else if index == 5 {
            0x2b
        } else {
            0x2c
        }
    }
}
