use crate::rekordbox::RekordboxToken;
use crate::utils::MacAddr;

pub struct ApplicationBroadcast<'a> {
    token: &'a RekordboxToken,
    physical_address: &'a MacAddr,
}

impl <'a>ApplicationBroadcast<'a> {
    pub fn new(token: &'a RekordboxToken, physical_address: &'a MacAddr) -> Self {
        Self {
            token: token,
            physical_address: physical_address
        }
    }
}
