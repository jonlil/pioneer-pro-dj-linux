use std::net::{UdpSocket, SocketAddr};
use crate::player::{Player, PlayerCollection};
use std::str;
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

mod test {}


pub struct Options {
    pub listen_address: String,
}

impl Options {
    fn listen_address(&self) -> String {
        self.listen_address.to_owned()
    }
}

#[allow(dead_code)]
pub fn run(
    players: &PlayerCollection,
    options: Options
) -> std::io::Result<()> {
    let socket = UdpSocket::bind(options.listen_address())?;
    let (tx, rx): (Sender<PlayerEvent>, Receiver<PlayerEvent>) = mpsc::channel();
    eprintln!("Running discovery");

    let discovery_tx = tx.clone();
    let discovery_handler = thread::spawn(move || loop {
        discovery_tx.send(recv_from(&socket)).unwrap();
    });

    loop {
        match rx.recv().unwrap() {
            PlayerEvent::Annoncement(player) => {
                eprintln!("#{:?}", player);
            },
            _ => {}
        }
    }

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
        })
    } else {
        PlayerEvent::Error(String::from("I have no clue"))
    }
}
