use bytes::{Bytes, BytesMut};
use nom::IResult;
use nom::bytes::complete::{take};
use nom::number::complete::{be_u32, be_u16};
use super::db_field::{DBField, DBFieldType};
use std::ops::Index;

#[derive(Debug, PartialEq, Clone)]
enum ArgumentType {
    U32,
    U16,
    U8,
    String,
    Binary,
}

impl ArgumentType {
    fn new(value: u8) -> ArgumentType {
        match value {
            0x04 => ArgumentType::U8,
            0x05 => ArgumentType::U16,
            0x06 => ArgumentType::U32,
            0x02 => ArgumentType::String,
            0x03 => ArgumentType::Binary,
            _ => panic!("Non-supported argument type."),
        }
    }

    fn value(&self) -> u8 {
        match *self {
            ArgumentType::U8 => 0x04,
            ArgumentType::U16 => 0x05,
            ArgumentType::U32 => 0x06,
            ArgumentType::String => 0x02,
            ArgumentType::Binary => 0x03,
        }
    }
}

impl From<ArgumentType> for DBFieldType {
    fn from(argument_type: ArgumentType) -> DBFieldType {
        match argument_type {
            ArgumentType::U32 => DBFieldType::U32,
            ArgumentType::U16 => DBFieldType::U16,
            ArgumentType::U8 => DBFieldType::U8,
            ArgumentType::String => DBFieldType::String,
            ArgumentType::Binary => DBFieldType::Binary,
        }
    }
}

