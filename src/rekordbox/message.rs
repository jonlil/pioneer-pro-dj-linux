use crate::rekordbox::{SOFTWARE_IDENTIFICATION, APPLICATION_NAME};
use crate::utils::network::PioneerNetwork;
use bytes::Bytes;
use super::packets::*;

pub type RekordboxMessageType = Vec<u8>;

macro_rules! rekordbox_message {
    ($T:ident) => {
        pub struct $T;
        impl $T {
            pub fn new() -> $T { $T {} }
        }
        impl Into<RekordboxMessageType> for $T {
            fn into(self) -> RekordboxMessageType {
                self.compose().into_iter().flatten().collect()
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ApplicationBroadcast<'a> {
    network: &'a PioneerNetwork,
}

impl<'a> ApplicationBroadcast<'a> {
    pub fn new(network: &'a PioneerNetwork) -> Self {
        Self { network: network }
    }
}

impl<'a> ComposeRekordboxMessage for ApplicationBroadcast<'a> {
    fn compose(&self) -> Vec<RekordboxMessageType> {
        vec![
            SOFTWARE_IDENTIFICATION.to_vec(),
            vec![0x06, 0x00],
            APPLICATION_NAME.to_vec(),
            vec![0x01,0x03,0x00,0x36,0x11,0x01],
            <[u8; 6]>::from(self.network.mac_address()).to_vec(),
            self.network.ip().octets().to_vec(),
            vec![0x01,0x01,0x00,0x00,0x04,0x08]
        ]
    }
}
impl<'a> Into<RekordboxMessageType> for ApplicationBroadcast<'a> {
    fn into(self) -> RekordboxMessageType {
        self.compose().into_iter().flatten().collect()
    }
}

rekordbox_message!(ApplicationLinkRequest);
impl ComposeRekordboxMessage for ApplicationLinkRequest {
    fn compose(&self) -> Vec<RekordboxMessageType> {
        vec![
            SOFTWARE_IDENTIFICATION.to_vec(),
            vec![0x16],
            APPLICATION_NAME.to_vec(),
            vec![0x01, 0x01, 0x12],
            vec![0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00],
        ]
    }
}

// TODO: refactor to RekordboxReply
rekordbox_message!(InitiateRPCState);
impl From<&InitiateRPCState> for Bytes {
    fn from(_message: &InitiateRPCState) -> Bytes {
        Bytes::from(StatusPacket::new(
            StatusPacketType::RekordboxReply,
            1,
            1,
            StatusContentType::RekordboxReply(RekordboxReply {
                name: "Term DJ".to_string(),
            })
        ))
    }
}

impl ComposeRekordboxMessage for InitiateRPCState {
    fn compose(&self) -> Vec<RekordboxMessageType> {
        vec![Bytes::from(self).into_iter().collect::<Vec<u8>>()]
    }
}

rekordbox_message!(AcknowledgeSuccessfulLinking);
impl ComposeRekordboxMessage for AcknowledgeSuccessfulLinking {
    fn compose(&self) -> Vec<RekordboxMessageType> {
        vec![Bytes::from(self).into_iter().collect::<Vec<u8>>()]
    }
}

impl From<&AcknowledgeSuccessfulLinking> for Bytes {
    fn from(_message: &AcknowledgeSuccessfulLinking) -> Bytes {
        Bytes::from(StatusPacket::new(
            StatusPacketType::LinkReply,
            1,
            1,
            StatusContentType::LinkReply(LinkReply {
                source_player_number: 0x11,
                slot: PlayerSlot::Rekordbox,
                name: Utf16FixedString::new("rekordbox".to_string(), 64),
                date: Utf16FixedString::new("".to_string(), 24),
                unknown5: Utf16FixedString::new("".to_string(), 32),
                track_count: 1051,
                unknown6: 0,
                unknown7: 257,
                playlist_count: 94,
                bytes_total: 0,
                bytes_free: 0,
            }),
        ))
    }
}

trait ComposeRekordboxMessage {
    fn compose(&self) -> Vec<RekordboxMessageType>;
}

#[cfg(test)]
mod test {
    use super::{InitiateRPCState};

    fn initiate_rpc_state_message_package_size() {
        let message: Vec<u8> = InitiateRPCState::new().into();
        assert_eq!(message.len(), 48);
    }
}
