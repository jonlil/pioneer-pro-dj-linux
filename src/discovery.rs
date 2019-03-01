use std::net::{UdpSocket, SocketAddr};
use crate::player::{Player, PlayerCollection};
use std::str;
use std::thread;

mod test {}


pub struct Options {
    pub listen_address: String,
}

impl Options {
    fn listen_address(&self) -> String {
        self.listen_address.to_owned()
    }
}

pub fn run(
    players: &PlayerCollection,
    options: Options
) -> std::io::Result<()> {
    // bind socket in thread
    let socket = UdpSocket::bind(options.listen_address())?;

    eprintln!("Running discovery");

    let discovery_handler = thread::spawn(move || loop {
        match recv_from(&socket) {
            PlayerEvent::Annoncement(player) => {
                eprintln!("#{:?}", player);
            },
            _ => {}
        }
    });

    discovery_handler.join().expect("Discovery handler thread has paniced!");

    Ok(())
}

fn recv_from(socket: &UdpSocket) -> PlayerEvent {
    let mut buffer = [0u8; 512];
    match socket.recv_from(&mut buffer) {
        Ok((number_of_bytes, source)) => {
            parse_udp_package(number_of_bytes, source, &buffer[..number_of_bytes])
        },
        Err(error) => PlayerEvent::Error(error.to_string()),
    }
}

enum PlayerEvent {
    Annoncement(Player),
    Disconnect(Player),
    Error(String),
}

fn parse_udp_package(
    size: usize,
    source: SocketAddr,
    data: &[u8]
) -> PlayerEvent {
    if size == 54 {
        PlayerEvent::Annoncement(Player {
            address: source,
            model: str::from_utf8(&data[12..19]).unwrap().to_owned(),
            number: str::from_utf8(&data[37..38]).unwrap().to_owned(),
        })
    } else {
        PlayerEvent::Error(String::from("I have no clue"))
    }
}
