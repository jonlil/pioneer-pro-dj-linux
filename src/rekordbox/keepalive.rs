use std::convert::TryFrom;
use std::net::{UdpSocket, SocketAddr, IpAddr, Ipv4Addr};
use std::thread;
use std::time::Duration;
use pnet::datalink::{MacAddr};
use bytes::{Bytes, BytesMut, BufMut};
use super::packets::{UdpMagic, ModelName};
use nom::{
IResult,
number::complete::{be_u32, be_u16, be_u8},
sequence::tuple,
bytes::complete::take,
error::ErrorKind::Switch};

pub struct KeepAliveServerOptions {
    socket_addr: SocketAddr,
}

impl Default for KeepAliveServerOptions {
    fn default() -> KeepAliveServerOptions {
        KeepAliveServerOptions {
            socket_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 50000),
        }
    }
}

pub struct KeepAliveServer {
    options: KeepAliveServerOptions,
    socket: UdpSocket,
}

impl KeepAliveServer {
    pub fn new(socket: UdpSocket, options: KeepAliveServerOptions) -> KeepAliveServer {
        KeepAliveServer {
            options,
            socket,
        }
    }

    pub fn bind(options: KeepAliveServerOptions) -> Result<KeepAliveServer, String> {
        let socket = KeepAliveServer::get_socket(options.socket_addr)?;
        let server = KeepAliveServer {
            options,
            socket,
        };

        server.set_broadcast()?;

        Ok(server)
    }

    pub fn run(&self) {
        loop {
            self.recv_from();
            thread::sleep(Duration::from_millis(250));
        }
    }

    fn on_event(&self, event: (KeepAlivePacket, SocketAddr)) {
        eprintln!("{:#?}", event.0);
    }

    fn recv_from(&self) {
        let mut buffer = [0u8; 1024];
        match self.socket.recv_from(&mut buffer) {
            Ok((nob, peer)) => {
                match KeepAlivePacket::try_from(Bytes::from(&buffer[..nob])) {
                    Ok(packet) => self.on_event((packet, peer)),
                    Err(_err) => eprintln!("Failed decoding KeepAlivePacket: {:?}", &buffer[..nob]),
                }
            },
            Err(_err) => {},
        };
    }

    fn set_broadcast(&self) -> Result<(), String> {
        self.socket.set_broadcast(true).map_err(|err| format!("{}", err))
    }

    fn get_socket(socket_addr: SocketAddr) -> Result<UdpSocket, String> {
        UdpSocket::bind(socket_addr).map_err(|err| format!("{}", err))
    }
}

#[derive(Debug, PartialEq)]
pub struct KeepAlivePacket {
    kind: KeepAlivePacketType,
    subkind: KeepAlivePacketSubType,
    model: ModelName,
    unknown1: u8,
    device_type: DeviceType,
    content: KeepAliveContentType,
}

impl Decoder<KeepAlivePacket> for KeepAlivePacket {
    fn decode(input: &[u8]) -> IResult<&[u8], KeepAlivePacket> {

        let (input, _magic) = UdpMagic::decode(input)?;
        let (input, kind) = KeepAlivePacketType::decode(input)?;
        let (input, _padding) = take(1u8)(input)?;
        let (input, model) = ModelName::decode(input)?;
        let (input, unknown1) = be_u8(input)?;
        let (input, device_type) = DeviceType::decode(input)?;
        let (input, _padding) = take(1u8)(input)?;
        let (input, subkind) = KeepAlivePacketSubType::decode(input)?;
        let (input, content) = kind.decode_content(input)?;

        Ok((input, KeepAlivePacket {
            kind,
            subkind,
            model,
            unknown1,
            device_type,
            content,
        }))
    }
}

