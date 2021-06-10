use super::packets::{ModelName, UdpMagic};
use super::EventHandler;
use bytes::{BufMut, Bytes, BytesMut};
use nom::{
    bytes::complete::take,
    error::ErrorKind::Switch,
    number::complete::{be_u16, be_u32, be_u8},
    sequence::tuple,
    IResult,
};
use pnet::datalink::MacAddr;
use std::convert::TryFrom;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::thread;
use std::time::Duration;
use crate::utils::parse_error;

pub struct KeepAliveMacPackage;
impl KeepAliveMacPackage {
    pub fn new(iteration: u8, mac_addr: MacAddr) -> KeepAlivePacket {
        KeepAlivePacket {
            kind: KeepAlivePacketType::Mac,
            subkind: KeepAlivePacketSubType::Mac,
            model: ModelName::new("rekordbox".to_string()),
            unknown1: 1,
            device_type: DeviceType::Rekordbox,
            content: KeepAliveContentType::Mac(Mac {
                iteration,
                unknown2: 4,
                mac_addr,
            }),
        }
    }
}

pub struct KeepAliveIpPackage;
impl KeepAliveIpPackage {
    pub fn new(iteration: u8, index: u8, ip_addr: Ipv4Addr, mac_addr: MacAddr) -> KeepAlivePacket {
        KeepAlivePacket {
            kind: KeepAlivePacketType::Ip,
            subkind: KeepAlivePacketSubType::Ip,
            model: ModelName::new("rekordbox".to_string()),
            unknown1: 1,
            device_type: DeviceType::Rekordbox,
            content: KeepAliveContentType::Ip(Ip {
                ip_addr,
                mac_addr,
                iteration,
                index,
                player_number_assignment: PlayerNumberAssignment::Auto,
            }),
        }
    }
}

pub struct KeepAliveStatusPackage;
impl KeepAliveStatusPackage {
    pub fn new(
        ip_addr: Ipv4Addr,
        mac_addr: MacAddr,
        unknown3: u16,
        unknown4: u8,
    ) -> KeepAlivePacket {
        KeepAlivePacket {
            kind: KeepAlivePacketType::Status,
            subkind: KeepAlivePacketSubType::Status,
            model: ModelName::new("rekordbox".to_string()),
            unknown1: 1,
            device_type: DeviceType::Rekordbox,
            content: KeepAliveContentType::Status(Status {
                player_number: 17,
                unknown2: 1,
                ip_addr,
                mac_addr,
                device_count: 1,
                unknown3,
                unknown4,
            }),
        }
    }
}

pub type Event = (KeepAlivePacket, SocketAddr);

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
        KeepAliveServer { options, socket }
    }

    pub fn bind(options: KeepAliveServerOptions) -> Result<KeepAliveServer, String> {
        let socket = get_socket(options.socket_addr)?;
        let server = KeepAliveServer { options, socket };

        server.set_broadcast()?;

        Ok(server)
    }

    pub fn run<T: EventHandler<Event>>(&self, event_handler: T) {
        loop {
            self.recv_from(&event_handler);
            thread::sleep(Duration::from_millis(250));
        }
    }

    fn recv_from<T: EventHandler<Event>>(&self, event_handler: &T) {
        let mut buffer = [0u8; 1024];
        match self.socket.recv_from(&mut buffer) {
            Ok((number_of_bytes, peer)) => {
                match KeepAlivePacket::try_from(Bytes::from(buffer[..number_of_bytes].to_vec())) {
                    Ok(packet) => event_handler.on_event((packet, peer)),
                    Err(_err) => eprintln!(
                        "Failed decoding KeepAlivePacket: {:?}",
                        &buffer[..number_of_bytes]
                    ),
                }
            }
            Err(_err) => {}
        };
    }

    fn set_broadcast(&self) -> Result<(), String> {
        self.socket
            .set_broadcast(true)
            .map_err(|err| format!("{}", err))
    }
}

fn get_socket(socket_addr: SocketAddr) -> Result<UdpSocket, String> {
    UdpSocket::bind(socket_addr).map_err(|err| err.to_string())
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

impl KeepAlivePacket {
    pub fn kind(&self) -> &KeepAlivePacketType {
        &self.kind
    }

    pub fn content(&self) -> &KeepAliveContentType {
        &self.content
    }

    pub fn model(&self) -> &ModelName {
        &self.model
    }
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

        Ok((
            input,
            KeepAlivePacket {
                kind,
                subkind,
                model,
                unknown1,
                device_type,
                content,
            },
        ))
    }
}

