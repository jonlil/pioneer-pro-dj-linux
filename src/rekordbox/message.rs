extern crate pnet;

use pnet::datalink::MacAddr;
use std::net::Ipv4Addr;

pub struct ApplicationBroadcast<'a> {
    address: Ipv4Addr,
    physical_address: &'a MacAddr,
}

impl <'a>ApplicationBroadcast<'a> {
    pub fn new(address: Ipv4Addr, physical_address: &'a MacAddr) -> Self {
        Self {
            address: address,
            physical_address: physical_address
        }
    }
}
