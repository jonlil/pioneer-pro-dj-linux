use std::net::{SocketAddr};
use std::error::{Error};

#[cfg(test)]
mod tests {
    use crate::player::{Player, PlayerIter};
    use std::net::{SocketAddr,IpAddr, Ipv4Addr};

    #[test]
    fn it_can_create_player_instance() {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(169, 254, 17, 30)), 50000);
        let player = Player {
            model: "XDJ-700".to_string(),
            address: socket,
        };

        assert_eq!(player.model, "XDJ-700".to_string());
        assert_eq!("169.254.17.30:50000".parse(), Ok(socket));
        assert_eq!(socket.port(), 50000);
    }

    #[test]
    fn it_can_find_player() {
        let mut players = PlayerIter::new();

        let playerA = Player {
            model: "XDJ-700".to_string(),
            address: SocketAddr::new(IpAddr::V4(
                Ipv4Addr::new(169, 254, 17, 30)
            ), 50000),
        };
        let playerB = Player {
            model: "XDJ-700".to_string(),
            address: SocketAddr::new(IpAddr::V4(
                Ipv4Addr::new(169, 254, 17, 31)
            ), 50000),
        };

        players.push(playerA);
        players.push(playerB);

        assert_eq!(players.len(), 2);
        assert_eq!(
            players.get(&SocketAddr::new(IpAddr::V4(
                Ipv4Addr::new(169, 254, 17, 30)
            ), 50000)).unwrap(),
            &Player {
                model: "XDJ-700".to_string(),
                address: SocketAddr::new(IpAddr::V4(
                    Ipv4Addr::new(169, 254, 17, 30)
                ), 50000),
            }
        );
    }

    #[test]
    fn it_can_add_players() {
        let mut players = PlayerIter::new();

        players.push(Player {
            model: "XDJ-700".to_string(),
            address: SocketAddr::new(IpAddr::V4(
                Ipv4Addr::new(169, 254, 17, 30)
            ), 50000),
        });

        players.add_or_update(Player {
            model: "XDJ-700".to_string(),
            address: SocketAddr::new(IpAddr::V4(
                Ipv4Addr::new(169, 254, 17, 30)
            ), 50000),
        });

        assert_eq!(players.len(), 1);

        // Find a new player and add it to the list
        players.add_or_update(Player {
            model: "XDJ-700".to_string(),
            address: SocketAddr::new(IpAddr::V4(
                Ipv4Addr::new(169, 254, 17, 31)
            ), 50000),
        });
        assert_eq!(players.len(), 2);
    }
}

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
