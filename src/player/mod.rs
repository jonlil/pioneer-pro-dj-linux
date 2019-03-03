use std::net::{SocketAddr, IpAddr, UdpSocket};

#[derive(Debug, Clone)]
pub struct Player {
    pub model: String,
    pub address: SocketAddr,
    pub number: u8,
    pub linked: bool,
}

impl Player {
    pub fn ip(&self) -> IpAddr {
        return self.address.ip()
    }

    pub fn link(&mut self) {
        let mut socket = UdpSocket::bind("0.0.0.0:50002").expect("Could not bind player communication port");
        let buffer = [0u8; 296];
        match socket.send_to(&buffer, format!("{}:50002", self.ip())) {
            Ok(number_of_bytes) => {
                eprintln!("#{:?}", number_of_bytes);
            },
            Err(error) => {
                eprintln!("#{:?}", error);
            }
        };
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
        Self {
            players: Vec::new(),
        }
    }

    pub fn iter(&mut self) -> PlayerIter {
        PlayerIter {
            players: self.players.to_vec()
        }
    }

    pub fn push(&mut self, player: Player) {
        self.players.push(player);
    }

    pub fn get_mut(&mut self, address: &SocketAddr) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.address == *address)
    }

    pub fn add_or_update(&mut self, player: Player) {
        match self.get_mut(&player.address) {
            Some(mut p) => {
                p.number = player.number;
            },
            None => {
                self.push(player);
            },
        }
    }

    pub fn link(&mut self) {
        for player in self.players.iter_mut() {
            player.link();
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
