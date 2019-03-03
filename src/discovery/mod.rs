pub mod event;
pub mod listener;

use std::net::{UdpSocket, SocketAddr};
use crate::player::{Player, PlayerCollection};
use std::str;
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

pub struct Options {
    pub listen_address: String,
}

impl Options {
    fn listen_address(&self) -> String {
        self.listen_address.to_owned()
    }
}



pub enum PlayerEvent {
    Annoncement(Player),
    Disconnect(Player),
    Error(String),
}

#[allow(dead_code)]
fn parse_udp_package(
    size: usize,
    source: SocketAddr,
    data: &[u8]
) -> PlayerEvent {
    if size == 54 {
        PlayerEvent::Annoncement(Player {
            address: source,
            model: str::from_utf8(&data[12..19]).unwrap().to_owned(),
            number: data[36].to_owned(),
            linked: false,
        })
    } else {
        PlayerEvent::Error(String::from("I have no clue"))
    }
}
