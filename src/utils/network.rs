extern crate pnet;
extern crate ipnetwork;

use pnet::datalink::{NetworkInterface};
use ipnetwork::{IpNetwork};
use std::net::IpAddr;

pub fn find_interface(ifaces: Vec<NetworkInterface>, address: IpAddr) -> Option<IpNetwork> {
    ifaces.iter()
        .flat_map(|iface| iface.ips.to_owned())
        .filter(|network: &IpNetwork| network.contains(address))
        .next()
}

#[cfg(test)]
mod tests {
    extern crate pnet;
    extern crate ipnetwork;

    use pnet::datalink::{NetworkInterface, MacAddr};
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


    use crate::utils::network::find_interface;

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
                    IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(0xa9, 0xfe, 0x13, 0xe6), 16).unwrap()),
                ],
            },
        ]
    }

    #[test]
    fn it_finds_local_address_based_on_remote_address() {
        let remote_address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 52));
        let local_network_address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
        let network = find_interface(interfaces(), remote_address);

        assert_eq!(network.is_none(), false);
        assert_eq!(network.unwrap().ip(), local_network_address);
    }

    #[test]
    fn it_find_network_in_a_smaller_cidr() {
        let remote_address = IpAddr::V4(Ipv4Addr::new(192, 168, 12, 230));
        let local_network_address = IpAddr::V4(Ipv4Addr::new(192, 168, 12, 200));
        let network = find_interface(interfaces(), remote_address);

        assert_eq!(network.is_none(), false);
        assert_eq!(network.unwrap().ip(), local_network_address);


        let remote_address = IpAddr::V4(Ipv4Addr::new(192, 168, 12, 24));
        let local_network_address = IpAddr::V4(Ipv4Addr::new(192, 168, 12, 50));
        let network = find_interface(interfaces(), remote_address);

        assert_eq!(network.is_none(), false);
        assert_eq!(network.unwrap().ip(), local_network_address);
    }
}
