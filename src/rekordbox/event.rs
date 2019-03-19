use crate::rekordbox::player::Player;
use crate::rekordbox::{
    SOFTWARE_IDENTIFICATION,
    APPLICATION_NAME
};
use std::net::{Ipv4Addr, SocketAddr, IpAddr};
use std::str;

#[derive(Debug, PartialEq)]
pub enum Event {
    PlayerBroadcast(Player),
    ApplicationBroadcast,
    Unknown,
    Error,
    InitiateLink,
    Tick,
}

pub struct EventHandler;
impl EventHandler {
    pub fn is_rekordbox_event(data: &[u8]) -> bool {
        if data.len() < SOFTWARE_IDENTIFICATION.len() {
            return false
        }

        SOFTWARE_IDENTIFICATION  == data[..SOFTWARE_IDENTIFICATION.len()]
    }

    fn get_type(buffer: &[u8]) -> Event {
        if !Self::is_rekordbox_event(&buffer) {
            return Event::Unknown
        }

        let number_of_bytes = buffer.len();
        if number_of_bytes == 54 && &buffer[10..=11] == &[0x06, 0x00] {
            if &buffer[32..=34] == &[0x01, 0x02, 0x00] {
                let model_name = str::from_utf8(&buffer[12..=31])
                    .unwrap()
                    .trim_end_matches('\u{0}');

                return Event::PlayerBroadcast(Player::new(
                    model_name.to_string(),
                    buffer[36],
                    IpAddr::V4(Ipv4Addr::new(buffer[44], buffer[45], buffer[46], buffer[47])),
                ))
            } else if &buffer[32..=34] == &[0x01, 0x03, 0x00] {
                return Event::ApplicationBroadcast
            }
        }

        Event::Unknown
    }

    pub fn parse(
        buffer: &[u8],
        metadata: (usize, SocketAddr)
    ) -> Event {
        Self::get_type(&buffer[..metadata.0])
    }
}

#[cfg(test)]
mod tests {
    use crate::rekordbox::event::{
        Event,
        EventHandler
    };
    use crate::rekordbox::{
        player::Player,
        SOFTWARE_IDENTIFICATION,
        APPLICATION_NAME,
    };
    use std::str;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn it_should_identify_rekordbox_in_network() {
        assert_eq!(EventHandler::is_rekordbox_event(&[0x00, 0x01]), false);
        assert_eq!(EventHandler::is_rekordbox_event(&SOFTWARE_IDENTIFICATION), true);
    }

    #[test]
    fn it_should_handle_unknown_packages() {
        let mut payload: Vec<u8> = Vec::with_capacity(13);
        payload.extend(&SOFTWARE_IDENTIFICATION.to_vec());
        payload.extend(vec![0x00, 0x00, 0x00]);
        assert_eq!(EventHandler::get_type(&payload), Event::Unknown);
    }

    #[test]
    fn it_should_handle_player_broadcasts() {
        let mut payload: Vec<u8> = Vec::with_capacity(54);
        payload.extend(&SOFTWARE_IDENTIFICATION.to_vec());
        payload.extend(vec![0x06, 0x00]);
        payload.extend(&APPLICATION_NAME.to_vec());
        payload.extend(vec![0x01, 0x02, 0x00]);
        payload.extend(vec![
            0x36,0x03,0x01,0xc8,0x3d,0xfc,
            0x04,0x1e,0xc4,0xa9,0xfe,0x1e,
            0xc4,0x02,0x00,0x00,0x00,0x01,
            0x00
        ]);
        assert_eq!(payload.as_slice().len(), 54);
        assert_eq!(EventHandler::get_type(&payload), Event::PlayerBroadcast(Player::new(
            str::from_utf8(&APPLICATION_NAME[..]).unwrap().trim_end_matches('\u{0}').to_string(),
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
        assert_eq!(EventHandler::get_type(&payload), Event::ApplicationBroadcast);
    }
}