impl From<KeepAlivePacket> for Bytes {
    fn from(packet: KeepAlivePacket) -> Bytes {
        let mut buf = BytesMut::with_capacity(128);

        buf.extend(Bytes::from(UdpMagic {}));
        buf.extend(Bytes::from(packet.kind));
        buf.extend(&[0x00]);
        buf.extend(Bytes::from(packet.model));
        buf.put_u8(packet.unknown1);
        buf.extend(Bytes::from(packet.device_type));
        buf.put_u8(0u8);
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
            _ => Err(parse_error(input, Switch)),
        }
    }
}

impl From<DeviceType> for Bytes {
    fn from(device_type: DeviceType) -> Bytes {
        let mut buf = BytesMut::new();

        buf.put_u8(match device_type {
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
            _ => Err(parse_error(input, Switch)),
        }
    }
}

impl KeepAlivePacketType {
    fn decode_content<'a>(&self, input: &'a [u8]) -> IResult<&'a [u8], KeepAliveContentType> {
        let (input, decoded_value) = match self {
            Self::Status => Status::decode,
            Self::Hello => Hello::decode,
            Self::Change => Change::decode,
            Self::Number => Number::decode,
            Self::Mac => Mac::decode,
            Self::Ip => Ip::decode,
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
        buf.put_u8(kind);

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
            }),
        ))
    }
}

impl From<Mac> for Bytes {
    fn from(value: Mac) -> Bytes {
        let mut buf = BytesMut::with_capacity(8);

        buf.put_u8(value.iteration);
        buf.put_u8(value.unknown2);
        buf.extend_from_slice(&value.mac_addr.octets());

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
            _ => Err(parse_error(input, Switch)),
        }
    }
}

impl From<PlayerNumberAssignment> for Bytes {
    fn from(value: PlayerNumberAssignment) -> Bytes {
        let mut buf = BytesMut::with_capacity(1);
        buf.put_u8(match value {
            PlayerNumberAssignment::Auto => 1u8,
            PlayerNumberAssignment::Manual => 2u8,
        });

        buf.freeze()
    }
}

#[derive(Debug, PartialEq)]
pub struct Ip {
    ip_addr: Ipv4Addr,
    mac_addr: MacAddr,
    //player_number: u8,
    iteration: u8,
    index: u8,
    player_number_assignment: PlayerNumberAssignment,
}

impl Ip {
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

impl Decoder<KeepAliveContentType> for Ip {
    fn decode(input: &[u8]) -> IResult<&[u8], KeepAliveContentType> {
        let (input, ip_addr) = Ipv4Addr::decode(input)?;
        let (input, mac_addr) = MacAddr::decode(input)?;
        let (input, iteration) = be_u8(input)?;
        let (input, index) = be_u8(input)?;
        let (input, _padding) = be_u8(input)?;
        let (input, player_number_assignment) = PlayerNumberAssignment::decode(input)?;

        Ok((
            input,
            KeepAliveContentType::Ip(Ip {
                ip_addr,
                mac_addr,
                iteration,
                index,
                player_number_assignment,
            }),
        ))
    }
}

impl From<Ip> for Bytes {
    fn from(ip: Ip) -> Bytes {
        let mut buf = BytesMut::with_capacity(16);
        buf.put(ip_addr_to_bytes(ip.ip_addr));
        buf.extend_from_slice(&ip.mac_addr.octets());
        buf.put_u8(ip.map_sequence_byte());
        buf.put_u8(ip.iteration);
        buf.put_u8(4u8);
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

        Ok((
            input,
            KeepAliveContentType::Change(Change {
                old_player_number,
                ip_addr,
            }),
        ))
    }
}

impl From<Change> for Bytes {
    fn from(value: Change) -> Bytes {
        let mut buf = BytesMut::with_capacity(5);

        buf.put_u8(value.old_player_number);
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
            }),
        ))
    }
}

impl From<Number> for Bytes {
    fn from(value: Number) -> Bytes {
        Bytes::from(vec![value.proposed_player_number, value.iteration])
    }
}

#[derive(Debug, PartialEq)]
pub struct Hello {
    unknown2: u8,
}

impl Decoder<KeepAliveContentType> for Hello {
    fn decode(input: &[u8]) -> IResult<&[u8], KeepAliveContentType> {
        let (input, unknown2) = be_u8(input)?;

        Ok((input, KeepAliveContentType::Hello(Hello { unknown2 })))
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
    unknown4: u8,
}

impl Status {
    pub fn ip_addr(&self) -> &Ipv4Addr {
        &self.ip_addr
    }

    pub fn player_number(&self) -> &u8 {
        &self.player_number
    }
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
        let (input, unknown4) = be_u8(input)?;

        Ok((
            input,
            KeepAliveContentType::Status(Status {
                player_number,
                unknown2,
                mac_addr,
                ip_addr,
                device_count,
                unknown3,
                unknown4,
            }),
        ))
    }
}