impl From<KeepAlivePacket> for Bytes {
    fn from(packet: KeepAlivePacket) -> Bytes {
        let mut buf = BytesMut::with_capacity(128);

        buf.extend(Bytes::from(UdpMagic {}));
        buf.extend(Bytes::from(packet.kind));
        buf.extend(&[0x00]);
        buf.extend(Bytes::from(packet.model));
        buf.put(packet.unknown1);
        buf.extend(Bytes::from(packet.device_type));
        buf.put(0u8);
        buf.extend(Bytes::from(packet.subkind));
        buf.extend(Bytes::from(packet.content));

        buf.freeze()
    }
}

impl TryFrom<Bytes> for KeepAlivePacket {
    type Error = &'static str;

    fn try_from(message: Bytes) -> Result<Self, Self::Error> {
        match KeepAlivePacket::decode(&message) {
            Ok((_input, message)) => Ok(message),
            Err(_err) => Err("Failed decoding KeepAlivePacket."),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum DeviceType {
    Djm,
    Cdj,
    Rekordbox,
}

impl Decoder<DeviceType> for DeviceType {
    fn decode(input: &[u8]) -> IResult<&[u8], DeviceType> {
        let (input, device_type) = be_u8(input)?;

        match device_type {
            1 => Ok((input, DeviceType::Djm)),
            2 => Ok((input, DeviceType::Cdj)),
            3 => Ok((input, DeviceType::Rekordbox)),
            _ => Err(nom::Err::Error((input, Switch))),
        }
    }
}

impl From<DeviceType> for Bytes {
    fn from(device_type: DeviceType) -> Bytes {
        let mut buf = BytesMut::new();

        buf.put(match device_type {
            DeviceType::Djm => 1u8,
            DeviceType::Cdj => 2u8,
            DeviceType::Rekordbox => 3u8,
        });

        buf.freeze()
    }
}

#[derive(Debug, PartialEq)]
pub enum KeepAlivePacketType {
    Hello,
    Number,
    Mac,
    Ip,
    Status,
    Change,
}

impl Decoder<KeepAlivePacketType> for KeepAlivePacketType {
    fn decode(input: &[u8]) -> IResult<&[u8], KeepAlivePacketType> {
        let (input, kind) = take(1u8)(input)?;

        match kind[0] {
            0x0a => Ok((input, KeepAlivePacketType::Hello)),
            0x04 => Ok((input, KeepAlivePacketType::Number)),
            0x00 => Ok((input, KeepAlivePacketType::Mac)),
            0x02 => Ok((input, KeepAlivePacketType::Ip)),
            0x06 => Ok((input, KeepAlivePacketType::Status)),
            0x08 => Ok((input, KeepAlivePacketType::Change)),
            _    => Err(nom::Err::Error((input, Switch))),
        }
    }
}

impl KeepAlivePacketType {
    fn decode_content<'a>(&self, input: &'a [u8]) -> IResult<&'a [u8], KeepAliveContentType> {
        let (input, decoded_value) = match self {
            KeepAlivePacketType::Status => Status::decode,
            KeepAlivePacketType::Hello  => Hello::decode,
            KeepAlivePacketType::Change => Change::decode,
            KeepAlivePacketType::Number => Number::decode,
            KeepAlivePacketType::Mac    => Mac::decode,
            KeepAlivePacketType::Ip     => Ip::decode,
        }(input)?;

        Ok((input, decoded_value))
    }
}

impl From<KeepAlivePacketType> for Bytes {
    fn from(packet: KeepAlivePacketType) -> Bytes {
        let mut buf = BytesMut::new();

        let kind: u8 = match packet {
            KeepAlivePacketType::Hello => 0x0a,
            KeepAlivePacketType::Number => 0x04,
            KeepAlivePacketType::Mac => 0x00,
            KeepAlivePacketType::Ip => 0x02,
            KeepAlivePacketType::Status => 0x06,
            KeepAlivePacketType::Change => 0x08,
        };
        buf.put(kind);

        buf.freeze()
    }
}

#[derive(Debug, PartialEq)]
pub struct Mac {
    iteration: u8,
    unknown2: u8,
    mac_addr: MacAddr,
}

impl Decoder<KeepAliveContentType> for Mac {
    fn decode(input: &[u8]) -> IResult<&[u8], KeepAliveContentType> {
        let (input, iteration) = be_u8(input)?;
        let (input, unknown2) = be_u8(input)?;
        let (input, mac_addr) = MacAddr::decode(input)?;

        Ok((
            input,
            KeepAliveContentType::Mac(Mac {
                iteration,
                unknown2,
                mac_addr,
            })
        ))
    }
}

impl From<Mac> for Bytes {
    fn from(value: Mac) -> Bytes {
        let mut buf = BytesMut::with_capacity(8);

        buf.put(value.iteration);
        buf.put(value.unknown2);
        buf.put(mac_addr_to_bytes(value.mac_addr));

        buf.freeze()
    }
}

#[derive(Debug, PartialEq)]
pub enum PlayerNumberAssignment {
    Auto,
    Manual,
}

impl Decoder<PlayerNumberAssignment> for PlayerNumberAssignment {
    fn decode(input: &[u8]) -> IResult<&[u8], PlayerNumberAssignment> {
        let (input, assignment) = be_u8(input)?;
        match assignment {
            1 => Ok((input, PlayerNumberAssignment::Auto)),
            2 => Ok((input, PlayerNumberAssignment::Manual)),
            _ => Err(nom::Err::Error((input, Switch))),
        }
    }
}

impl From<PlayerNumberAssignment> for Bytes {
    fn from(value: PlayerNumberAssignment) -> Bytes {
        let mut buf = BytesMut::with_capacity(1);
        buf.put(match value {
            Auto => 1u8,
            Manual => 2u8,
        });

        buf.freeze()
    }
}

#[derive(Debug, PartialEq)]
pub struct Ip {
    ip_addr: Ipv4Addr,
    mac_addr: MacAddr,
    player_number: u8,
    iteration: u8,
    unknown2: u8,
    player_number_assignment: PlayerNumberAssignment,
}

impl Decoder<KeepAliveContentType> for Ip {
    fn decode(input: &[u8]) -> IResult<&[u8], KeepAliveContentType> {
        let (input, ip_addr) = Ipv4Addr::decode(input)?;
        let (input, mac_addr) = MacAddr::decode(input)?;
        let (input, player_number) = be_u8(input)?;
        let (input, iteration) = be_u8(input)?;
        let (input, unknown2) = be_u8(input)?;
        let (input, player_number_assignment) = PlayerNumberAssignment::decode(input)?;

        Ok((
            input,
            KeepAliveContentType::Ip(Ip {
                ip_addr,
                mac_addr,
                player_number,
                iteration,
                unknown2,
                player_number_assignment,
            })
        ))
    }
}

impl From<Ip> for Bytes {
    fn from(ip: Ip) -> Bytes {
        let mut buf = BytesMut::with_capacity(14);

        buf.put(ip_addr_to_bytes(ip.ip_addr));
        buf.put(mac_addr_to_bytes(ip.mac_addr));
        buf.put(ip.player_number);
        buf.put(ip.iteration);
        buf.put(ip.unknown2);
        buf.put(Bytes::from(ip.player_number_assignment));

        buf.freeze()
    }
}

#[derive(Debug, PartialEq)]
pub struct Change {
    old_player_number: u8,
    ip_addr: Ipv4Addr,
}

impl Decoder<KeepAliveContentType> for Change {
    fn decode(input: &[u8]) -> IResult<&[u8], KeepAliveContentType> {
        let (input, old_player_number) = be_u8(input)?;
        let (input, ip_addr) = Ipv4Addr::decode(input)?;

        Ok((input, KeepAliveContentType::Change(Change {
            old_player_number,
            ip_addr,
        })))
    }
}

impl From<Change> for Bytes {
    fn from(value: Change) -> Bytes {
        let mut buf = BytesMut::with_capacity(5);

        buf.put(value.old_player_number);
        buf.put(ip_addr_to_bytes(value.ip_addr));

        buf.freeze()
    }
}

#[derive(Debug, PartialEq)]
pub struct Number {
    proposed_player_number: u8,
    iteration: u8,
}

impl Decoder<KeepAliveContentType> for Number {
    fn decode(input: &[u8]) -> IResult<&[u8], KeepAliveContentType> {
        let (input, proposed_player_number) = be_u8(input)?;
        let (input, iteration) = be_u8(input)?;

        Ok((
            input,
            KeepAliveContentType::Number(Number {
                proposed_player_number,
                iteration,
            })
        ))
    }
}

impl From<Number> for Bytes {
    fn from(value: Number) -> Bytes {
        Bytes::from(vec![
            value.proposed_player_number,
            value.iteration,
        ])
    }
}

#[derive(Debug, PartialEq)]
pub struct Hello {
    unknown2: u8,
}

impl Decoder<KeepAliveContentType> for Hello {
    fn decode(input: &[u8]) -> IResult<&[u8], KeepAliveContentType> {
        let (input, unknown2) = be_u8(input)?;

        Ok((input, KeepAliveContentType::Hello(Hello {
            unknown2,
        })))
    }
}

impl From<Hello> for Bytes {
    fn from(value: Hello) -> Bytes {
        Bytes::from(vec![value.unknown2])
    }
}

#[derive(Debug, PartialEq)]
pub struct Status {
    player_number: u8,
    unknown2: u8,
    mac_addr: MacAddr,
    ip_addr: Ipv4Addr,
    device_count: u8,
    unknown3: u16,
}

impl Decoder<KeepAliveContentType> for Status {
    fn decode(input: &[u8]) -> IResult<&[u8], KeepAliveContentType> {
        let (input, player_number) = be_u8(input)?;
        let (input, unknown2) = be_u8(input)?;
        let (input, mac_addr) = MacAddr::decode(input)?;
        let (input, ip_addr) = Ipv4Addr::decode(input)?;
        let (input, device_count) = be_u8(input)?;
        let (input, _padding) = take(2u8)(input)?;
        let (input, unknown3) = be_u16(input)?;

        Ok((input, KeepAliveContentType::Status(Status {
            player_number,
            unknown2,
            mac_addr,
            ip_addr,
            device_count,
            unknown3,
        })))
    }
}

impl From<Status> for Bytes {
    fn from(value: Status) -> Bytes {
        let mut buf = BytesMut::with_capacity(15);
        buf.put(value.player_number);
        buf.put(value.unknown2);
        buf.put(mac_addr_to_bytes(value.mac_addr));
        buf.put(ip_addr_to_bytes(value.ip_addr));
        buf.put(value.device_count);
        buf.extend(Bytes::from(value.unknown3.to_be_bytes().to_vec()));

        buf.freeze()
    }
}

trait Decoder<T> {
    fn decode(input: &[u8]) -> IResult<&[u8], T>;
}

impl Decoder<MacAddr> for MacAddr {
    fn decode(input: &[u8]) -> IResult<&[u8], MacAddr> {
        let (input, (a, b, c, d, e, f)) = tuple((
            be_u8,
            be_u8,
            be_u8,
            be_u8,
            be_u8,
            be_u8,
        ))(input)?;

        Ok((input, MacAddr::new(a, b, c, d, e, f)))
    }
}

fn mac_addr_to_bytes(mac_addr: MacAddr) -> Bytes {
    Bytes::from(<[u8; 6]>::from(mac_addr).to_vec())
}

fn ip_addr_to_bytes(ip_addr: Ipv4Addr) -> Bytes {
    Bytes::from(ip_addr.octets().to_vec())
}

impl Decoder<Ipv4Addr> for Ipv4Addr {
    fn decode(input: &[u8]) -> IResult<&[u8], Ipv4Addr> {
        let (input, addr) = be_u32(input)?;

        Ok((input, Ipv4Addr::from(addr)))
    }
}

#[derive(Debug, PartialEq)]
pub enum KeepAlivePacketSubType {
    Hello,
    Number,
    Mac,
    Ip,
    Status,
    Change,
    StatusMixer,
    Unknown(u8),
}

impl Decoder<KeepAlivePacketSubType> for KeepAlivePacketSubType {
    fn decode(input: &[u8]) -> IResult<&[u8], KeepAlivePacketSubType> {
        let (input, kind) = take(1u8)(input)?;

        Ok((input, match kind[0] {
            0x25 => KeepAlivePacketSubType::Hello,
            0x26 => KeepAlivePacketSubType::Number,
            0x2c => KeepAlivePacketSubType::Mac,
            0x32 => KeepAlivePacketSubType::Ip,
            0x36 => KeepAlivePacketSubType::Status,
            0x29 => KeepAlivePacketSubType::Change,
            0x00 => KeepAlivePacketSubType::StatusMixer,
            _    => KeepAlivePacketSubType::Unknown(kind[0]),
        }))
    }
}

impl From<KeepAlivePacketSubType> for Bytes {
    fn from(subkind: KeepAlivePacketSubType) -> Bytes {
        Bytes::from(vec![match subkind {
            KeepAlivePacketSubType::Hello => 0x25,
            KeepAlivePacketSubType::Number => 0x26,
            KeepAlivePacketSubType::Mac => 0x2c,
            KeepAlivePacketSubType::Ip => 0x32,
            KeepAlivePacketSubType::Status => 0x36,
            KeepAlivePacketSubType::Change => 0x29,
            KeepAlivePacketSubType::StatusMixer => 0x00,
            KeepAlivePacketSubType::Unknown(value) => value,
        }])
    }
}

#[derive(Debug, PartialEq)]
pub enum KeepAliveContentType {
    Hello(Hello),
    Number(Number),
    Mac(Mac),
    Ip(Ip),
    Status(Status),
    Change(Change),
}

impl From<KeepAliveContentType> for Bytes {
    fn from(content: KeepAliveContentType) -> Bytes {
        match content {
            KeepAliveContentType::Hello(value) => Bytes::from(value),
            KeepAliveContentType::Number(value) => Bytes::from(value),
            KeepAliveContentType::Mac(value) => Bytes::from(value),
            KeepAliveContentType::Ip(value) => Bytes::from(value),
            KeepAliveContentType::Status(value) => Bytes::from(value),
            KeepAliveContentType::Change(value) => Bytes::from(value),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::message;
    use super::super::super::utils::network::PioneerNetwork;
    use ipnetwork::{IpNetwork, Ipv4Network};
    use pnet::datalink::{MacAddr};
    use pretty_assertions::assert_eq;

    fn get_pioneer_network() -> PioneerNetwork {
        let addr = Ipv4Addr::new(192, 168, 10, 50);
        let cidr = 24;
        let network = IpNetwork::V4(Ipv4Network::new(addr, cidr).unwrap());

        PioneerNetwork::new(
            network,
            MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        )
    }

    #[test]
    fn test_mac_iteration() {
        let network = get_pioneer_network();
        let discovery_sequence_1 = Bytes::from(message::DiscoveryInitial::new(&network, 1));

        assert_eq!(discovery_sequence_1, Bytes::from(
            KeepAlivePacket {
                kind: KeepAlivePacketType::Mac,
                subkind: KeepAlivePacketSubType::Mac,
                model: ModelName::new("Linux".to_string()),
                unknown1: 1,
                device_type: DeviceType::Rekordbox,
                content: KeepAliveContentType::Mac(
                    Mac {
                        iteration: 1,
                        unknown2: 4,
                        mac_addr: MacAddr::from([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
                    },
                ),
            }
        ));
    }
}
