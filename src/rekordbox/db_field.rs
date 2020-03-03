use bytes::{Bytes, BytesMut};

pub struct Binary {
    value: Bytes,
}

impl Binary {
    pub fn new(value: Bytes) -> Binary {
        Binary {
            value,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct DBField {
    pub kind: DBFieldType,
    pub value: Bytes,
}

impl From<Binary> for DBField {
    fn from(value: Binary) -> DBField {
        DBField::new(DBFieldType::Binary, &value.value)
    }
}

impl From<u32> for DBField {
    fn from(value: u32) -> DBField {
        DBField::new(DBFieldType::U32, &value.to_be_bytes())
    }
}

impl From<u16> for DBField {
    fn from(value: u16) -> DBField {
        DBField::new(DBFieldType::U16, &value.to_be_bytes())
    }
}

impl From<u8> for DBField {
    fn from(value: u8) -> DBField {
        DBField::new(DBFieldType::U8, &[value])
    }
}

impl From<&str> for DBField {
    fn from(value: &str) -> DBField {
        let mut bytes = BytesMut::new();

        if value.len() > 0 {
            bytes.extend(value.encode_utf16()
                .into_iter()
                .flat_map(|item| { item.to_be_bytes().to_vec() })
                .collect::<Bytes>());
        }

        DBField {
            kind: DBFieldType::String,
            value: Bytes::from(bytes),
        }
    }
}

impl From<[u8; 4]> for DBField {
    fn from(value: [u8; 4]) -> DBField {
        DBField::new(DBFieldType::U32, &value)
    }
}

impl From<[u8; 2]> for DBField {
    fn from(value: [u8; 2]) -> DBField {
        DBField::new(DBFieldType::U16, &value)
    }
}

impl DBField {
    pub fn new(kind: DBFieldType, value: &[u8]) -> Self {
        Self {
            kind,
            value: Bytes::from(value.to_vec()),
        }
    }

    pub fn as_bytes(&self) -> Bytes {
        let mut buffer = BytesMut::new();

        match self.kind {
            DBFieldType::String => {
                buffer.extend(vec![self.kind.value()]);
                buffer.extend(vec![0x00, 0x00, 0x00, self.value.len() as u8 / 2+1]);
                buffer.extend(self.value.to_vec());
                buffer.extend(vec![0x00, 0x00]);
            },
            DBFieldType::Binary => {
                if self.value.len() > 0 {
                    buffer.extend(vec![self.kind.value()]);
                    buffer.extend((self.value.len() as u32).to_be_bytes().to_vec());
                    buffer.extend(self.value.to_vec());
                }
            },
            _ => {
                buffer.extend(vec![self.kind.value()]);
                buffer.extend(self.value.to_vec());
            }
        };

        buffer.freeze()
    }
}

impl From<DBField> for Bytes {
    fn from(field: DBField) -> Self {
        field.as_bytes()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum DBFieldType {
  U8,
  U16,
  U32,
  String,
  Binary,
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn string_decode_and_encode() {
        let field = DBField::from("Loopmasters");
        assert_eq!(DBField::new(DBFieldType::String, &[
            0x00, 0x4c, 0x00, 0x6f, 0x00, 0x6f, 0x00, 0x70,
            0x00, 0x6d, 0x00, 0x61, 0x00, 0x73, 0x00, 0x74,
            0x00, 0x65, 0x00, 0x72, 0x00, 0x73
        ]), field);
        assert_eq!(Bytes::from(vec![
            0x26, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x4c, 0x00, 0x6f, 0x00, 0x6f, 0x00, 0x70,
            0x00, 0x6d, 0x00, 0x61, 0x00, 0x73, 0x00, 0x74, 0x00, 0x65, 0x00, 0x72,
            0x00, 0x73, 0x00, 0x00,
        ]), field.as_bytes());
    }

    #[test]
    fn string_artist_argument() {
        let field = DBField::from("\u{fffa}ARTIST\u{fffb}");
        let expected_bytes = &[
            0xff, 0xfa, 0x00, 0x41, 0x00, 0x52, 0x00, 0x54, 0x00, 0x49, 0x00, 0x53, 0x00, 0x54, 0xff, 0xfb,
        ];

        assert_eq!(DBField::new(DBFieldType::String, expected_bytes), field);
        assert_eq!(vec![
            0x26, 0x00, 0x00, 0x00, 0x09, 0xff, 0xfa, 0x00, 0x41,
            0x00, 0x52, 0x00, 0x54, 0x00, 0x49, 0x00, 0x53,
            0x00, 0x54, 0xff, 0xfb, 0x00, 0x00,
        ], field.as_bytes());
    }

    #[test]
    fn string_history_argument() {
        let field = DBField::from("\u{fffa}HISTORY\u{fffb}");
        let expected_bytes = &[
            0xff, 0xfa, 0x00, 0x48, 0x00, 0x49, 0x00, 0x53, 0x00, 0x54, 0x00, 0x4f,
            0x00, 0x52, 0x00, 0x59, 0xff, 0xfb,
        ];

        assert_eq!(DBField::new(DBFieldType::String, expected_bytes), field);
        assert_eq!(vec![
            0x26, 0x00, 0x00, 0x00, 0x0a, 0xff, 0xfa, 0x00, 0x48, 0x00, 0x49, 0x00, 0x53,
            0x00, 0x54, 0x00, 0x4f, 0x00, 0x52, 0x00, 0x59, 0xff, 0xfb, 0x00, 0x00,
        ], field.as_bytes());
    }

    #[test]
    fn empty_string_argument() {
        let field = DBField::from("");
        assert_eq!(DBField::new(DBFieldType::String, &[]), field);
        assert_eq!(vec![0x26, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00], field.as_bytes());
    }

    #[test]
    fn binary_to_dbfield() {
        let fixture = Bytes::from(vec![
            0x38, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xe8, 0x03,
            0x9b, 0x2a, 0x01, 0x00, 0xff, 0xff, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ]);

        let field = DBField::from(Binary {
            value: fixture.clone(),
        });

        let mut expected_value = BytesMut::new();
        expected_value.extend(vec![0x14, 0x00, 0x00, 0x00, 0x38]);
        expected_value.extend(fixture);
        assert_eq!(
            expected_value,
            field.as_bytes(),
        );
    }
}
