use std::convert::TryFrom;

use crate::utils::parse_error;
use nom::bytes::complete::{tag, take};
use nom::number::complete::{be_u64, be_u32, be_u16, be_u8};
use nom::{IResult, multi::count};
use bytes::{Bytes, BytesMut, BufMut};
use std::net::Ipv4Addr;
use super::db_field::{DBField, DBFieldType};
use super::db_request_type::DBRequestType;
use super::db_message_argument::ArgumentCollection;
use crate::rekordbox::library::{MetadataType, ROOT_ARTIST};

type DBMessageResult<'a> = IResult<&'a [u8], &'a [u8]>;
type DBMessageU32<'a> = IResult<&'a [u8], u32>;
pub type DBMessageResultType<'a, T> = IResult<&'a [u8], T>;

#[derive(Debug, PartialEq)]
pub struct ManyDBMessages(Vec<DBMessage>);

impl ManyDBMessages {
    pub fn new(messages: Vec<DBMessage>) -> ManyDBMessages {
        ManyDBMessages(messages)
    }

    pub fn push(&mut self, message: DBMessage) {
        self.0.push(message);
    }

    pub fn extend(&mut self, iter: Vec<DBMessage>) {
        self.0.extend(iter);
    }
}

impl IntoIterator for ManyDBMessages {
    type Item = DBMessage;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, PartialEq)]
enum Error {
    ParseError,
}

#[derive(Debug, PartialEq)]
pub struct DBMessage {
    pub transaction_id: DBField,
    pub request_type: DBRequestType,
    pub arguments: ArgumentCollection,
}

pub struct Arguments<'a> {
    pub entry_id1: u32,
    pub entry_id2: u32,
    pub entry_id3: u32,
    pub entry_id4: u32,
    pub value1: &'a str,
    pub value2: &'a str,
    pub _type: MetadataType,
}

impl<'a> Default for Arguments<'a> {
    fn default() -> Self {
        Self {
            entry_id1: 0,
            entry_id2: 0,
            entry_id3: 0,
            entry_id4: 0,
            value1: "",
            value2: "",
            _type: ROOT_ARTIST,
        }
    }
}

impl<'a> From<Arguments<'a>> for ArgumentCollection {
    fn from(arguments: Arguments) -> ArgumentCollection {
        ArgumentCollection::new(vec![
            DBField::from(arguments.entry_id1.to_be_bytes()),
            DBField::from(arguments.entry_id2.to_be_bytes()),
            DBField::from((arguments.value1.encode_utf16().count() * 2 + 2) as u32),
            DBField::from(arguments.value1),
            DBField::from((arguments.value2.encode_utf16().count() * 2 + 2) as u32),
            DBField::from(arguments.value2),
            DBField::from(arguments._type),
            DBField::from(0u32),
            DBField::from(arguments.entry_id3.to_be_bytes()),
            DBField::from(0u32),
            DBField::from(arguments.entry_id4.to_be_bytes()),
            DBField::from(0u32),
        ])
    }
}

impl DBMessage {
    const MAGIC: [u8; 4] = [0x87, 0x23, 0x49, 0xae];

    pub fn new<T: Into<ArgumentCollection>>(
        transaction_id: DBField,
        request_type: DBRequestType,
        arguments: T
    ) -> DBMessage {
        DBMessage {
            transaction_id,
            request_type,
            arguments: arguments.into(),
        }
    }

    fn magic(i: &[u8]) -> DBMessageResult {
        let (i, _) = take(1u8)(i)?;
        tag(DBMessage::MAGIC)(i)
    }

    fn transaction_id(input: &[u8]) -> IResult<&[u8], DBField> {
        let (input, kind) = take(1u8)(input)?;
        let (input, transaction) = take(4u8)(input)?;

        Ok((
            input,
            DBField::new(
                DBFieldType::name(kind[0]).unwrap(),
                &Bytes::from(transaction.to_vec()),
            ),
        ))
    }

    fn request_type(input: &[u8]) -> DBMessageResultType<DBRequestType> {
        let (input, _) = take(1u8)(input)?;

        let request_type: DBMessageResultType<u16> = be_u16(input);
        match request_type {
            Ok((input, data)) => Ok((input, DBRequestType::new(data))),
            Err(err) => Err(err),
        }
    }

    fn arguments(i: &[u8]) -> IResult<&[u8], ArgumentCollection> {
        ArgumentCollection::decode(i)
    }

    pub fn parse(i: &[u8]) -> IResult<&[u8], DBMessage> {
        let (i, _magic) = DBMessage::magic(i)?;
        let (i, transaction_id) = DBMessage::transaction_id(i)?;
        let (i, request_type) = DBMessage::request_type(i)?;
        let (i, arguments) = DBMessage::arguments(i)?;

        Ok((i, DBMessage {
            transaction_id: transaction_id,
            request_type: request_type,
            arguments: arguments,
        }))
    }

