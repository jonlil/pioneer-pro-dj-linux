use std::net::SocketAddr;
use std::str;

// TODO: Make private
pub const SOFTWARE_IDENTIFICATION: [u8; 10] = [
    0x51,0x73,0x70,0x74,0x31,0x57,0x6d,0x4a,0x4f,0x4c
];

// TODO: Make private
pub const APPLICATION_NAME: [u8; 20] = [
    0x4c,0x69,0x6e,0x75,0x78,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00
];

pub enum RekordboxMessage {
    Player(Player),
}

pub struct RekordboxEventHandler;
impl RekordboxEventHandler {
    pub fn is_rekordbox_event(data: &[u8]) -> bool {
        if data.len() < SOFTWARE_IDENTIFICATION.len() {
            return false
        }

        if SOFTWARE_IDENTIFICATION  == &data[..SOFTWARE_IDENTIFICATION.len()] {
            true
        } else {
            false
        }
    }

    fn get_type(buffer: &[u8]) -> RekordboxEvent {
        if !Self::is_rekordbox_event(&buffer) {
            return RekordboxEvent::Unknown
        }

        let number_of_bytes = buffer.len();
        if number_of_bytes == 54 && &buffer[10..=11] == &[0x06, 0x00] {
            if &buffer[32..=34] == &[0x01, 0x02, 0x00] {
                let model_name = str::from_utf8(&buffer[12..=31])
                    .unwrap()
                    .trim_end_matches('\u{0}');

                return RekordboxEvent::PlayerBroadcast {
                    model: model_name.to_string(),
                    number: buffer[36],
                    token: RekordboxToken::new(buffer[44], buffer[45], buffer[46]),
                }
            } else if &buffer[32..=34] == &[0x01, 0x03, 0x00] {
                return RekordboxEvent::ApplicationBroadcast
            }
        }

        RekordboxEvent::Unknown
    }

    pub fn parse(buffer: &[u8], metadata: (usize, SocketAddr)) -> Option<RekordboxMessage> {
        match Self::get_type(buffer) {
            RekordboxEvent::PlayerBroadcast { model, number, token } => {
                Some(RekordboxMessage::Player(
                    Player::new(model, number, token, metadata.1)
                ))
            },
            RekordboxEvent::ApplicationBroadcast => None,
            RekordboxEvent::Error => None,
            RekordboxEvent::Unknown => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Player {
    model: String,
    address: SocketAddr,
    number: u8,
    token: RekordboxToken,
}

impl Player {
    pub fn new(
        model: String,
        number: u8,
        token: RekordboxToken,
        address: SocketAddr
    ) -> Self {
        Self { model: model, number: number, token: token, address: address }
    }
}

#[derive(Debug, PartialEq)]
pub struct RekordboxToken {
    a: u8,
    b: u8,
    c: u8,
}

impl RekordboxToken {
    pub fn new(a: u8, b: u8, c: u8) -> Self {
        Self { a: a, b: b, c: c }
    }
}

#[derive(Debug, PartialEq)]
pub enum RekordboxEvent {
    PlayerBroadcast {
        model: String,
        number: u8,
        token: RekordboxToken,
    },
    ApplicationBroadcast,
    Unknown,
    Error,
}

#[cfg(test)]
mod tests {
    use crate::rekordbox::{
        RekordboxEventHandler,
        RekordboxEvent,
        RekordboxToken,
        SOFTWARE_IDENTIFICATION,
        APPLICATION_NAME,
    };
    use std::str;

    #[test]
    fn it_should_identify_rekordbox_in_network() {
        assert_eq!(RekordboxEventHandler::is_rekordbox_event(&[0x00, 0x01]), false);
        assert_eq!(RekordboxEventHandler::is_rekordbox_event(&SOFTWARE_IDENTIFICATION), true);
    }

    #[test]
    fn it_should_handle_unknown_packages() {
        let mut payload: Vec<u8> = Vec::with_capacity(13);
        payload.extend(&SOFTWARE_IDENTIFICATION.to_vec());
        payload.extend(vec![0x00, 0x00, 0x00]);
        assert_eq!(RekordboxEventHandler::get_type(&payload), RekordboxEvent::Unknown);
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
        assert_eq!(RekordboxEventHandler::get_type(&payload), RekordboxEvent::PlayerBroadcast {
            model: str::from_utf8(&APPLICATION_NAME[..]).unwrap().trim_end_matches('\u{0}').to_string(),
            number: 0x03,
            token: RekordboxToken::new(0xa9, 0xfe, 0x1e),
        });
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
        assert_eq!(RekordboxEventHandler::get_type(&payload), RekordboxEvent::ApplicationBroadcast);
    }
}
