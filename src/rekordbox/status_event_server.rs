use std::convert::TryFrom;
use std::net::{UdpSocket, SocketAddr};
use std::sync::{Arc, Mutex};
use bytes::{Bytes};
use std::io;
use std::thread;
use std::time::Duration;

use super::packets::{StatusPacket, StatusPacketType, StatusContentType, RekordboxReply};

#[derive(Debug)]
pub struct StatusEventServer {
    socket: Arc<Mutex<UdpSocket>>,
}

impl StatusEventServer {
    pub fn new(socket: Arc<Mutex<UdpSocket>>) -> StatusEventServer {
        StatusEventServer {
            socket,
        }
    }

    fn sleep(&self) {
        thread::sleep(Duration::from_millis(150));
    }

    fn process_packet(&self, packet: StatusPacket) -> Option<StatusPacket> {
        if packet.kind() == &StatusPacketType::Cdj {
            return None;
        }

        match packet.kind() {
            StatusPacketType::RekordboxHello => {
                Some(StatusPacket::new(
                    StatusPacketType::RekordboxReply,
                    1,
                    1,
                    StatusContentType::RekordboxReply(RekordboxReply {
                        name: "Term DJ".to_string(),
                    })
                ))
            },
            _ => {
                eprintln!("Other event {:#?}", packet);
                None
            },
        }
    }

    fn send_to(&self, response: (StatusPacket, SocketAddr)) -> Result<(), &'static str> {
        match self.socket.lock() {
            Ok(socket) => {
                match socket.send_to(&Bytes::from(response.0), response.1) {
                    Ok(_number_of_bytes) => {},
                    Err(err) => eprintln!("Failed sending status packet event: {:?}", err),
                };
            },
            Err(_err) => {},
        };

        Ok(())
    }

    pub fn run(&self) -> io::Result<()> {
        loop {
            match self.recv_from() {
                Ok((data, peer)) => {
                    if let Some(response) = self.process_packet(data) {
                        self.send_to((response, peer));
                    }
                },
                Err(_err) => {},
            }
            self.sleep();
        }
    }

    fn recv_from(&self, ) -> io::Result<(StatusPacket, SocketAddr)> {
        match self.socket.lock() {
            Ok(socket) => {
                let mut buffer = [0u8; 1024];

                match socket.recv_from(&mut buffer) {
                    Ok((number_of_bytes, peer)) => {
                        match StatusPacket::try_from(&buffer[..number_of_bytes]) {
                            Ok(packet) => Ok((packet, peer)),
                            Err(err) => {
                                eprintln!("Failed decoding StatusPacket: {:?}\n{:?}", err, &buffer[..number_of_bytes]);
                                Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed decoding data."))
                            }
                        }
                    },
                    Err(ref err) if err.kind() != std::io::ErrorKind::WouldBlock => {
                        println!("Something went wrong: {}", err);
                        Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed reading from socket."))
                    },
                    _ => {
                        Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed reading from socket."))
                    }
                }
            },
            Err(_err) => Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed reading from socket.")),
        }
    }
}
