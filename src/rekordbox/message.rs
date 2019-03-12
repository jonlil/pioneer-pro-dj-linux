use crate::rekordbox::RekordboxToken;
use crate::utils::MacAddr;

pub struct ApplicationBroadcast<'a> {
    token: &'a RekordboxToken,
    physical_address: &'a MacAddr,
}

impl ApplicationBroadcast {
    pub fn new(token: &RekordboxToken, physical_address: &MacAddr) -> Self {
        Self {
            token: token,
            physical_address: physical_address
        } 
    }
}
