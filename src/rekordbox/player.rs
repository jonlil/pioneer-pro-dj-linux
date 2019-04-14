use std::net::{IpAddr, UdpSocket, TcpStream};
use std::ops::Index;
use std::io::{self, Read};
use crate::rekordbox::message as Message;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Player {
    model: String,
    address: IpAddr,
    number: u8,
    linking: bool,
}

impl Player {
    pub fn new(model: String, number: u8, address: IpAddr) -> Self {
        Self { model: model, number: number, address: address, linking: false }
    }

    pub fn address(&self) -> IpAddr {
        self.address
    }

    pub fn link(&mut self, socket: &UdpSocket) -> Result<(usize), io::Error> {
        let data: Vec<u8> = Message::ApplicationLinkRequest::new().into();
        self.linking = true;
        self.connect_library();
        socket.send_to(data.as_ref(), (self.address, 50002))
    }

    pub fn is_linking(&self) -> bool {
        self.linking
    }

    pub fn set_linking(&mut self, val: bool) {
        self.linking = val;
    }

    pub fn connect_library(&mut self) {
        let address = self.address.clone();
        thread::spawn(move || {
            let mut stream = TcpStream::connect((address, 12523)).unwrap();

            loop {
                let mut buffer = [0; 1024];
                match stream.read(&mut buffer) {
                    Ok(number_of_bytes) => {
                        eprintln!(
                            "Received TCP Package: {:?}",
                            String::from_utf8_lossy(&buffer[..number_of_bytes])
                        );
                    },
                    Err(err) => eprintln!("TCPError: {:?}", err),
                }

                thread::sleep(Duration::from_millis(500));
            }
        });

    }
}

impl PartialEq for Player {
    fn eq(&self, other: &Player) -> bool {
        self.address == other.address
    }
}

#[derive(Debug)]
pub struct PlayerCollection {
    players: Vec<Player>,
}

impl PlayerCollection {
    pub fn new() -> Self {
        Self { players: Vec::new() }
    }

    pub fn iter(&mut self) -> PlayerIter {
        PlayerIter {
            players: self.players.to_vec()
        }
    }

    pub fn len(&self) -> usize {
        return self.players.len()
    }

    pub fn push(&mut self, player: Player) {
        self.players.push(player);
    }

    pub fn get_mut(&mut self, address: &IpAddr) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.address == *address)
    }

    pub fn add_or_update(&mut self, player: Player) {
        match self.get_mut(&player.address) {
            Some(mut p) => {
                p.number = player.number;

                // Resetting of this state should be handled by some timer
                // TODO: Implement unresponsive player checker
                if p.linking == false && player.linking == true {
                    p.linking = player.linking;
                }
            },
            None => {
                self.push(player);
            },
        }
    }
}

#[derive(Debug)]
pub struct PlayerIter {
    players: Vec<Player>,
}

impl Iterator for PlayerIter {
    type Item = Player;

    fn next(&mut self) -> Option<Self::Item> {
        self.players.pop()
    }
}

impl ExactSizeIterator for PlayerIter {
    fn len(&self) -> usize {
        self.players.len()
    }
}

impl Index<usize> for PlayerCollection {
    type Output = Player;

    fn index(&self, idx: usize) -> &Player {
        &self.players[idx]
    }
}

#[cfg(test)]
mod tests {
    use crate::rekordbox::player::{PlayerCollection, Player};
    use std::net::{Ipv4Addr, IpAddr};

    #[test]
    fn it_support_pushing() {
        let mut players = PlayerCollection::new();
        players.push(Player {
            linking: false,
            number: 0x01,
            address: IpAddr::V4(Ipv4Addr::new(0x01, 0x00, 0x00, 0x01)),
            model: String::from("XDJ-700")
        });

        assert_eq!(players.len(), 1);
    }

    #[test]
    fn it_can_update_matching_players() {
        let mut players = PlayerCollection::new();
        players.push(Player {
            linking: false,
            number: 0x01,
            address: IpAddr::V4(Ipv4Addr::new(0x01, 0x00, 0x00, 0x01)),
            model: String::from("XDJ-700")
        });
        assert_eq!(players[0].number, 0x01);

        players.add_or_update(Player {
            linking: false,
            number: 0x02,
            address: IpAddr::V4(Ipv4Addr::new(0x01, 0x00, 0x00, 0x01)),
            model: String::from("XDJ-700")
        });
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].number, 0x02);

        players.add_or_update(Player {
            linking: true,
            number: 0x02,
            address: IpAddr::V4(Ipv4Addr::new(0x01, 0x00, 0x00, 0x01)),
            model: String::from("XDJ-700")
        });
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].linking, true);

        players.add_or_update(Player {
            linking: false,
            number: 0x03,
            address: IpAddr::V4(Ipv4Addr::new(0x01, 0x00, 0x00, 0x05)),
            model: String::from("XDJ-700")
        });
        assert_eq!(players.len(), 2);
        assert_eq!(players[0].number, 0x02);
        assert_eq!(players[1].number, 0x03);
    }
}