impl From<DBFieldType> for ArgumentType {
    fn from(field_type: DBFieldType) -> ArgumentType {
        match field_type {
            DBFieldType::U32 => ArgumentType::U32,
            DBFieldType::U16 => ArgumentType::U16,
            DBFieldType::U8 => ArgumentType::U8,
            DBFieldType::String => ArgumentType::String,
            DBFieldType::Binary => ArgumentType::Binary,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ArgumentCollection(Vec<DBField>);

impl IntoIterator for ArgumentCollection {
    type Item = DBField;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Index<usize> for ArgumentCollection {
    type Output = DBField;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl ArgumentCollection {
    pub fn new(items: Vec<DBField>) -> ArgumentCollection {
        ArgumentCollection(items)
    }

    pub fn iter(&self) -> std::slice::Iter<DBField> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn decode(input: &[u8]) -> IResult<&[u8], ArgumentCollection> {
        let (input, argument_count) = take(2u8)(input)?;
        let (input, _) = take(5u8)(input)?;
        let (mut input, argument_types) = take(12u8)(input)?;

        let items = (0x00 .. argument_count[1]).into_iter().map(|counter| {
            let argument_type = ArgumentType::new(argument_types[counter as usize]);
            let args = Argument::decode(argument_type, &input).unwrap();
            input = args.0;
            args.1
        }).collect::<Vec<DBField>>();

        Ok((&[][..], ArgumentCollection(items)))
    }
}

impl From<ArgumentCollection> for Bytes {
    fn from(collection: ArgumentCollection) -> Self {
        let mut buffer = BytesMut::new();
        let arg_count = collection.len() as u8;

        buffer.extend(vec![0x0f, arg_count, 0x14, 0x00, 0x00, 0x00, 0x0c]);
        buffer.extend(&collection.iter().map(|item| {
            ArgumentType::from(item.kind.clone()).value()
        }).collect::<Vec<u8>>());

        if arg_count < 12 {
            buffer.extend(Bytes::from(vec![0x00; 12 - arg_count as usize]));
        }

        for item in collection.into_iter() {
            buffer.extend(item.as_bytes());
        }

        Bytes::from(buffer)
    }
}


#[derive(Debug, PartialEq)]
pub struct Argument {
    kind: ArgumentType,
    value: Bytes,
}

impl Decode for Argument {
    fn decode(kind: ArgumentType, input: &[u8]) -> IResult<&[u8], DBField> {
        match kind {
            ArgumentType::String => {
                let (input, _) = take(1u8)(input)?;
                let (input, variable_size) = be_u32(input)?;
                let (input, data) = take((variable_size - 1) * 2)(input)?;
                let (input, _) = be_u16(input)?;

                Ok((input, DBField::new(DBFieldType::String, data)))
            },
            ArgumentType::U8 => {
                let (input, _) = take(1u8)(input)?;
                let (input, data) = take(1u8)(input)?;
                Ok((input, DBField::new(DBFieldType::U8, data)))
            },
            ArgumentType::U16 => {
                let (input, _) = take(1u8)(input)?;
                let (input, data) = take(2u8)(input)?;
                Ok((input, DBField::new(DBFieldType::U16, data)))
            },
            ArgumentType::U32 => {
                let (input, _) = take(1u8)(input)?;
                let (input, data) = take(4u8)(input)?;
                Ok((input, DBField::new(DBFieldType::U32, data)))
            },
            ArgumentType::Binary => {
                if input.len() < 4 {
                    return Ok((input, DBField::new(DBFieldType::Binary, &[])));
                }

                let (input, variable_size) = be_u32(input)?;
                let (input, data) = take(variable_size)(input)?;
                Ok((input, DBField::new(DBFieldType::Binary, data)))
            },
        }
    }
}

impl Encode for Argument {
    fn encode(&self) -> Bytes {
        let mut buffer = BytesMut::new();

        match self.kind {
            ArgumentType::String => {
                buffer.extend(vec![
                    DBFieldType::from(self.kind.clone()).value(),
                    0x00,
                    0x00,
                    0x00,
                    self.value.len() as u8 / 2+1,
                ]);
                buffer.extend(&*self.value);
                buffer.extend(vec![0x00, 0x00]);
            },
            _ => {
                buffer.extend(vec![DBFieldType::from(self.kind.clone()).value()]);
                buffer.extend(&*self.value);
            }
        }

        Bytes::from(buffer)
    }
}

trait Decode {
    fn decode(kind: ArgumentType, value: &[u8]) -> IResult<&[u8], DBField>;
}

trait Encode {
    fn encode(&self) -> Bytes;
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::{assert_eq};

    const PARTIAL_RAW_MESSAGE: &[u8] = &[
        0x0f, 0x07, 0x14, 0x00, 0x00, 0x00, 0x0c,
        0x06, 0x05, 0x04, 0x06, 0x06, 0x06, 0x02,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x11, 0x02,
        0x02, 0x04, 0x01, 0x10, 0x00, 0x00, 0x0f,
        0x00, 0x11, 0x00, 0x00, 0x00, 0x01, 0x11,
        0x00, 0x00, 0x00, 0x02, 0x11, 0x00, 0x00,
        0x00, 0x03, 0x26, 0x00, 0x00, 0x00, 0x08,
        0x00, 0x55, 0x00, 0x6e, 0x00, 0x6b, 0x00,
        0x6e, 0x00, 0x6f, 0x00, 0x77, 0x00, 0x6e,
        0x00, 0x00,
    ];

    #[test]
    fn test_decoding_string_argument() {
        assert_eq!(
            Ok((&[][..], DBField {
                kind: DBFieldType::String,
                value: Bytes::from(vec![
                    0x00, 0x55, 0x00, 0x6e, 0x00, 0x6b, 0x00,
                    0x6e, 0x00, 0x6f, 0x00, 0x77, 0x00, 0x6e,
                ])
            })),
            Argument::decode(ArgumentType::String, &[
                0x26, 0x00, 0x00, 0x00, 0x08, 0x00, 0x55, 0x00, 0x6e, 0x00, 0x6b, 0x00,
                0x6e, 0x00, 0x6f, 0x00, 0x77, 0x00, 0x6e, 0x00, 0x00,
            ]),
        )
    }

    #[test]
    fn test_decoding_from_raw_data_to_argument_collection() {
        assert_eq!(
            Ok((&[][..], ArgumentCollection(vec![
                DBField { kind: DBFieldType::U32, value: Bytes::from("\x02\x02\x04\x01") },
                DBField { kind: DBFieldType::U16, value: Bytes::from("\x00\x00") },
                DBField { kind: DBFieldType::U8, value: Bytes::from("\x00") },
                DBField { kind: DBFieldType::U32, value: Bytes::from("\x00\x00\x00\x01") },
                DBField { kind: DBFieldType::U32, value: Bytes::from("\x00\x00\x00\x02") },
                DBField { kind: DBFieldType::U32, value: Bytes::from("\x00\x00\x00\x03") },
                DBField {
                    kind: DBFieldType::String,
                    value: Bytes::from(vec![
                        0x00, 0x55, 0x00, 0x6e, 0x00, 0x6b, 0x00,
                        0x6e, 0x00, 0x6f, 0x00, 0x77, 0x00, 0x6e,
                    ]),
                },
            ]))),
            ArgumentCollection::decode(PARTIAL_RAW_MESSAGE)
        );
    }

    #[test]
    fn test_encode_from_argument_collection_to_raw_bytes() {
        let arguments = ArgumentCollection(vec![
            DBField { kind: DBFieldType::U32, value: Bytes::from("\x02\x02\x04\x01") },
            DBField { kind: DBFieldType::U16, value: Bytes::from("\x00\x00") },
            DBField { kind: DBFieldType::U8, value: Bytes::from("\x00") },
            DBField { kind: DBFieldType::U32, value: Bytes::from("\x00\x00\x00\x01") },
            DBField { kind: DBFieldType::U32, value: Bytes::from("\x00\x00\x00\x02") },
            DBField { kind: DBFieldType::U32, value: Bytes::from("\x00\x00\x00\x03") },
            DBField {
                kind: DBFieldType::String,
                value: Bytes::from(vec![
                    0x00, 0x55, 0x00, 0x6e, 0x00, 0x6b, 0x00,
                    0x6e, 0x00, 0x6f, 0x00, 0x77, 0x00, 0x6e,
                ]),
            },
        ]);

        assert_eq!(
            Bytes::from(PARTIAL_RAW_MESSAGE),
            Bytes::from(arguments),
        );
    }
}
