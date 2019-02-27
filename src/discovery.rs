use std::net::{UdpSocket, SocketAddr};
use crate::player::{Player, PlayerIter};
use std::str;

mod test {}

pub struct PlayerDiscovery {}

impl PlayerDiscovery {
    pub fn run(
        socket: &mut UdpSocket,
        players: &mut PlayerIter
    ) -> std::io::Result<()> {
        eprintln!("Running discovery");

        loop {
            // Set buffer size, larger packages will be discarded.
            let mut buf = [0; 100];

            // Read from network
            let (number_of_bytes, src) = socket.recv_from(&mut buf)?;

            // reassign buffer with read bytes
            let buf = &mut buf[..number_of_bytes];
            match parse_udp_package(number_of_bytes, src, &buf) {
                PlayerEvent::Annoncement(player) => {
                    players.add_or_update(player);
                },
                _ => {}
            }

            eprintln!("#{:?}", players);
        }

        Ok(())
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
            model: str::from_utf8(&data[12..19]).unwrap().to_owned()
        })
    } else {
        PlayerEvent::Error(String::from("I have no clue"))
    }
}
