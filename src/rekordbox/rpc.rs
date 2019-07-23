use crate::rpc::events::EventHandler as RpcEventHandler;
use super::state::{LockedClientState};
use crate::rpc::packets::RpcReplyMessage;
use std::io::{Error, ErrorKind};

pub struct EventHandler {
    state: LockedClientState,
    // mpsc
}

impl EventHandler {
    pub fn new(client_state: LockedClientState) -> Self {
        EventHandler {
            state: client_state,
        }
    }
}

impl RpcEventHandler for EventHandler {
    fn on_event(&self) -> Result<RpcReplyMessage, std::io::Error> {
        Err(Error::new(ErrorKind::InvalidInput, "failed"))
    }
}
