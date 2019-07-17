extern crate tokio;

use std::net::SocketAddr;

use tokio::prelude::*;
use tokio::net::{UdpFramed, UdpSocket};
use tokio::codec::BytesCodec;
use bytes::Bytes;
use std::convert::TryFrom;

use super::packets::RpcMessage;

pub struct RPCServer {
    socket_addr: SocketAddr,
}

fn process(data: Bytes) {
    match RpcMessage::try_from(data) {
        Ok(message) => eprintln!("{:?}", message),
        Err(err) => eprintln!("{:?}", err),
    };
}

impl RPCServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            socket_addr: addr,
        }
    }

    pub fn run(&self) -> Result<(), Box<std::error::Error>> {
        let socket = UdpSocket::bind(&self.socket_addr)?;

        let framed = UdpFramed::new(socket, BytesCodec::new());
        let (writer, reader) = framed.split();

        let processor = reader.for_each(|(msg, addr)| {
            process(Bytes::from(msg));
            Ok(())
        }).map_err(|e| eprintln!("{:?}", e));

        tokio::run(processor);
        Ok(())
    }
}
