use std::net::{Ipv4Addr};
use std::ops::Index;

#[derive(Debug, Clone)]
pub struct Player {
    model: String,
    address: Ipv4Addr,
    number: u8,
}

impl Player {
    pub fn new(model: String, number: u8, address: Ipv4Addr) -> Self {
        Self { model: model, number: number, address: address }
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

    pub fn get_mut(&mut self, address: &Ipv4Addr) -> Option<&mut Player> {
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
    use crate::player::{PlayerCollection, Player};
    use std::net::Ipv4Addr;

    #[test]
    fn it_support_pushing() {
        let mut players = PlayerCollection::new();
        players.push(Player {
            number: 0x01,
            address: Ipv4Addr::new(0x01, 0x00, 0x00, 0x01),
            model: String::from("XDJ-700")
        });

        assert_eq!(players.len(), 1);
    }

    #[test]
    fn it_can_update_matching_players() {
        let mut players = PlayerCollection::new();
        players.push(Player {
            number: 0x01,
            address: Ipv4Addr::new(0x01, 0x00, 0x00, 0x01),
            model: String::from("XDJ-700")
        });
        assert_eq!(players[0].number, 0x01);

        players.add_or_update(Player {
            number: 0x02,
            address: Ipv4Addr::new(0x01, 0x00, 0x00, 0x01),
            model: String::from("XDJ-700")
        });
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].number, 0x02);

        players.add_or_update(Player {
            number: 0x03,
            address: Ipv4Addr::new(0x01, 0x00, 0x00, 0x05),
            model: String::from("XDJ-700")
        });
        assert_eq!(players.len(), 2);
        assert_eq!(players[0].number, 0x02);
        assert_eq!(players[1].number, 0x03);
    }
}

