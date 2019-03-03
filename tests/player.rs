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
            number: 1u8,
        };

        assert_eq!(player.model, "XDJ-700".to_string());
        assert_eq!("169.254.17.30:50000".parse(), Ok(socket));
        assert_eq!(socket.port(), 50000);
    }

    #[test]
    fn it_can_find_player() {
        let mut players = PlayerCollection::new();

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

