use crate::rekordbox::{
    SOFTWARE_IDENTIFICATION,
    APPLICATION_NAME,
};
use std::net::IpAddr;
use crate::utils::network::PioneerNetwork;

pub type RekordboxMessageType = Vec<u8>;

#[derive(Debug, PartialEq)]
pub struct ApplicationBroadcast<'a> {
    network: &'a PioneerNetwork,
}

impl <'a>ApplicationBroadcast<'a> {
    pub fn new(network: &'a PioneerNetwork) -> Self {
        Self { network: network }
    }
}

impl <'a>IntoRekordboxMessage for ApplicationBroadcast<'a> {
    fn compose(&self) -> Vec<RekordboxMessageType> {
        vec![
            SOFTWARE_IDENTIFICATION.to_vec(),
            vec![0x06, 0x00],
            APPLICATION_NAME.to_vec(),
            vec![0x01,0x03,0x00,0x36,0x11,0x01],
            <[u8; 6]>::from(self.network.mac_address()).to_vec(),
            match self.network.ip() {
                IpAddr::V4(ip) => ip.octets().to_vec(),
                IpAddr::V6(_ip) => panic!("IPv6 is not supported by TermDJ protocol"),
            },
            vec![0x01,0x01,0x00,0x00,0x04,0x08]
        ]
    }
}

impl <'a>Into<RekordboxMessageType> for ApplicationBroadcast<'a> {
    fn into(self) -> RekordboxMessageType {
        self.compose().into_iter().flatten().collect()
    }
}

pub struct DiscoveryInitial<'a> {
    network: &'a PioneerNetwork,
    sequence: u8,
}

impl <'a>DiscoveryInitial<'a> {
    pub fn new(network: &'a PioneerNetwork, sequence: u8) -> Self {
        Self { network: network, sequence: sequence }
    }
}

impl <'a>IntoRekordboxMessage for DiscoveryInitial<'a> {
    fn compose(&self) -> Vec<RekordboxMessageType> {
        vec![
            SOFTWARE_IDENTIFICATION.to_vec(),
            vec![0x00,0x00],
            APPLICATION_NAME.to_vec(),
            vec![0x01, 0x03, 0x00, 0x2c, self.sequence, 0x04],
            <[u8; 6]>::from(self.network.mac_address()).to_vec(),
        ]
    }
}

impl <'a>Into<RekordboxMessageType> for DiscoveryInitial<'a> {
    fn into(self) -> RekordboxMessageType {
        self.compose().into_iter().flatten().collect()
    }
}

pub struct DiscoverySequence<'a> {
    network: &'a PioneerNetwork,
    sequence: u8,
    index: i32,
}

impl <'a>DiscoverySequence<'a> {
    pub fn new(network: &'a PioneerNetwork, sequence: u8, index: i32) -> Self {
        Self { network: network, sequence: sequence, index: index }
    }

    fn map_sequence_byte(&self) -> u8 {
        if self.index == 1 {
            0x11
        } else if self.index == 2 {
            0x12
        } else if self.index == 3 {
            0x29
        } else if self.index == 4 {
            0x2a
        } else if self.index == 5 {
            0x2b
        } else {
            0x2c
        }
    }
}

// TODO: Possible performance fix here is to reuse this struct instead of composing
// new ones for each of these 36 messages.
impl <'a>IntoRekordboxMessage for DiscoverySequence<'a> {
    fn compose(&self) -> Vec<RekordboxMessageType> {
        vec![
            SOFTWARE_IDENTIFICATION.to_vec(),
            vec![0x02, 0x00],
            APPLICATION_NAME.to_vec(),
            vec![0x01, 0x03, 0x00, 0x32],
            match self.network.ip() {
                IpAddr::V4(ip) => ip.octets().to_vec(),
                IpAddr::V6(_ip) => panic!("IPv6 is not supported by TermDJ protocol"),
            },
            <[u8; 6]>::from(self.network.mac_address()).to_vec(),
            vec![self.map_sequence_byte(), self.sequence],
            vec![0x04, 0x01]
        ]
    }
}

impl <'a>Into<RekordboxMessageType> for DiscoverySequence<'a> {
    fn into(self) -> RekordboxMessageType {
        self.compose().into_iter().flatten().collect()
    }
}

pub struct ApplicationLinkRequest;
impl ApplicationLinkRequest {
    pub fn new() -> Self { Self {} }
}

impl IntoRekordboxMessage for ApplicationLinkRequest {
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

impl Into<RekordboxMessageType> for ApplicationLinkRequest {
    fn into(self) -> RekordboxMessageType {
        self.compose().into_iter().flatten().collect()
    }
}

pub struct InitiateRPCState;
impl InitiateRPCState {
    pub fn new() -> Self { Self {} }
}

impl IntoRekordboxMessage for InitiateRPCState {
    fn compose(&self) -> Vec<RekordboxMessageType> {
        let mut payload = vec![];
        payload.extend(&SOFTWARE_IDENTIFICATION.to_vec());
        payload.extend(vec![0x11]);
        payload.extend(&APPLICATION_NAME.to_vec());
        payload.extend(vec![0x01, 0x01, 0x11]);

        // Extract hostname
        payload.extend(vec![
            0x01,0x04,0x11,0x01,0x00,0x00,
            0x00,0x4a,0x00,0x6f,0x00,0x6e,
            0x00,0x61,0x00,0x73,0x00,0x73,
            0x00,0x2d,0x00,0x4d,0x00,0x42,
            0x00,0x50,0x00,0x2d,0x00,0x32
        ]);
        payload.extend(vec![0x00; 232]);

        vec![payload]
    }
}

impl Into<RekordboxMessageType> for InitiateRPCState {
    fn into(self) -> RekordboxMessageType {
        self.compose().into_iter().flatten().collect()
    }
}

trait IntoRekordboxMessage {
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
