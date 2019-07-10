use nom::bytes::complete::{tag, take};
use nom::number::complete::{be_u16};
use nom::IResult;
use bytes::{Bytes, BytesMut};
use super::db_field::{DBField, DBFieldType, Binary};
use super::db_request_type::DBRequestType;
use super::db_message_argument::ArgumentCollection;

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

type DBMessageResult<'a> = IResult<&'a [u8], &'a [u8]>;
type DBMessageU32<'a> = IResult<&'a [u8], u32>;
pub type DBMessageResultType<'a, T> = IResult<&'a [u8], T>;

impl DBMessage {
    const MAGIC: [u8; 4] = [0x87, 0x23, 0x49, 0xae];

    pub fn new(
        transaction_id: DBField,
        request_type: DBRequestType,
        arguments: ArgumentCollection
    ) -> DBMessage {
        DBMessage {
            transaction_id,
            request_type,
            arguments,
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
                &Bytes::from(transaction),
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

#[cfg(test)]
mod test {
    use super::*;
    use super::super::fixtures;

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
            Err(nom::Err::Error((&[0, 0, 0, 1][..], nom::error::ErrorKind::Tag))),
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
            fixtures::root_menu_request(),
        );
    }

    #[test]
    fn try_parse_dbmessage_with_broken_magic() {
        let message = [0x49, 0xae, 0x11, 0x05, 0x80];

        // First byte is consumed so skip that when asserting
        assert_eq!(
            Err(nom::Err::Error((&message[1..], nom::error::ErrorKind::Tag))),
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

    #[cfg(test)]
    mod db_message_parsing {
        use super::*;

        #[test]
        fn construct_menu_footer() {
            assert_eq!(
                DBMessage::parse(&fixtures::raw_menu_footer_request()).unwrap().1,
                DBMessage::new(
                    DBField::from([0x05, 0x80, 0x00, 0x0f]),
                    DBRequestType::MenuFooter,
                    ArgumentCollection::new(vec![]),
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
                        DBField::from(Binary::new(Bytes::from(&binary_data[4..]))),
                    ]),
                ))),
                DBMessage::parse(&message),
            );
        }
    }
}
