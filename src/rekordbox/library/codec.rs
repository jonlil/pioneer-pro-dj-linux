use crate::rekordbox::packets::DBMessage;
use bytes::Bytes;
use std::convert::TryFrom;
use std::io::{Error, ErrorKind};
use tokio_util::codec::{Decoder, Encoder};

use bytes::BytesMut;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DbBytesCodec(());

impl DbBytesCodec {
    /// Creates a new `RpcBytesCodec` for shipping `RpcMessage` back and forth
    pub fn new() -> DbBytesCodec {
        DbBytesCodec(())
    }
}

impl Decoder for DbBytesCodec {
    type Item = DBMessage;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buf.len() > 0 {
            let len = buf.len();

            match DBMessage::try_from(Bytes::from(buf.split_to(len))) {
                Ok(message) => Ok(Some(message)),
                Err(err) => Err(Error::new(ErrorKind::InvalidInput, err)),
            }
        } else {
            Ok(None)
        }
    }
}

impl Encoder<DBMessage> for DbBytesCodec {
    type Error = Error;

    fn encode(&mut self, data: DBMessage, buf: &mut BytesMut) -> Result<(), Error> {
        buf.extend(Bytes::from(data));

        Ok(())
    }
}
