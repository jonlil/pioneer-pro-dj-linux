use std::sync::{RwLock, Arc};
use crate::rekordbox::player::{PlayerCollection};
use crate::utils::network::{PioneerNetwork};


pub type LockedClientState = Arc<RwLock<ClientState>>;


// ClientState
//
// Provides thread safe access to stateful properties for Rekordbox::Client
#[derive(Debug)]
pub struct ClientState {
    // If the performer has pressed the button to start the linking phase.
    linking: bool,

    // If we have discovered rekordbox compatibile network devices that we have
    // have in a recent time responded to.
    discovery: bool,

    // True when the linking & discovery phases have completed
    linked: bool,

    // Network to send Rekordbox messages to
    address: Option<PioneerNetwork>,

    // TODO: implement mutable accessor method #mut_players
    pub players: PlayerCollection,
}

// TODO: Implement macro for llvm generation of getter and setters
impl ClientState {
    pub fn new() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(ClientState {
            address: None,
            discovery: false,
            linked: false,
            linking: false,
            players: PlayerCollection::new(),
        } ))
    }

    pub fn players(&self) -> &PlayerCollection {
        &self.players
    }

    pub fn set_linking(&mut self, value: bool) {
        self.linking = value;
    }

    pub fn set_discovery(&mut self, value: bool) {
        self.discovery = value;
    }

    pub fn is_discovery(&self) -> bool {
        self.discovery
    }

    pub fn is_linking(&self) -> bool {
        self.linking
    }

    pub fn set_address(&mut self, address: PioneerNetwork) {
        self.address = Some(address);
    }

    pub fn address(&self) -> &Option<PioneerNetwork> {
        &self.address
    }
}
