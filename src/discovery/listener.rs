use std::net::{SocketAddr, UdpSocket};
use crate::discovery::event::{Event};
use crate::player::{Player};
use std::str;

struct Config {
    address: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            address: String::from("0.0.0.0:50000"),
        }
    }
}

pub struct DiscoveryListener {
    socket: UdpSocket,
}

impl DiscoveryListener {
    pub fn new() -> Self {
        let config = Config::default();
        let mut socket = UdpSocket::bind(config.address)
            .unwrap();

        Self {
            socket: socket,
        }
    }

    pub fn receive(&self) -> Event {
        self.recv_from()
    }

    fn recv_from(&self) -> Event {
        let mut buffer = [0u8; 512];

        match self.socket.recv_from(&mut buffer) {
            Ok(data) => self.handle_message(&buffer[..data.0], data),
            Err(error) => Event::Error(error.to_string())
        }
    }

    fn handle_message(&self, buffer: &[u8], metadata: (usize, SocketAddr)) -> Event {
        if metadata.0 == 54 {
            Event::Annoncement(Player {
                address: metadata.1,
                model: str::from_utf8(&buffer[12..19]).unwrap().to_owned(),
                number: buffer[36].to_owned(),
            })
        } else {
            Event::Error(String::from("Unable to parse discovery package."))
        }
    }
}
