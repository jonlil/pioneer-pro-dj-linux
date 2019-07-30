extern crate pnet;
extern crate ipnetwork;

use pnet::datalink::{NetworkInterface, MacAddr, interfaces};
use ipnetwork::{IpNetwork};
use std::net::{IpAddr, Ipv4Addr};

#[derive(Debug, PartialEq)]
pub struct PioneerNetwork {
    network: IpNetwork,
    mac: MacAddr,
}

impl PioneerNetwork {
    pub fn new(network: IpNetwork, mac: MacAddr) -> Self {
        Self { network: network, mac: mac }
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        self.network.contains(ip)
    }

    pub fn ip(&self) -> IpAddr {
        self.network.ip()
    }

    pub fn mask(&self) -> IpAddr {
        self.network.mask()
    }

    pub fn mac_address(&self) -> MacAddr {
        self.mac
    }

    pub fn broadcast(&self) -> IpAddr {
        self.network.broadcast()
    }
}

pub fn find_interface(address: Ipv4Addr) -> Option<PioneerNetwork> {
    match_interface(interfaces(), address)
}

fn match_interface(ifaces: Vec<NetworkInterface>, address: Ipv4Addr) -> Option<PioneerNetwork> {
    let address = IpAddr::V4(address);
    ifaces.iter()
        .flat_map(|iface| iface.ips.iter().map(move |ip| PioneerNetwork::new(
            *ip,
            iface.mac.unwrap(),
        )))
        .find(|network: &PioneerNetwork| network.contains(address))
}

#[cfg(test)]
mod tests {
    use std::net::{
        IpAddr,
        Ipv4Addr,
        Ipv6Addr,
    };
    use ipnetwork::{
        IpNetwork,
        Ipv4Network,
        Ipv6Network,
    };
    use super::*;

    fn interfaces() -> Vec<NetworkInterface> {
        vec![
            NetworkInterface {
                name: "eno1".to_string(),
                mac: Some(MacAddr::new(0x00, 0x45, 0xcb, 0x9a, 0xa5, 0x0b)),
                index: 0,
                flags: 69699,
                ips: vec![
                    IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(0xc0, 0xa8, 0x0a, 0x32), 24).unwrap()),
                    IpNetwork::V6(Ipv6Network::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 64).unwrap()),
                ],
            },

            NetworkInterface {
                name: "eno1:1".to_string(),
                mac: Some(MacAddr::new(0x00, 0x45, 0xcb, 0x9a, 0xa5, 0x0c)),
                index: 0,
                flags: 69699,
                ips: vec![
                    IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(0xc0, 0xa8, 0x0b, 0x32), 24).unwrap()),
                ],
            },

            NetworkInterface {
                name: "eno1:2".to_string(),
                mac: Some(MacAddr::new(0x00, 0x45, 0xcb, 0x9a, 0xa5, 0x0c)),
                index: 0,
                flags: 69699,
                ips: vec![
                    IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(0xc0, 0xa8, 0x0c, 0x32), 25).unwrap()),
                ],
            },

            NetworkInterface {
                name: "eno1:3".to_string(),
                mac: Some(MacAddr::new(0x00, 0x45, 0xcb, 0x9a, 0xa5, 0x0c)),
                index: 0,
                flags: 69699,
                ips: vec![
                    IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(0xc0, 0xa8, 0x0c, 0xc8), 25).unwrap()),
                ],
            },

            NetworkInterface {
                name: "en4".to_string(),
                mac: Some(MacAddr::new(0x00, 0x45, 0xcb, 0x9a, 0xa5, 0x10)),
                index: 0,
                flags: 69699,
                ips: vec![
                    IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(169, 254, 21, 48), 16).unwrap()),
                ],
            },
        ]
    }

    #[test]
    fn it_finds_local_address_based_on_remote_address() {
        let remote_address = Ipv4Addr::new(192, 168, 10, 52);
        let local_network_address = IpNetwork::V4(Ipv4Network::new(
                Ipv4Addr::new(192, 168, 10, 50), 24).unwrap());

        let network = match_interface(interfaces(), remote_address);

        assert_eq!(network.is_none(), false);
        assert_eq!(network, Some(PioneerNetwork {
            network: local_network_address,
            mac: MacAddr::new(0x00, 0x45, 0xcb, 0x9a, 0xa5, 0x0b)
        }));
    }

    #[test]
    fn it_find_network_in_a_smaller_cidr() {
        let remote_address = Ipv4Addr::new(192, 168, 12, 230);
        let local_network_address = IpAddr::V4(Ipv4Addr::new(192, 168, 12, 200));
        let network = match_interface(interfaces(), remote_address);

        assert_eq!(network.is_none(), false);
        assert_eq!(network.unwrap().ip(), local_network_address);


        let remote_address = Ipv4Addr::new(192, 168, 12, 24);
        let local_network_address = IpAddr::V4(Ipv4Addr::new(192, 168, 12, 50));
        let network = match_interface(interfaces(), remote_address);

        assert_eq!(network.is_none(), false);
        assert_eq!(network.unwrap().ip(), local_network_address);
    }
}