    pub fn to_response(&self) -> BytesMut {
        let mut bytes = BytesMut::new();

        bytes.extend(vec![0x11]);
        bytes.extend(DBMessage::MAGIC.to_vec());
        bytes.extend(self.transaction_id.as_bytes());

        bytes
    }
}

impl From<DBMessage> for Bytes {
    fn from(message: DBMessage) -> Bytes {
        let mut buffer: BytesMut = message.to_response();

        buffer.extend(vec![0x10]);
        buffer.extend(&message.request_type.value());
        buffer.extend(Bytes::from(message.arguments));

        Bytes::from(buffer)
    }
}

impl TryFrom<Bytes> for DBMessage {
    type Error = &'static str;

    fn try_from(message: Bytes) -> Result<Self, Self::Error> {
        match DBMessage::parse(&message) {
            Ok((_input, message)) => Ok(message),
            Err(_err) => Err("Failed decoding DBMessage."),
        }
    }
}

impl From<ManyDBMessages> for Bytes {
    fn from(messages: ManyDBMessages) -> Bytes {
        Bytes::from(messages.into_iter().fold(BytesMut::new(), |mut acc, message| {
            acc.extend(Bytes::from(message));
            acc
        }))
    }
}

const UDP_MAGIC: [u8; 10] = [0x51, 0x73, 0x70, 0x74, 0x31, 0x57, 0x6d, 0x4a, 0x4f, 0x4c];


pub mod term_dj {
    use bytes::Bytes;

    const APPLICATION_NAME: &str = "TermDJ";

