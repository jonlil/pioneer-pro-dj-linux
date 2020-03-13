use bytes::{Bytes, BytesMut, BufMut};
use tokio_util::codec::{Encoder, Decoder};
use super::packets::RpcMessage;
use std::io::{Error, ErrorKind};
use std::convert::TryFrom;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RpcBytesCodec(());

impl RpcBytesCodec {
    /// Creates a new `RpcBytesCodec` for shipping `RpcMessage` back and forth
    pub fn new() -> RpcBytesCodec { RpcBytesCodec(()) }
}

impl Decoder for RpcBytesCodec {
    type Item = RpcMessage;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buf.len() > 0 {
            let len = buf.len();

            match RpcMessage::try_from(Bytes::from(buf.split_to(len))) {
                Ok(message) => Ok(Some(message)),
                Err(err) => Err(Error::new(ErrorKind::InvalidInput, err)),
            }
        } else {
            Ok(None)
        }
    }
}

fn calculate_fill_bytes(length: usize) -> Vec<u8> {
    let modulo = length % 4;
    if modulo != 0 {
        vec![0x00; (4 - modulo) as usize]
    } else {
        vec![]
    }
}

impl Encoder<RpcMessage> for RpcBytesCodec {
    type Error = Error;

    fn encode(&mut self, data: RpcMessage, buf: &mut BytesMut) -> Result<(), Error> {
        match Bytes::try_from(data) {
            Ok(data) => {
                let fill_bytes = calculate_fill_bytes(data.len());
                buf.reserve(data.len());
                buf.put(data);
                buf.extend(&fill_bytes);
                Ok(())
            },
            Err(_err) => Err(Error::new(
                ErrorKind::InvalidInput,
                "Failed encoding RpcMessage",
            )),
        }
    }
}
