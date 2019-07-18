use std::net::{SocketAddr, IpAddr, Ipv4Addr};

use tokio::prelude::*;
use tokio::net::{UdpFramed, UdpSocket};
use bytes::Bytes;
use std::convert::TryFrom;
use rand::Rng;
use std::io::{Read, Write, self};
use futures::{Future, Async, Poll};

use super::packets::{RpcMessage, RpcMessageType};
use super::codec::RpcBytesCodec;

pub struct RpcServer {
    socket_addr: SocketAddr,
    clients: Vec<()>,
}

/// This is the Portmap server
impl RpcServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            socket_addr: addr,
            clients: vec![],
        }
    }

    pub fn run(&self) -> Result<(), Box<std::error::Error>> {
        let socket = UdpSocket::bind(&self.socket_addr)?;
        let framed = UdpFramed::new(socket, RpcBytesCodec::new());
        let (_writer, reader) = framed.split();

        let processor = reader.for_each(|(rpc_msg, _addr)| {
            match rpc_msg.message() {
                RpcMessageType::Call(_) => {
                    // This port must be coerced into u32 (RPC requirement)
                    let port: u16 = rand::thread_rng().gen_range(2076, 2200);

                    eprintln!("{:#?}", rpc_msg);

                    allocate_rpc_channel(port);
                    Ok(())
                },
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "RpcCall is the only supported message type",
                )),
            }
        }).map_err(|e| eprintln!("{:?}", e));

        tokio::run(processor);
        Ok(())
    }
}

/// Method for processing rpc messages
fn process(data: Bytes) -> Result<RpcMessage, &'static str> {
    RpcMessage::try_from(data)
}

fn allocate_rpc_channel(port: u16) -> AllocateRpcChannelHandler {
    AllocateRpcChannelHandler {
        port,
    }
}

struct AllocateRpcChannelHandler {
    port: u16,
}

impl Future for AllocateRpcChannelHandler {
    type Item = (); // TODO: Change RpcClient here
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(Async::Ready(()))
    }
}

fn get_ipv4_socket_addr(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    /// Create a Rpc mock server that only listens on localhost
    fn mock_rpc_server() -> RpcServer {
        RpcServer::new(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 50111),
        )
    }

    #[test]
    fn export_mount_service() {

    }
}