    pub struct Name;
    impl From<Name> for Bytes {
        fn from(_val: Name) -> Bytes {
            Bytes::from("TermDJ".encode_utf16()
                .into_iter()
                .flat_map(|item| { item.to_be_bytes().to_vec() })
                .collect::<Bytes>())
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ModelName(String);

impl ModelName {
    pub fn new(name: String) -> ModelName { ModelName(name) }

    pub fn decode(input: &[u8]) -> IResult<&[u8], ModelName> {
        let (input, model) = take(20u8)(input)?;

        match String::from_utf8(model.to_vec()) {
            Ok(model) => Ok((input, ModelName(model.trim_end_matches('\u{0}').to_string()))),
            Err(_err) => Err(parse_error(input, nom::error::ErrorKind::Tag)),
        }
    }

    pub fn encode(self) -> Bytes {
        let mut model = self.0.as_bytes().to_vec();
        let padding = 20 - model.len();
        if padding > 0 {
            model.extend(vec![0x00; padding as usize])
        }

        Bytes::from(model)
    }

    pub fn to_string(&self) -> &String {
        &self.0
    }
}

impl From<ModelName> for Bytes {
    fn from(model_name: ModelName) -> Self {
        model_name.encode()
    }
}

#[derive(Debug, PartialEq)]
pub enum PlayerSlot {
  Empty,
  Cd,
  Sd,
  Usb,
  Rekordbox,
  Unknown(u8),
}

impl PlayerSlot {
    fn decode(input: &[u8]) -> IResult<&[u8], PlayerSlot> {
        let (input, value) = be_u8(input)?;

        Ok((input, match value {
            0 => Self::Empty,
            1 => Self::Cd,
            2 => Self::Sd,
            3 => Self::Usb,
            4 => Self::Rekordbox,
            _ => Self::Unknown(value),
        }))
    }
}

impl From<PlayerSlot> for Bytes {
    fn from(slot: PlayerSlot) -> Bytes {
        Bytes::from(vec![match slot {
             PlayerSlot::Empty => 0,
             PlayerSlot::Cd => 1,
             PlayerSlot::Sd => 2,
             PlayerSlot::Usb => 3,
             PlayerSlot::Rekordbox => 4,
             PlayerSlot::Unknown(value) => value,
        }])
    }
}

#[derive(Debug, PartialEq)]
pub struct UdpMagic;

impl UdpMagic {
    pub fn decode(input: &[u8]) -> IResult<&[u8], UdpMagic> {
        let (input, _) = tag(UDP_MAGIC)(input)?;
        Ok((input, UdpMagic))
    }
}

impl From<UdpMagic> for Bytes {
    fn from(_magic: UdpMagic) -> Bytes {
        Bytes::from(UDP_MAGIC.to_vec())
    }
}

#[derive(Debug, PartialEq)]
pub struct StatusPacket {
    kind: StatusPacketType,
    model: ModelName,
    pub unknown1: u8,
    pub player_number: u8,
    content: StatusContentType,
}

impl StatusPacket {
    pub fn new(
        kind: StatusPacketType,
        unknown1: u8,
        player_number: u8,
        content: StatusContentType
    ) -> StatusPacket {
        StatusPacket {
            kind: kind,
            model: ModelName("Linux".to_string()),
            unknown1: unknown1,
            player_number: player_number,
            content: content,
        }
    }

    fn decode(input: &[u8]) -> IResult<&[u8], StatusPacket> {
        let (input, _) = UdpMagic::decode(input)?;
        let (input, kind) = StatusPacketType::decode(input)?;
        let (input, model)  = ModelName::decode(input)?;
        let (input, _unknown) = be_u8(input)?;
        let (input, unknown1) = be_u8(input)?;
        let (input, player_number) = be_u8(input)?;
        let (input, content) = kind.decode_content(input)?;

        Ok((input, StatusPacket {
            kind,
            model,
            unknown1,
            player_number,
            content,
        }))
    }

    pub fn kind(&self) -> &StatusPacketType {
        &self.kind
    }
}

impl From<StatusPacket> for Bytes {
    fn from(packet: StatusPacket) -> Bytes {
        let mut buffer = BytesMut::new();

        buffer.extend(Bytes::from(UdpMagic));
        buffer.extend(Bytes::from(packet.kind.clone()));
        buffer.extend(Bytes::from(packet.model));
        buffer.extend(Bytes::from(vec![
            0x01, // some const value
            packet.unknown1,
            0x11, // rekordbox, players use their player_number here.
        ]));
        buffer.extend(Bytes::from(packet.content));

        Bytes::from(buffer)
    }
}

impl TryFrom<Bytes> for StatusPacket {
    type Error = &'static str;

    fn try_from(message: Bytes) -> Result<Self, Self::Error> {
        match StatusPacket::decode(&message) {
            Ok((_input, message)) => Ok(message),
            Err(_err) => Err("Failed decoding StatusPacket."),
        }
    }
}

impl TryFrom<&[u8]> for StatusPacket {
    type Error = &'static str;

    fn try_from(message: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from(Bytes::from(message.to_vec()))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum StatusPacketType {
    Cdj,
    Djm,
    LoadCmd,
    LoadCmdReply,
    LinkQuery,
    LinkReply,
    RekordboxHello,
    RekordboxReply,
    Unknown(u8),
}

impl StatusPacketType {
    fn decode(input: &[u8]) -> IResult<&[u8], StatusPacketType> {
        let (input, kind) = take(1u8)(input)?;

        Ok((input, match kind[0] {
            0x0a => StatusPacketType::Cdj,
            0x29 => StatusPacketType::Djm,
            0x19 => StatusPacketType::LoadCmd,
            0x1a => StatusPacketType::LoadCmdReply,
            0x05 => StatusPacketType::LinkQuery,
            0x06 => StatusPacketType::LinkReply,
            0x10 => StatusPacketType::RekordboxHello,
            0x11 => StatusPacketType::RekordboxReply,
            _    => StatusPacketType::Unknown(kind[0]),
        }))
    }

    fn decode_content<'a>(&self, input: &'a [u8]) -> IResult<&'a [u8], StatusContentType> {
        let (input, decoded_value) = (match self {
            StatusPacketType::LinkQuery      => LinkQuery::decode,
            StatusPacketType::RekordboxHello => RekordboxHello::decode,
            StatusPacketType::LinkReply      => LinkReply::decode,
            StatusPacketType::Cdj            => Cdj::decode,
            StatusPacketType::Djm            => Djm::decode,
            _ => {
                eprintln!("{:?}", self);
                unimplemented!()
            },
        })(input)?;
        Ok((input, decoded_value))
    }
}

impl From<StatusPacketType> for Bytes {
    fn from(packet_type: StatusPacketType) -> Bytes {
        Bytes::from([match packet_type {
            StatusPacketType::Cdj => 0x0a,
            StatusPacketType::Djm => 0x29,
            StatusPacketType::LoadCmd => 0x19,
            StatusPacketType::LoadCmdReply => 0x1a,
            StatusPacketType::LinkQuery => 0x05,
            StatusPacketType::LinkReply => 0x06,
            StatusPacketType::RekordboxHello => 0x10,
            StatusPacketType::RekordboxReply => 0x11,
            StatusPacketType::Unknown(val) => val,
        }].to_vec())
    }
}

#[derive(Debug, PartialEq)]
pub struct Utf16FixedString {
    capacity: usize,
    value: String,
}

impl Utf16FixedString {
    pub fn new(value: String, capacity: usize) -> Utf16FixedString {
        Utf16FixedString {
            value,
            capacity,
        }
    }

    fn decode(input: &[u8], capacity: usize) -> IResult<&[u8], Utf16FixedString> {
        let (input, value) = count(be_u16, capacity / 2)(input)?;

        let value = match String::from_utf16(&value) {
            Ok(val)  => val,
            Err(_err) => return Err(parse_error(input, nom::error::ErrorKind::Tag)),
        };

        Ok((input, Utf16FixedString {
            value: value,
            capacity: capacity
        }))
    }
}

impl From<Utf16FixedString> for Bytes {
    fn from(fixed_string: Utf16FixedString) -> Bytes {
        let mut encoded = fixed_string.value
            .encode_utf16()
            .into_iter()
            .flat_map(|item| { item.to_be_bytes().to_vec() })
            .collect::<Vec<u8>>();
        encoded.extend(vec![0x00; fixed_string.capacity - encoded.len()]);

        Bytes::from(encoded)
    }
}

#[derive(Debug, PartialEq)]
pub enum TrackAnalyzeType {
    Unknown,
    Rekordbox,
    File,
    Cd,
}

impl Decode for TrackAnalyzeType {
    type Item = TrackAnalyzeType;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Item> {
        let (input, track_analyze_type) = be_u8(input)?;

        Ok((
            input,
            match track_analyze_type {
                0 => TrackAnalyzeType::Unknown,
                1 => TrackAnalyzeType::Rekordbox,
                2 => TrackAnalyzeType::File,
                5 => TrackAnalyzeType::Cd,
                _ => TrackAnalyzeType::Unknown,
            }
        ))
    }
}

#[derive(Debug, PartialEq)]
pub struct Cdj {
    activity: u16,
    loaded_player_number: u16,
    loaded_slot: PlayerSlot,
    track_analyze_type: TrackAnalyzeType,
    track_id: u32,
    track_number: u32,
}

trait Decode {
    type Item;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Item>;
}

impl Decode for Cdj {
    type Item = StatusContentType;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Item> {
        let (input, activity) = be_u16(input)?;
        let (input, loaded_player_number) = be_u16(input)?;
        let (input, loaded_slot) = PlayerSlot::decode(input)?;
        let (input, track_analyze_type) = TrackAnalyzeType::decode(input)?;
        let (input, _padding) = take(1u8)(input)?;
        let (input, track_id) = be_u32(input)?;
        let (input, track_number) = be_u32(input)?;

        Ok((
            input,
            StatusContentType::Cdj(Cdj {
                activity,
                loaded_player_number,
                loaded_slot,
                track_analyze_type,
                track_id,
                track_number,
            })
        ))
    }
}

#[derive(Debug, PartialEq)]
pub struct Djm;

impl Decode for Djm {
    type Item = StatusContentType;
    fn decode(input: &[u8]) -> IResult<&[u8], Self::Item> {
        Ok((input, StatusContentType::Djm(Djm)))
    }
}

#[derive(Debug, PartialEq)]
pub struct LinkReply {
    pub source_player_number: u8,
    pub slot: PlayerSlot,
    pub name: Utf16FixedString,
    pub date: Utf16FixedString,
    pub unknown5: Utf16FixedString,
    pub track_count: u32,
    pub unknown6: u16,
    pub unknown7: u16,
    pub playlist_count: u32,
    pub bytes_total: u64,
    pub bytes_free: u64,
}

impl LinkReply {
    fn decode(input: &[u8]) -> IResult<&[u8], StatusContentType> {
        let (input, slot) = PlayerSlot::decode(input)?;
        let (input, name) = Utf16FixedString::decode(input, 64)?;
        let (input, date) = Utf16FixedString::decode(input, 24)?;
        let (input, unknown5) = Utf16FixedString::decode(input, 32)?;
        let (input, track_count) = be_u32(input)?;
        let (input, unknown6) = be_u16(input)?;
        let (input, unknown7) = be_u16(input)?;
        let (input, playlist_count) = be_u32(input)?;
        let (input, bytes_total) = be_u64(input)?;
        let (input, bytes_free) = be_u64(input)?;

        Ok((
            input,
            StatusContentType::LinkReply(LinkReply {
                source_player_number: 0x11,
                slot: slot,
                name: name,
                date: date,
                unknown5: unknown5,
                track_count: track_count,
                unknown6: unknown6,
                unknown7: unknown7,
                playlist_count: playlist_count,
                bytes_total: bytes_total,
                bytes_free: bytes_free,
            }),
        ))
    }
}

impl From<LinkReply> for Bytes {
    fn from(reply: LinkReply) -> Bytes {
        let mut buf = BytesMut::new();

        buf.extend((156 as u16).to_be_bytes().to_vec());
        buf.extend(vec![0x00, 0x00, 0x00]);
        buf.put_u8(reply.source_player_number);
        buf.extend(vec![0x00, 0x00, 0x00]);
        buf.extend(Bytes::from(reply.slot));
        buf.extend(Bytes::from(reply.name));
        buf.extend(Bytes::from(reply.date));
        buf.extend(Bytes::from(reply.unknown5));
        buf.extend(Bytes::from(reply.track_count.to_be_bytes().to_vec()));
        buf.extend(Bytes::from(reply.unknown6.to_be_bytes().to_vec()));
        buf.extend(Bytes::from(reply.unknown7.to_be_bytes().to_vec()));
        buf.extend(Bytes::from(reply.playlist_count.to_be_bytes().to_vec()));
        buf.extend(Bytes::from(reply.bytes_total.to_be_bytes().to_vec()));
        buf.extend(Bytes::from(reply.bytes_free.to_be_bytes().to_vec()));

        buf.freeze()
    }
}

#[derive(Debug, PartialEq)]
pub struct LinkQuery {
    source_ip: Ipv4Addr,
    remote_player_number: u8,
    slot: PlayerSlot,
}

impl LinkQuery {
    fn decode(input: &[u8]) -> IResult<&[u8], StatusContentType> {
        let (input, _u3) = be_u16(input)?;
        let (input, source_ip) = take(4u8)(input)?;
        let (input, _padding) = take(3u8)(input)?;
        let (input, remote_player_number) = be_u8(input)?;
        let (input, _padding) = take(3u8)(input)?;
        let (input, slot) = PlayerSlot::decode(input)?;

        Ok((input, StatusContentType::LinkQuery(LinkQuery {
            source_ip: Ipv4Addr::new(
                source_ip[0],
                source_ip[1],
                source_ip[2],
                source_ip[3],
            ),
            remote_player_number: remote_player_number,
            slot: slot,
        })))
    }
}

struct RekordboxHello;
impl RekordboxHello {
    fn decode(input: &[u8]) -> IResult<&[u8], StatusContentType> {
        Ok((input, StatusContentType::RekordboxHello))
    }
}

#[derive(Debug, PartialEq)]
pub enum StatusContentType {
    Cdj(Cdj),
    Djm(Djm),
    LoadCmd,
    LoadCmdReply,
    LinkQuery(LinkQuery),
    LinkReply(LinkReply),
    RekordboxHello,
    RekordboxReply(RekordboxReply),
    Unknown(u8),
}

impl From<StatusContentType> for Bytes {
    fn from(status_content_type: StatusContentType) -> Bytes {
        match status_content_type {
            StatusContentType::RekordboxReply(reply) => Bytes::from(reply),
            StatusContentType::LinkReply(reply) => Bytes::from(reply),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct RekordboxReply {
    pub name: String,
}

impl From<RekordboxReply> for Bytes {
    fn from(reply: RekordboxReply) -> Bytes {
        let mut buffer = BytesMut::new();

        buffer.extend(vec![
            0x01, 0x04,
            0x11, 0x01,
            0x00, 0x00, // padding
        ]);

        // Convert to Utf16FixedString
        let mut encoded_name = reply.name
            .encode_utf16()
            .into_iter()
            .flat_map(|item| { item.to_be_bytes().to_vec() })
            .collect::<Vec<u8>>();
        encoded_name.extend(vec![0x00; 256 - encoded_name.len()]);
        buffer.extend(encoded_name);

        buffer.freeze()
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use super::super::fixtures;
    use pretty_assertions::assert_eq;
    use super::super::db_field::{DBField, DBFieldType, Binary};
    use crate::rekordbox::library::ARTIST;

    #[test]
    fn extract_magic_from_db_message() {
        assert_eq!(
            Ok((&[0x11][..], &[135, 35, 73, 174][..])),
            DBMessage::magic(&[0x11, 0x87, 0x23, 0x49, 0xae, 0x11]),
        );
    }

    #[test]
    fn parse_transaction_id() {
        assert_eq!(
            Ok((&[0x20][..], DBField::from([0x00, 0x00, 0x00, 0x01]))),
            DBMessage::transaction_id(&[DBFieldType::U32.value(), 0x00, 0x00, 0x00, 0x01, 0x20]),
        );
        assert_eq!(
            Ok((&[0x20][..], DBField::new(DBFieldType::U32, &[0x00, 0x00, 0x01, 0x00]))),
            DBMessage::transaction_id(&[DBFieldType::U32.value(), 0x00, 0x00, 0x01, 0x00, 0x20]),
        );
        assert_eq!(
            Ok((&[0x20, 0x21][..], DBField::new(DBFieldType::U32, &[0x05, 0x80, 0x00, 0x32]))),
            DBMessage::transaction_id(&[DBFieldType::U32.value(), 0x05, 0x80, 0x00, 0x32, 0x20, 0x21]),
        );
    }

    #[test]
    fn parse_request_types() {
        assert_eq!(
            Ok((&[][..], DBRequestType::Setup)),
            DBMessage::request_type(&[DBFieldType::U16 as u8, 0x00, 0x00]),
        );

        // Verify parsing only 3 bytes (size identifier + u16)
        assert_eq!(
            Ok((&[0x00][..], DBRequestType::Setup)),
            DBMessage::request_type(&[DBFieldType::U16 as u8, 0x00, 0x00, 0x00]),
        );

        // Verify matching unknown packages (and that data is kept for debug
        assert_eq!(
            Ok((&[][..], DBRequestType::Unknown(255_u16))),
            DBMessage::request_type(&[DBFieldType::U16 as u8, 0x00, 0xff]),
        );
    }

    #[test]
    fn verify_title_request_parsing() {
        assert_eq!(
            Ok((&[][..], DBRequestType::TitleRequest)),
            DBMessage::request_type(&[0x10, 0x10, 0x04]),
        );
    }

    #[test]
    fn verify_album_by_artist_request() {
        assert_eq!(
            Ok((&[][..], DBRequestType::AlbumByArtistRequest)),
            DBMessage::request_type(&[0x10, 0x11, 0x02]),
        )
    }

    #[test]
    fn dbmessage_setup_request_and_response() {
        let dbmessage = fixtures::setup_request_packet().unwrap().1;
        assert_eq!(DBRequestType::Setup, dbmessage.request_type);
    }

    #[test]
    fn parse_test_library_handler() {
        assert_eq!(
            Err(parse_error(&[0, 0, 0, 1][..], nom::error::ErrorKind::Tag)),
            DBMessage::parse(b"\x11\x00\x00\x00\x01"),
        );
    }

    #[test]
    fn parse_dbmessage() {
        assert_eq!(
            Ok((&[][..], DBMessage {
                transaction_id: DBField::from([0x05, 0x80, 0x00, 0x32]),
                request_type: DBRequestType::RootMenuRequest,
                arguments: ArgumentCollection::new(vec![
                    DBField::from([0x02, 0x01, 0x04, 0x01]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0xff, 0xff, 0xff]),
                ]),
            })),
            DBMessage::parse(&fixtures::root_menu_dialog().0),
        );
    }

    #[test]
    fn try_parse_dbmessage_with_broken_magic() {
        let message = [0x49, 0xae, 0x11, 0x05, 0x80];

        // First byte is consumed so skip that when asserting
        assert_eq!(
            Err(parse_error(&message[1..], nom::error::ErrorKind::Tag)),
            DBMessage::parse(&message),
        );
    }

    #[test]
    fn encode_db_field_to_bytes() {
        assert_eq!(b"\x0f\x32", &DBField::from(0x32u8).as_bytes()[..]);
        assert_eq!(b"\x10\x12\x13", &DBField::from([0x12u8, 0x13u8]).as_bytes()[..]);
        assert_eq!(
            b"\x11\x00\x00\x00\x01",
            &DBField::from([0x00, 0x00, 0x00, 0x01]).as_bytes()[..],
        );
    }

    #[test]
    fn test_binary_parsing() {
        let mut message = vec![
            0x11, 0x87, 0x23, 0x49, 0xae, 0x11, 0x05, 0x80,
            0x00, 0x1e, 0x10, 0x20, 0x04, 0x0f, 0x01, 0x14,
            0x00, 0x00, 0x00, 0x0c, 0x03, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let binary_data = vec![
            0x00, 0x00, 0x00, 0x38, 0x38, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0xe8, 0x03, 0x9b, 0x2a, 0x01, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        message.extend(binary_data.clone());

        assert_eq!(
            Ok((&[][..], DBMessage::new(
                DBField::from([0x05, 0x80, 0x00, 0x1e]),
                DBRequestType::PreviewWaveformRequest,
                ArgumentCollection::new(vec![
                    DBField::from(Binary::new(Bytes::from(binary_data[4..].to_vec()))),
                ]),
            ))),
            DBMessage::parse(&message),
        );
    }

    #[test]
    fn parse_packet_with_missing_binary_argument() {
        let data = vec![
            0x11, 0x87, 0x23, 0x49, 0xae,
            0x11, 0x05, 0x80, 0x00, 0x51,
            0x10, 0x20, 0x04,
            0x0f, 0x05, 0x14, 0x00, 0x00, 0x00, 0x0c,
            0x06, 0x06, 0x06, 0x06, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x01, 0x08, 0x04, 0x01,
            0x11, 0x00, 0x00, 0x00, 0x04,
            0x11, 0x00, 0x00, 0x00, 0x05,
            0x11, 0x00, 0x00, 0x00, 0x00
        ];

        assert_eq!(Ok((&[][..], DBMessage::new(
            DBField::from([0x05, 0x80, 0x00, 0x51]),
            DBRequestType::PreviewWaveformRequest,
            ArgumentCollection::new(vec![
                DBField::from([0x01, 0x08, 0x04, 0x01]),
                DBField::from([0x00, 0x00, 0x00, 0x04]),
                DBField::from([0x00, 0x00, 0x00, 0x05]),
                DBField::from([0x00, 0x00, 0x00, 0x00]),
                DBField::from(Binary::new(Bytes::new())),
            ]),
        ))), DBMessage::parse(&data));
    }

    #[test]
    fn decode_link_query() {
        assert_eq!(Ok((&[][..], StatusPacket {
            kind: StatusPacketType::LinkQuery,
            model: ModelName("XDJ-700".to_string()),
            unknown1: 0,
            player_number: 1,
            content: StatusContentType::LinkQuery(LinkQuery {
                source_ip: Ipv4Addr::new(192, 168, 10, 58),
                remote_player_number: 17,
                slot: PlayerSlot::Rekordbox,
            }),
        })), StatusPacket::decode(&fixtures::link_query()[..48]))
    }

    #[test]
    fn decode_rekordbox_hello() {
        assert_eq!(Ok((&[][..], StatusPacket {
            kind: StatusPacketType::RekordboxHello,
            model: ModelName("XDJ-700".to_string()),
            unknown1: 0,
            player_number: 1,
            content: StatusContentType::RekordboxHello,
        })), StatusPacket::decode(&fixtures::rekordbox_hello()[..34]))
    }

    #[test]
    fn encode_rekordbox_reply() {
        assert_eq!(
            Bytes::from(StatusPacket {
                kind: StatusPacketType::RekordboxReply,
                model: ModelName("Linux".to_string()),
                unknown1: 1,
                player_number: 1,
                content: StatusContentType::RekordboxReply(RekordboxReply {
                    name: "Term DJ".to_string(),
                })
            }),
            Bytes::from(vec![
                0x51, 0x73, 0x70, 0x74, 0x31, 0x57, 0x6d, 0x4a,
                0x4f, 0x4c, 0x11, 0x4c, 0x69, 0x6e, 0x75, 0x78,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
                0x01, 0x11, 0x01, 0x04, 0x11, 0x01, 0x00, 0x00,
                0x00, 0x54, 0x00, 0x65, 0x00, 0x72, 0x00, 0x6d,
                0x00, 0x20, 0x00, 0x44, 0x00, 0x4a, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ])
        );
    }

    #[test]
    #[ignore]
    fn build_discovery_sequence() {
        let _data = Bytes::from(vec![
            0x51, 0x73, 0x70, 0x74, 0x31, 0x57, 0x6d, 0x4a,
            0x4f, 0x4c, 0x29, 0x72, 0x65, 0x6b, 0x6f, 0x72,
            0x64, 0x62, 0x6f, 0x78, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
            0x01, 0x11, 0x00, 0x38, 0x11, 0x00, 0x00, 0xc0,
            0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x10, 0x00, 0x00, 0x00, 0x09, 0xff, 0x00
        ]);
    }

    #[test]
    fn encode_linking_reply() {
        assert_eq!(
            Bytes::from(vec![
                0x51, 0x73, 0x70, 0x74, 0x31, 0x57, 0x6d, 0x4a, 0x4f, 0x4c, 0x06, 0x72, 0x65, 0x6b, 0x6f, 0x72,
                0x64, 0x62, 0x6f, 0x78, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
                0x01, 0x11, 0x00, 0x9c, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x04, 0x00, 0x72, 0x00, 0x65,
                0x00, 0x6b, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x64, 0x00, 0x62, 0x00, 0x6f, 0x00, 0x78, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x1b, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x5e,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ]),
            Bytes::from(StatusPacket {
                kind: StatusPacketType::LinkReply,
                model: ModelName("rekordbox".to_string()),
                unknown1: 1,
                player_number: 1,
                content: StatusContentType::LinkReply(LinkReply {
                    source_player_number: 0x11,
                    slot: PlayerSlot::Rekordbox,
                    name: Utf16FixedString {
                        value: "rekordbox".to_string(),
                        capacity: 64,
                    },
                    date: Utf16FixedString {
                        value: "".to_string(),
                        capacity: 24,
                    },
                    unknown5: Utf16FixedString {
                        value: "".to_string(),
                        capacity: 32,
                    },
                    track_count: 1051,
                    unknown6: 0,
                    unknown7: 257,
                    playlist_count: 94,
                    bytes_total: 0,
                    bytes_free: 0,
                }),
            }),
        );
    }

    #[test]
    fn build_term_dj_status_message() {
        assert_eq!(
            StatusPacket::try_from(Bytes::from(vec![
                0x51, 0x73, 0x70, 0x74, 0x31, 0x57, 0x6d, 0x4a,
                0x4f, 0x4c, 0x29, 0x72, 0x65, 0x6b, 0x6f, 0x72,
                0x64, 0x62, 0x6f, 0x78, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
                0x01, 0x11, 0x00, 0x38, 0x11, 0x00, 0x00, 0xc0,
                0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x10, 0x00, 0x00, 0x00, 0x09, 0xff, 0x00,
            ])).is_ok(),
            true,
        );
    }

    #[test]
    fn it_can_decode_arguments_into_argument_collection() {
        let argument = Arguments {
            value1: "Loopmasters",
            value2: "",
            entry_id1: 1,
            entry_id2: 2,
            _type: ARTIST,
            entry_id3: 3,
            entry_id4: 0,
        };

        assert_eq!(ArgumentCollection::from(argument), ArgumentCollection::new(vec![
            DBField::from(1u32),
            DBField::from(2u32),
            DBField::from(0x18 as u32),
            DBField::from("Loopmasters"),
            DBField::from(2u32),
            DBField::from(""),
            DBField::from(ARTIST),
            DBField::from(0u32),
            DBField::from(3u32),
            DBField::from(0u32),
            DBField::from(0u32),
            DBField::from(0u32),
        ]));
    }

    #[test]
    fn it_can_build_mount_info_package() {
        let package = Bytes::from(vec![
            0x11,0x87,
            0x23,0x49,0xae,0x11,0x05,0x80,0x04,0x52,0x10,0x41,0x01,0x0f,0x0c,0x14,0x00,0x00,
            0x00,0x0c,0x06,0x06,0x06,0x02,0x06,0x02,0x06,0x06,0x06,0x06,0x06,0x06,0x11,0x00,
            0x56,0xd8,0xd3,0x11,0x00,0x00,0x00,0x6b,0x11,0x00,0x00,0x00,0xfa,0x26,0x00,0x00,
            0x00,0x7d,0x00,0x2f,0x00,0x56,0x00,0x6f,0x00,0x6c,0x00,0x75,0x00,0x6d,0x00,0x65,
            0x00,0x73,0x00,0x2f,0x00,0x6d,0x00,0x75,0x00,0x7a,0x00,0x69,0x00,0x6b,0x00,0x61,
            0x00,0x2f,0x00,0x69,0x00,0x54,0x00,0x75,0x00,0x6e,0x00,0x65,0x00,0x73,0x00,0x2f,
            0x00,0x69,0x00,0x54,0x00,0x75,0x00,0x6e,0x00,0x65,0x00,0x73,0x00,0x20,0x00,0x4d,
            0x00,0x65,0x00,0x64,0x00,0x69,0x00,0x61,0x00,0x2f,0x00,0x4d,0x00,0x75,0x00,0x73,
            0x00,0x69,0x00,0x63,0x00,0x2f,0x00,0x53,0x00,0x77,0x00,0x65,0x00,0x64,0x00,0x69,
            0x00,0x73,0x00,0x68,0x00,0x20,0x00,0x48,0x00,0x6f,0x00,0x75,0x00,0x73,0x00,0x65,
            0x00,0x20,0x00,0x4d,0x00,0x61,0x00,0x66,0x00,0x69,0x00,0x61,0x00,0x20,0x00,0x76,
            0x00,0x73,0x00,0x20,0x00,0x4b,0x00,0x6e,0x00,0x69,0x00,0x66,0x00,0x65,0x00,0x20,
            0x00,0x50,0x00,0x61,0x00,0x72,0x00,0x74,0x00,0x79,0x00,0x2f,0x00,0x4e,0x00,0x6f,
            0x00,0x77,0x00,0x20,0x00,0x54,0x00,0x68,0x00,0x61,0x00,0x74,0x00,0x27,0x00,0x73,
            0x00,0x20,0x00,0x57,0x00,0x68,0x00,0x61,0x00,0x74,0x00,0x20,0x00,0x49,0x00,0x20,
            0x00,0x43,0x00,0x61,0x00,0x6c,0x00,0x6c,0x00,0x20,0x00,0x4d,0x00,0x75,0x00,0x73,
            0x00,0x69,0x00,0x63,0x00,0x20,0x00,0x38,0x00,0x31,0x00,0x2f,0x00,0x33,0x00,0x31,
            0x00,0x20,0x00,0x41,0x00,0x6e,0x00,0x74,0x00,0x69,0x00,0x64,0x00,0x6f,0x00,0x74,
            0x00,0x65,0x00,0x2e,0x00,0x6d,0x00,0x70,0x00,0x33,0x00,0x00,0x11,0x00,0x00,0x00,
            0x02,0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,
            0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,
            0x00,0x11,0x00,0x00,0x00,0x00
        ]);
        assert_eq!(true, DBMessage::try_from(package.clone()).is_ok());
    }
}
