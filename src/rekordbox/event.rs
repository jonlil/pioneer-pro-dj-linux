use crate::rekordbox::player::Player;
use crate::rekordbox::SOFTWARE_IDENTIFICATION;
use std::net::{Ipv4Addr, SocketAddr, IpAddr};
use std::str;

#[derive(Debug, PartialEq)]
pub enum Event {
    PlayerBroadcast(Player),
    ApplicationBroadcast,
    Unknown,
    Error,
    PlayerLinkingWaiting(Player),
    InitiateLink,
    Tick,
}

pub struct EventParser;
impl EventParser {
    pub fn is_rekordbox_event(data: &[u8]) -> bool {
        if data.len() < SOFTWARE_IDENTIFICATION.len() {
            return false
        }

        SOFTWARE_IDENTIFICATION == data[..SOFTWARE_IDENTIFICATION.len()]
    }

    fn parse_model_name(buffer: &[u8]) -> String {
        str::from_utf8(&buffer[12..=31])
            .unwrap()
            .trim_end_matches('\u{0}')
            .to_string()
    }

    fn get_type(buffer: &[u8], metadata: (usize, SocketAddr)) -> Event {
        if !Self::is_rekordbox_event(&buffer) {
            return Event::Unknown
        }

        let number_of_bytes = metadata.0;
        if number_of_bytes == 54 && &buffer[10..=11] == &[0x06, 0x00] {
            if &buffer[32..=34] == &[0x01, 0x02, 0x00] {
                return Event::PlayerBroadcast(Player::new(
                    Self::parse_model_name(&buffer),
                    buffer[36],
                    IpAddr::V4(Ipv4Addr::new(buffer[44], buffer[45], buffer[46], buffer[47])),
                ))
            } else if &buffer[32..=34] == &[0x01, 0x03, 0x00] {
                return Event::ApplicationBroadcast
            }
        } else if number_of_bytes == 36 && buffer[10] == 0x10 {
            return Event::PlayerLinkingWaiting(Player::new(
                Self::parse_model_name(&buffer),
                buffer[33],
                metadata.1.ip(),
            ))
        }

        Event::Unknown
    }

    pub fn parse(
        buffer: &[u8],
        metadata: (usize, SocketAddr)
    ) -> Event {
        Self::get_type(&buffer[..metadata.0], metadata)
    }
}

#[cfg(test)]
mod tests {
    use crate::rekordbox::event::{
        Event,
        EventParser
    };
    use crate::rekordbox::{
        player::Player,
        SOFTWARE_IDENTIFICATION,
        APPLICATION_NAME,
    };
    use std::str;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    const MOCKED_PLAYER_NAME: [u8; 20] = [
        0x58,0x44,0x4a,0x2d,0x37,0x30,
        0x30,0x00,0x00,0x00,0x00,0x00,
        0x00,0x00,0x00,0x00,0x00,0x00,
        0x00,0x00
    ];

    fn get_socket_metadata(size: usize) -> (usize, SocketAddr) {
        (size, SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0xa9, 0xfe, 0x1e, 0xc4)), 50000
        ))
    }

    #[test]
    fn it_should_identify_rekordbox_in_network() {
        assert_eq!(EventParser::is_rekordbox_event(&[0x00, 0x01]), false);
        assert_eq!(EventParser::is_rekordbox_event(&SOFTWARE_IDENTIFICATION), true);
    }

    #[test]
    fn it_should_handle_unknown_packages() {
        let mut payload: Vec<u8> = Vec::with_capacity(13);
        payload.extend(&SOFTWARE_IDENTIFICATION.to_vec());
        payload.extend(vec![0x00, 0x00, 0x00]);
        assert_eq!(EventParser::get_type(&payload, get_socket_metadata(13)), Event::Unknown);
    }

    #[test]
    fn it_should_handle_player_broadcasts() {
        let mut payload: Vec<u8> = Vec::with_capacity(54);
        payload.extend(&SOFTWARE_IDENTIFICATION.to_vec());
        payload.extend(vec![0x06, 0x00]);
        payload.extend(&MOCKED_PLAYER_NAME.to_vec());
        payload.extend(vec![0x01, 0x02, 0x00]);
        payload.extend(vec![
            0x36,0x03,0x01,0xc8,0x3d,0xfc,
            0x04,0x1e,0xc4,0xa9,0xfe,0x1e,
            0xc4,0x02,0x00,0x00,0x00,0x01,
            0x00
        ]);
        assert_eq!(payload.as_slice().len(), 54);
        assert_eq!(EventParser::get_type(&payload, get_socket_metadata(54)), Event::PlayerBroadcast(Player::new(
            str::from_utf8(&MOCKED_PLAYER_NAME[..]).unwrap().trim_end_matches('\u{0}').to_string(),
            0x03,
            IpAddr::V4(Ipv4Addr::new(0xa9, 0xfe, 0x1e, 0xc4)),
        )));
    }

    // This method is more or less added for documentation purpose.
    #[test]
    fn it_should_handle_software_broadcasts() {
        let mut payload: Vec<u8> = Vec::with_capacity(54);
        payload.extend(&SOFTWARE_IDENTIFICATION.to_vec());
        payload.extend(vec![0x06, 0x00]);
        payload.extend(&APPLICATION_NAME.to_vec());
        payload.extend(vec![0x01, 0x03, 0x00]);
        payload.extend(vec![
            0x36,0x03,0x01,0xc8,0x3d,0xfc,
            0x04,0x1e,0xc4,0xa9,0xfe,0x1e,
            0xc4,0x02,0x00,0x00,0x00,0x01,
            0x00
        ]);
        assert_eq!(payload.as_slice().len(), 54);
        assert_eq!(EventParser::get_type(&payload, get_socket_metadata(54)), Event::ApplicationBroadcast);
    }

    #[test]
    fn it_should_handle_player_linking_feedback() {
        let mut payload: Vec<u8> = Vec::with_capacity(36);
        payload.extend(&SOFTWARE_IDENTIFICATION.to_vec());
        payload.extend(vec![0x10]);
        payload.extend(&MOCKED_PLAYER_NAME.to_vec());
        payload.extend(vec![0x01, 0x00, 0x01, 0x00, 0x00]);
        assert_eq!(payload.as_slice().len(), 36);
        assert_eq!(EventParser::get_type(&payload, get_socket_metadata(36)), Event::PlayerLinkingWaiting(Player::new(
            str::from_utf8(&MOCKED_PLAYER_NAME[..]).unwrap().trim_end_matches('\u{0}').to_string(),
            0x01,
            IpAddr::V4(Ipv4Addr::new(0xa9, 0xfe, 0x1e, 0xc4)),
        )));
    }
}
