use nom::multi::count;
use nom::bytes::complete::{tag, take};
use nom::number::complete::{be_u32, be_u16, be_u8};
use nom::IResult;
use bytes::{Bytes, BytesMut};

#[derive(Debug, PartialEq)]
enum Error {
    ParseError,
}

#[derive(Debug, PartialEq, Clone)]
pub enum DBFieldType {
  U8,
  U16,
  U32,
  Binary,
  String,
}

impl DBFieldType {
    pub fn name(value: u8) -> Result<DBFieldType, &'static str> {
        Ok(match value {
            0x0f => DBFieldType::U8,
            0x10 => DBFieldType::U16,
            0x11 => DBFieldType::U32,
            0x14 => DBFieldType::Binary,
            0x26 => DBFieldType::String,
            _ => {
                return Err("unmatched type.")
            },
        })
    }

    pub fn value(&self) -> u8 {
        match *self {
            DBFieldType::U8 => 0x0f,
            DBFieldType::U16 => 0x10,
            DBFieldType::U32 => 0x11,
            DBFieldType::Binary => 0x14,
            DBFieldType::String => 0x26,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct DBField {
    kind: DBFieldType,
    value: Bytes,
}

impl DBField {
    pub fn new(kind: DBFieldType, value: &[u8]) -> Self {
        Self {
            kind,
            value: Bytes::from(value),
        }
    }

    pub fn u32(value: &[u8]) -> Self {
        DBField::new(DBFieldType::U32, value)
    }

    pub fn u16(value: &[u8]) -> Self {
        DBField::new(DBFieldType::U16, value)
    }

    pub fn u8(value: &[u8]) -> Self {
        DBField::new(DBFieldType::U8, value)
    }

    pub fn binary(value: &[u8]) -> Self {
        DBField::new(DBFieldType::Binary, value)
    }

    pub fn string(value: &str, wrapped: bool) -> Self {
        let mut bytes = BytesMut::new();
        let encoded: std::str::EncodeUtf16 = value.encode_utf16();
        let extra_bytes = if wrapped { 4 } else { 0 };

        if value.len() > 0 {
            let data: Bytes = encoded
                .into_iter()
                .flat_map(|item| { item.to_be_bytes().to_vec() })
                .collect();

            bytes.extend(&
                (((value.len() as u32 * 2) + extra_bytes)/2+1).to_be_bytes());
            if wrapped {
                bytes.extend(&[0xff, 0xfa]);
                bytes.extend(data);
                bytes.extend(&[0xff, 0xfb]);
            } else {
                bytes.extend(data);
            }
        } else {
            bytes.extend(&[0x00, 0x00, 0x00, 0x01]);
        }

        // Append padding
        bytes.extend(&[0x00, 0x00]);

        DBField {
            kind: DBFieldType::String,
            value: Bytes::from(bytes),
        }
    }

    pub fn as_bytes(&self) -> Bytes {
        let mut bytes = Bytes::from(vec![]);

        bytes.extend(vec![self.kind.value()]);
        bytes.extend(self.value.to_vec());

        bytes
    }
}

impl From<DBField> for Bytes {
    fn from(field: DBField) -> Self {
        field.as_bytes()
    }
}

#[derive(Debug, PartialEq)]
pub struct DBMessage<'a> {
    pub transaction_id: DBField,
    pub request_type: DBRequestType,
    pub argument_count: u8,
    pub arg_types: &'a [u8],
    pub args: Vec<DBField>,
}

type DBMessageResult<'a> = IResult<&'a [u8], &'a [u8]>;
type DBMessageU32<'a> = IResult<&'a [u8], u32>;
pub type DBMessageResultType<'a, T> = IResult<&'a [u8], T>;

impl<'a> DBMessage<'a> {
    const MAGIC: [u8; 4] = [0x87, 0x23, 0x49, 0xae];

    pub fn new(
        transaction_id: DBField,
        request_type: DBRequestType,
        argument_count: u8,
        arg_types: &'a [u8],
        args: Vec<DBField>
    ) -> DBMessage<'a> {
        DBMessage {
            transaction_id,
            request_type,
            argument_count,
            arg_types,
            args,
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
            DBField {
                kind: DBFieldType::name(kind[0]).unwrap(),
                value: Bytes::from(transaction),
            },
        ))
    }

    fn request_type(input: &[u8]) -> DBMessageResultType<DBRequestType> {
        let (input, _) = take(1u8)(input)?;

        let request_type: DBMessageResultType<u16> = be_u16(input);
        match request_type {
            Ok((input, 0_u16))    => Ok((input, DBRequestType::Setup)),
            Ok((input, 4096_u16)) => Ok((input, DBRequestType::RootMenuRequest)),
            Ok((input, 4097_u16)) => Ok((input, DBRequestType::GenreRequest)),
            Ok((input, 4098_u16)) => Ok((input, DBRequestType::ArtistRequest)),
            Ok((input, 4099_u16)) => Ok((input, DBRequestType::AlbumRequest)),
            Ok((input, 4100_u16)) => Ok((input, DBRequestType::TitleRequest)),
            Ok((input, 4114_u16)) => Ok((input, DBRequestType::HistoryRequest)),
            Ok((input, 4116_u16)) => Ok((input, DBRequestType::KeyRequest)),
            Ok((input, 4357_u16)) => Ok((input, DBRequestType::PlaylistRequest)),
            Ok((input, 4864_u16)) => Ok((input, DBRequestType::SearchQueryRequest)),
            Ok((input, 12288_u16)) => Ok((input, DBRequestType::RenderRequest)),
            Ok((input, 16385_u16)) => Ok((input, DBRequestType::MenuHeader)),
            Ok((input, 16641_u16)) => Ok((input, DBRequestType::MenuItem)),
            Ok((input, 16897_u16)) => Ok((input, DBRequestType::MenuFooter)),

            Ok((input, data))     => {
                eprintln!("{:?}", input);
                Ok((input, DBRequestType::Unknown(data)))
            },
            Err(err) => Err(err),
        }
    }

    fn argument_count(i: &[u8]) -> DBMessageResultType<u8> {
        let (i, _) = take(1u8)(i)?;
        be_u8(i)
    }

    fn arg_types(i: &[u8]) -> DBMessageResult {
        let (i, _) = take(1u8)(i)?;
        take(16u8)(i)
    }

    pub fn parse(i: &[u8]) -> IResult<&[u8], DBMessage> {
        fn parse_arguments(input: &[u8]) -> IResult<&[u8], DBField> {
            match be_u8(input) {
                Err(err) => Err(err),
                Ok((input, 0x11)) => {
                    match take(4u8)(input) {
                        Ok((input, value)) => {
                            Ok((input, DBField::u32(value)))
                        },
                        Err(err) => Err(err),
                    }
                },
                Ok((_input, _consumed)) => Err(nom::Err::Error((&[], nom::error::ErrorKind::Tag))),
            }
        }

        let (i, _magic) = DBMessage::magic(i)?;
        let (i, transaction_id) = DBMessage::transaction_id(i)?;
        let (i, request_type) = DBMessage::request_type(i)?;
        let (i, argument_count) = DBMessage::argument_count(i)?;
        let (i, arg_types) = DBMessage::arg_types(i)?;
        let (i, args) = count(parse_arguments, argument_count as usize)(i)?;

        Ok((i, DBMessage {
            transaction_id: transaction_id,
            request_type: request_type,
            argument_count: argument_count,
            arg_types: arg_types,
            args: args,
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

impl<'a> From<DBMessage<'a>> for Bytes {
    fn from(message: DBMessage) -> Bytes {
        let mut buffer: BytesMut = message.to_response();

        buffer.extend(vec![0x10]);
        buffer.extend(&message.request_type.value());
        buffer.extend(vec![0x0f, message.argument_count]);
        buffer.extend(Bytes::from(DBField::binary(message.arg_types)));

        for arg in message.args {
            buffer.extend(arg.as_bytes());
        }

        Bytes::from(buffer)
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum DBRequestType {
    AlbumRequest,
    ArtistRequest,
    GenreRequest,
    HistoryRequest,
    KeyRequest,
    MenuFooter,
    MenuHeader,
    MenuItem,
    PlaylistRequest,
    RenderRequest,
    RootMenuRequest,
    SearchQueryRequest,
    Setup,
    TitleRequest,
    Unknown(u16),
}

impl DBRequestType {
    fn value(&self) -> Bytes {
        Bytes::from(match self {
            DBRequestType::Setup => vec![0x00, 0x00],
            DBRequestType::MenuHeader => vec![0x40, 0x01],
            DBRequestType::MenuFooter => vec![0x42, 0x01],
            DBRequestType::MenuItem => vec![0x41, 0x01],
            _ => vec![0x00, 0x00],
        })
    }
}

#[cfg(test)]
pub mod fixtures {
    use super::{DBMessage};
    use nom::IResult;
    use bytes::Bytes;

    type DBMessageParseResult<'a> = IResult<&'a [u8], DBMessage<'a>>;
    type DBMessageParse<'a> = DBMessage<'a>;

    pub fn setup_request_packet<'a>() -> DBMessageParseResult<'a> {
        DBMessage::parse(&[
            0x11, 0x87, 0x23, 0x49, 0xae, 0x11, 0xff, 0xff,
            0xff, 0xfe, 0x10, 0x00, 0x00, 0x0f, 0x01, 0x14,
            0x00, 0x00, 0x00, 0x0c, 0x06, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x02,
        ])
    }

    pub fn setup_response_packet() -> Bytes {
        Bytes::from(vec![
            0x11, 0x87, 0x23, 0x49, 0xae, 0x11, 0xff, 0xff,
            0xff, 0xfe, 0x10, 0x40, 0x00, 0x0f, 0x02, 0x14,
            0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00,
            0x00, 0x11,
        ])
    }

    pub fn root_menu_request<'a>() -> DBMessageParseResult<'a> {
        DBMessage::parse(&[
            0x11, 0x87, 0x23, 0x49, 0xae, 0x11, 0x05, 0x80,
            0x00, 0x32, 0x10, 0x10, 0x00, 0x0f, 0x03, 0x14,
            0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x06, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x02, 0x01, 0x04, 0x01, 0x11, 0x00, 0x00,
            0x00, 0x00, 0x11, 0x00, 0xff, 0xff, 0xff,
        ])
    }

    pub fn root_menu_response_packet() -> Bytes {
        Bytes::from(vec![
            0x11, 0x87, 0x23, 0x49, 0xae, 0x11, 0x05, 0x80,
            0x00, 0x32, 0x10, 0x40, 0x00, 0x0f, 0x02, 0x14,
            0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x10, 0x00, 0x11, 0x00, 0x00,
            0x00, 0x08,
        ])
    }

    pub fn artist_request_type<'a>() -> DBMessageParse<'a> {
        let request = vec![
            0x11, 0x87, 0x23, 0x49, 0xae,
            0x11, 0x05, 0x80, 0x00, 0x1c,
            0x10, 0x21, 0x02,
            0x0f, 0x02, 0x14, 0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x02, 0x08, 0x04, 0x01,
            0x11, 0x00, 0x00, 0x00, 0x05,
        ];

        DBMessage::parse(&[
            0x11, 0x87, 0x23, 0x49, 0xae, 0x11, 0x05, 0x80,
            0x00, 0x10, 0x10, 0x10, 0x02, 0x0f, 0x02, 0x14,
            0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x02, 0x02, 0x04, 0x01,
            0x11, 0x00, 0x00, 0x00, 0x00
        ]).unwrap().1
    }

    pub fn render_root_menu_request<'a>() -> Vec<u8> {
        vec![
            0x11, 0x87, 0x23, 0x49, 0xae,
            0x11, 0x05, 0x80, 0x00, 0x0f,
            0x10, 0x30, 0x00,
            0x0f, 0x06, 0x14, 0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x02, 0x01, 0x04, 0x01,
            0x11, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x07,
            0x11, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x08,
            0x11, 0x00, 0x00, 0x00, 0x00,
        ]
    }

    pub fn artist_request() -> Bytes {
        Bytes::from(vec![
            0x11, 0x87, 0x23, 0x49, 0xae,
            0x11, 0x05, 0x80, 0x00, 0x11,
            0x10, 0x30, 0x00,
            0x0f, 0x06, 0x14, 0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x02, 0x02, 0x04, 0x01,
            0x11, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x01,
            0x11, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x01,
            0x11, 0x00, 0x00, 0x00, 0x00,
        ])
    }

    pub fn render_artist_request() -> Vec<u8> {
        vec![
            0x11, 0x87, 0x23, 0x49, 0xae, 0x11, 0x05, 0x80,
            0x00, 0x41, 0x10, 0x30, 0x00, 0x0f, 0x06, 0x14,
            0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x06, 0x06,
            0x06, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x02, 0x02, 0x04, 0x01, 0x11, 0x00, 0x00,
            0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x01, 0x11,
            0x00, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00,
            0x01, 0x11, 0x00, 0x00, 0x00, 0x00,
        ]
    }

    pub fn raw_menu_footer_request() -> Bytes {
        Bytes::from(vec![
            0x11, 0x87, 0x23, 0x49, 0xae, 0x11, 0x05, 0x80, 0x00, 0x0f, 0x10, 0x42, 0x01, 0x0f,
            0x00, 0x14, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ])
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
            Ok((
                &[0x20][..],
                DBField::u32(&[0x00, 0x00, 0x00, 0x01]),
            )),
            DBMessage::transaction_id(&[DBFieldType::U32.value(), 0x00, 0x00, 0x00, 0x01, 0x20]),
        );
        assert_eq!(
            Ok((
                &[0x20][..],
                DBField::new(DBFieldType::U32, &[0x00, 0x00, 0x01, 0x00]),
            )),
            DBMessage::transaction_id(&[DBFieldType::U32.value(), 0x00, 0x00, 0x01, 0x00, 0x20]),
        );
        assert_eq!(
            Ok((
                &[0x20, 0x21][..],
                DBField::new(DBFieldType::U32, &[0x05, 0x80, 0x00, 0x32]),
            )),
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

        assert_eq!(
            DBRequestType::ArtistRequest,
            fixtures::artist_request_type().request_type,
        )
    }

    #[test]
    fn parse_argument_count() {
        assert_eq!(
            Ok((&[][..], 3_u8)),
            DBMessage::argument_count(&[DBFieldType::U8 as u8, 0x03]),
        );
    }

    #[test]
    fn dbmessage_setup_request_and_response() {
        let dbmessage = fixtures::setup_request_packet().unwrap().1;
        assert_eq!(DBRequestType::Setup, dbmessage.request_type);
    }

    #[test]
    fn parse_test_library_handler() {
        assert_eq!(
            Err(nom::Err::Error((&[0, 0, 0, 1][..], nom::error::ErrorKind::Tag))),
            DBMessage::parse(b"\x11\x00\x00\x00\x01"),
        );
    }

    #[test]
    fn parse_dbmessage() {
        assert_eq!(
            Ok((&[][..], DBMessage {
                transaction_id: DBField::u32(&[0x05, 0x80, 0x00, 0x32]),
                request_type: DBRequestType::RootMenuRequest,
                argument_count: 3_u8,
                arg_types: &[
                    0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x06, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ],
                args: vec![
                    DBField::u32(&[0x02, 0x01, 0x04, 0x01]),
                    DBField::u32(&[0x00, 0x00, 0x00, 0x00]),
                    DBField::u32(&[0x00, 0xff, 0xff, 0xff]),
                ],
            })),
            fixtures::root_menu_request(),
        );
    }

    #[test]
    fn try_parse_dbmessage_with_broken_magic() {
        let message = [
            0x49, 0xae, 0x11, 0x05, 0x80,
            0x00, 0x32, 0x10, 0x10, 0x00, 0x0f, 0x03, 0x14,
            0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x06, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x02, 0x01, 0x04, 0x01, 0x11, 0x00, 0x00,
            0x00, 0x00, 0x11, 0x00, 0xff, 0xff, 0xff,
        ];

        // First byte is consumed so skip that when asserting
        assert_eq!(
            Err(nom::Err::Error((&message[1..], nom::error::ErrorKind::Tag))),
            DBMessage::parse(&message),
        );
    }

    #[test]
    fn encode_db_field_to_bytes() {
        assert_eq!(b"\x0f\x32", &DBField::u8(&[0x32]).as_bytes()[..]);
        assert_eq!(b"\x10\x12\x13", &DBField::u16(&[0x12, 0x13]).as_bytes()[..]);
        assert_eq!(
            b"\x11\x00\x00\x00\x01",
            &DBField::u32(&[0x00, 0x00, 0x00, 0x01]).as_bytes()[..],
        );
    }

    #[test]
    fn build_root_menu_render_request_package() {}

    #[cfg(test)]
    mod db_message_parsing {
        use super::*;

        #[test]
        fn construct_menu_footer() {
            assert_eq!(
                DBMessage::parse(&fixtures::raw_menu_footer_request()).unwrap().1,
                DBMessage::new(
                    DBField::u32(&[0x05, 0x80, 0x00, 0x0f]),
                    DBRequestType::MenuFooter,
                    0x00,
                    &[0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                    vec![],
                ),
            );
        }

        #[test]
        fn menu_footer_to_bytes() {
            assert_eq!(
                fixtures::raw_menu_footer_request(),
                Bytes::from(DBMessage::parse(&fixtures::raw_menu_footer_request()).unwrap().1),
            );
        }
    }

    #[cfg(test)]
    mod argument_types {
        use super::*;

        #[test]
        fn string_artist_argument() {
            assert_eq!(DBField::new(DBFieldType::String, &[
                0x00, 0x00, 0x00, 0x09, 0xff, 0xfa, 0x00, 0x41,
                0x00, 0x52, 0x00, 0x54, 0x00, 0x49, 0x00, 0x53,
                0x00, 0x54, 0xff, 0xfb, 0x00, 0x00,
            ]), DBField::string("ARTIST", true));
        }

        #[test]
        fn string_history_argument() {
            assert_eq!(DBField::new(DBFieldType::String, &[
                0x00, 0x00, 0x00, 0x0a, 0xff, 0xfa, 0x00, 0x48, 0x00, 0x49, 0x00, 0x53,
                0x00, 0x54, 0x00, 0x4f, 0x00, 0x52, 0x00, 0x59, 0xff, 0xfb, 0x00, 0x00,
            ]), DBField::string("HISTORY", true));
        }

        #[test]
        fn string_track_argument() {
            assert_eq!(DBField::new(DBFieldType::String, &[
                0x00, 0x00, 0x00, 0x08, 0xff, 0xfa, 0x00, 0x54,
                0x00, 0x52, 0x00, 0x41, 0x00, 0x43, 0x00, 0x4b,
                0xff, 0xfb, 0x00, 0x00,
            ]), DBField::string("TRACK", true));
        }

        #[test]
        fn string_key_argument() {
            assert_eq!(DBField::new(DBFieldType::String, &[
                0x00, 0x00, 0x00, 0x06, 0xff, 0xfa, 0x00, 0x4b,
                0x00, 0x45, 0x00, 0x59, 0xff, 0xfb, 0x00, 0x00,
            ]), DBField::string("KEY", true));
        }

        #[test]
        fn empty_string_argument() {
            assert_eq!(DBField::new(DBFieldType::String, &[
                0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            ]), DBField::string("", false));
        }

        #[test]
        fn unwrapped_string() {
            assert_eq!(DBField::new(DBFieldType::String, &[
                0x00, 0x00, 0x00, 0x0c, 0x00, 0x4c, 0x00, 0x6f, 0x00, 0x6f, 0x00, 0x70,
                0x00, 0x6d, 0x00, 0x61, 0x00, 0x73, 0x00, 0x74, 0x00, 0x65, 0x00, 0x72,
                0x00, 0x73, 0x00, 0x00,
            ]), DBField::string("Loopmasters", false));
        }
    }
}
