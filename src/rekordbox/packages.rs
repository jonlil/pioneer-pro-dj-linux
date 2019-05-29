extern crate nom;

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
struct DBMessage;

type DBMessageResult<'a> = IResult<&'a [u8], &'a [u8]>;
type DBMessageU32<'a> = IResult<&'a [u8], u32>;
type DBMessageU16<'a, T> = IResult<&'a [u8], T>;

impl DBMessage {
    fn magic(i: &[u8]) -> DBMessageResult {
        tag([0x87, 0x23, 0x49, 0xae])(i)
    }

    fn transaction_id(i: &[u8]) -> DBMessageU32 {
        let (i, _) = take(1u8)(i)?;
        let (i, transaction) = be_u32(i)?;

        Ok((i, transaction))
    }

    fn request_type(i: &[u8]) -> DBMessageU16<DBRequestType> {
        let (i, _) = take(1u8)(i)?;

        let request_type: DBMessageU16<u16> = be_u16(i);
        match request_type {
            Ok((i, 0_u16)) => Ok((i, DBRequestType::Setup)),
            Ok((i, data)) => Ok((i, DBRequestType::Unknown(data))),
            Err(err) => Err(err),
        }
    }
    fn argument_count() {}
    fn arg_types() {}
    fn args() {}

    pub fn parse(i: &[u8]) -> Result<DBMessage, Error> {
        Ok(DBMessage {})
    }
}

#[derive(Debug, PartialEq)]
enum DBRequestType {
    Setup,
    Unknown(u16),
}

#[cfg(test)]
mod test {
    use super::{DBMessage, DBFieldType, DBRequestType};

    #[test]
    fn extract_magic_from_db_message() {
        assert_eq!(
            Ok((&[0x11][..], &[135, 35, 73, 174][..])),
            DBMessage::magic(&[0x87, 0x23, 0x49, 0xae, 0x11]),
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

        /// Verify parsing only 3 bytes (size identifier + u16)
        assert_eq!(
            Ok((&[0x00][..], DBRequestType::Setup)),
            DBMessage::request_type(&[DBFieldType::U16 as u8, 0x00, 0x00, 0x00]),
        );

        /// Verify matching unknown packages (and that data is kept for debug
        assert_eq!(
            Ok((&[][..], DBRequestType::Unknown(255_u16))),
            DBMessage::request_type(&[DBFieldType::U16 as u8, 0x00, 0xff]),
        );
    }

    #[test]
    fn parse_db_field_type() {
    }
}
