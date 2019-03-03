use std::net::{SocketAddr};
use std::error::{Error};

#[derive(Debug, Clone)]
pub struct Player {
    pub model: String,
    pub address: SocketAddr,
    pub number: u8,
}

// impl Player {
//     pub fn verify(&self) -> Result<(), Error> {
//        Err()
//     }
// }

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

    pub fn get(&self, address: &SocketAddr) -> Option<&Player> {
        self.players.iter().find(|&p| p.address == *address)
    }

    pub fn add_or_update(&mut self, player: Player) {
        match self.get(&player.address) {
            Some(_) => {},
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