impl From<Status> for Bytes {
    fn from(value: Status) -> Bytes {
        let mut buf = BytesMut::with_capacity(15);
        buf.put_u8(value.player_number);
        buf.put_u8(value.unknown2);
        buf.extend_from_slice(&value.mac_addr.octets());
        buf.put(ip_addr_to_bytes(value.ip_addr));
        buf.put_u8(value.device_count);
        buf.put_u8(1u8);
        buf.put_u8(0u8);
        buf.extend(Bytes::from(value.unknown3.to_be_bytes().to_vec()));
        buf.put_u8(8u8);

        buf.freeze()
    }
}

trait Decoder<T> {
    fn decode(input: &[u8]) -> IResult<&[u8], T>;
}

impl Decoder<MacAddr> for MacAddr {
    fn decode(input: &[u8]) -> IResult<&[u8], MacAddr> {
        let (input, (a, b, c, d, e, f)) = tuple((be_u8, be_u8, be_u8, be_u8, be_u8, be_u8))(input)?;

        Ok((input, MacAddr::new(a, b, c, d, e, f)))
    }
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

        Ok((
            input,
            match kind[0] {
                0x25 => Self::Hello,
                0x26 => Self::Number,
                0x2c => Self::Mac,
                0x32 => Self::Ip,
                0x36 => Self::Status,
                0x29 => Self::Change,
                0x00 => Self::StatusMixer,
                _ => Self::Unknown(kind[0]),
            },
        ))
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
    use super::super::super::utils::network::PioneerNetwork;
    use super::*;
    use pnet::{datalink::MacAddr, ipnetwork::Ipv4Network};
    use pretty_assertions::assert_eq;

    fn get_pioneer_network() -> PioneerNetwork {
        let addr = Ipv4Addr::new(192, 168, 10, 50);
        let cidr = 24;
        let network = Ipv4Network::new(addr, cidr).unwrap();

        PioneerNetwork::new(network, MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff))
    }

    #[test]
    fn test_mac_iteration() {
        let discovery_sequence_1 = Bytes::from(vec![
            0x51, 0x73, 0x70, 0x74, 0x31, 0x57, 0x6d, 0x4a, 0x4f, 0x4c, 0x00, 0x00, 0x72, 0x65,
            0x6b, 0x6f, 0x72, 0x64, 0x62, 0x6f, 0x78, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x01, 0x03, 0x00, 0x2c, 0x01, 0x04, 0xac, 0x87, 0xa3, 0x35,
            0xbc, 0x4d,
        ]);

        assert_eq!(
            discovery_sequence_1,
            Bytes::from(KeepAlivePacket {
                kind: KeepAlivePacketType::Mac,
                subkind: KeepAlivePacketSubType::Mac,
                model: ModelName::new("rekordbox".to_string()),
                unknown1: 1,
                device_type: DeviceType::Rekordbox,
                content: KeepAliveContentType::Mac(Mac {
                    iteration: 1,
                    unknown2: 4,
                    mac_addr: MacAddr::from([0xac, 0x87, 0xa3, 0x35, 0xbc, 0x4d]),
                },),
            })
        );
    }

    #[test]
    fn test_keepalive_status_packet() {
        assert_eq!(
            Bytes::from(KeepAlivePacket {
                kind: KeepAlivePacketType::Status,
                subkind: KeepAlivePacketSubType::Status,
                model: ModelName::new("Linux".to_string(),),
                unknown1: 1,
                device_type: DeviceType::Rekordbox,
                content: KeepAliveContentType::Status(Status {
                    player_number: 17,
                    unknown2: 1,
                    ip_addr: Ipv4Addr::new(192, 168, 10, 50),
                    mac_addr: MacAddr::from([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
                    device_count: 1,
                    unknown3: 4,
                    unknown4: 1,
                },),
            })
            .len(),
            54
        );
    }

    #[test]
    fn test_decode_keepalive_ip_package() {
        let payload: [u8; 50] = [
            81, 115, 112, 116, 49, 87, 109, 74, 79, 76, 2, 0, 114, 101, 107, 111, 114, 100, 98,
            111, 120, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 3, 0, 50, 192, 168, 10, 47, 156, 182,
            208, 238, 255, 9, 44, 6, 4, 1,
        ];

        assert_eq!(
            KeepAlivePacket {
                kind: KeepAlivePacketType::Ip,
                subkind: KeepAlivePacketSubType::Ip,
                model: ModelName::new("rekordbox".to_string(),),
                unknown1: 1,
                device_type: DeviceType::Rekordbox,
                content: KeepAliveContentType::Ip(Ip {
                    ip_addr: Ipv4Addr::new(192, 168, 10, 47),
                    mac_addr: MacAddr::from([0x9c, 0xb6, 0xd0, 0xee, 0xff, 0x09]),
                    iteration: 44,
                    index: 6,
                    player_number_assignment: PlayerNumberAssignment::Auto,
                },),
            },
            KeepAlivePacket::decode(&payload).unwrap().1,
        );
    }
}
