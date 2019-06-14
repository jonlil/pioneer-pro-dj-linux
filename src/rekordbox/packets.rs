use nom::multi::count;
use nom::bytes::complete::{tag, take};
use nom::number::complete::{be_u32, be_u16, be_u8};
use nom::IResult;

#[derive(Debug, PartialEq)]
enum Error {
    ParseError,
}

#[derive(Debug, PartialEq)]
enum DBFieldType {
  U8 = 0x0f,
  U16 = 0x10,
  U32 = 0x11,
  Binary = 0x14,
  String = 0x26,
  Unknown,
}

#[derive(Debug, PartialEq)]
pub struct DBField<'a> {
    kind: DBFieldType,
    value: &'a [u8],
}


#[derive(Debug, PartialEq)]
pub struct DBMessage<'a> {
    pub transaction_id: u32,
    pub request_type: DBRequestType,
    pub argument_count: u8,
    pub arg_types: &'a [u8],
    pub args: Vec<&'a [u8]>,
}

type DBMessageResult<'a> = IResult<&'a [u8], &'a [u8]>;
type DBMessageU32<'a> = IResult<&'a [u8], u32>;
type DBMessageResultType<'a, T> = IResult<&'a [u8], T>;

impl<'a> DBMessage<'a> {
    fn magic(i: &[u8]) -> DBMessageResult {
        let (i, _) = take(1u8)(i)?;
        tag([0x87, 0x23, 0x49, 0xae])(i)
    }

    fn transaction_id(i: &[u8]) -> DBMessageU32 {
        let (i, _) = take(1u8)(i)?;
        let (i, transaction) = be_u32(i)?;

        Ok((i, transaction))
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
            Ok((input, data))     => {
                eprintln!("{:?}", input);
                Ok((input, DBRequestType::Unknown(data)))
            },
            Err(err)              => Err(err),
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
        fn parse_arguments(input: &[u8]) -> IResult<&[u8], &[u8]> {
            match be_u8(input) {
                Err(err) => Err(err),
                Ok((input, 0x11)) => take(4u8)(input),
                Ok((input, _consumed)) => Ok((input, &[])),
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
}

#[derive(Debug, PartialEq)]
pub enum DBRequestType {
    Setup,
    RenderRequest,
    RootMenuRequest,
    GenreRequest,
    ArtistRequest,
    AlbumRequest,
    TitleRequest,
    KeyRequest,
    PlaylistRequest,
    SearchQueryRequest,
    HistoryRequest,
    Unknown(u16),
}

#[cfg(test)]
mod test {
    use super::{DBMessage, DBFieldType, DBRequestType};

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
            Ok((&[0x20][..], 1_u32)),
            DBMessage::transaction_id(&[DBFieldType::U32 as u8, 0x00, 0x00, 0x00, 0x01, 0x20]),
        );
        assert_eq!(
            Ok((&[0x20][..], 256_u32)),
            DBMessage::transaction_id(&[DBFieldType::U32 as u8, 0x00, 0x00, 0x01, 0x00, 0x20]),
        );
        assert_eq!(
            Ok((&[0x20][..], 92_274_738_u32)),
            DBMessage::transaction_id(&[DBFieldType::U32 as u8, 0x05, 0x80, 0x00, 0x32, 0x20]),
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
    fn parse_argument_count() {
        assert_eq!(
            Ok((&[][..], 3_u8)),
            DBMessage::argument_count(&[DBFieldType::U8 as u8, 0x03]),
        );
    }

    #[test]
    fn parse_dbmessage_SetupRequest() {
        let message = [
            0x11,0x87,0x23,0x49,0xae,0x11,0xff,0xff,
            0xff,0xfe,0x10,0x00,0x00,0x0f,0x01,0x14,
            0x00,0x00,0x00,0x0c,0x06,0x00,0x00,0x00,
            0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x02,
        ];
        let parsed_message = DBMessage::parse(&message);
        assert_eq!(DBRequestType::Setup, parsed_message.unwrap().1.request_type);
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
        let message = [
            0x11, 0x87, 0x23, 0x49, 0xae, 0x11, 0x05, 0x80,
            0x00, 0x32, 0x10, 0x10, 0x00, 0x0f, 0x03, 0x14,
            0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x06, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x02, 0x01, 0x04, 0x01, 0x11, 0x00, 0x00,
            0x00, 0x00, 0x11, 0x00, 0xff, 0xff, 0xff,
        ];

        assert_eq!(
            Ok((&[][..], DBMessage {
                transaction_id: 92274738_u32,
                request_type: DBRequestType::RootMenuRequest,
                argument_count: 3_u8,
                arg_types: &[
                    0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x06, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ],
                args: vec![
                    &[0x02, 0x01, 0x04, 0x01],
                    &[0x00, 0x00, 0x00, 0x00],
                    &[0x00, 0xff, 0xff, 0xff],
                ],
            })),
            DBMessage::parse(&message),
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
    fn parse_db_field_type() {
    }
}
