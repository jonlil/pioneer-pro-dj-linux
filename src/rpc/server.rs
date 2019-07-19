use std::net::{SocketAddr, IpAddr, Ipv4Addr};

use tokio::prelude::*;
use tokio::net::{UdpFramed, UdpSocket};
use std::io::{Read, Write, self};
use futures::{Future, Async, Poll};
use std::io::{Error, ErrorKind};
use std::thread;

use super::packets::{
    RpcMessage,
    RpcMessageType,
    RpcAuth,
    RpcReplyState,
    RpcReply,
    RpcAcceptState,
    RpcReplyMessage,
    PortmapGetportReply
};
use super::codec::RpcBytesCodec;

fn rpc_program_server() -> Result<u16, Box<std::error::Error>> {
    // let the OS manage port assignment
    let socket = UdpSocket::bind(&get_ipv4_socket_addr(0))?;
    let local_addr = socket.local_addr()?;

    thread::spawn(move || {
        let framed = UdpFramed::new(socket, RpcBytesCodec::new());
        let (sink, stream) = framed.split();

        let program = stream.for_each(|(rpc_msg, _peer)| {
            eprintln!("{:#?}", rpc_msg);
            Ok(())
        });

        tokio::run(program.map_err(|e| eprintln!("{:?}", e)));
    });

    Ok(local_addr.port())
}

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
        let (sink, stream) = framed.split();

        let rpc_port_allocator = stream.and_then(|(rpc_msg, peer)| {
            // Move this logic into AllocateRpcChannelHandler
            AllocateRpcChannelHandler.map(move |port| {
                (
                    RpcMessage::new(
                        rpc_msg.transaction_id(),
                        RpcMessageType::Reply(RpcReply {
                            verifier: RpcAuth::Null,
                            reply_state: RpcReplyState::Accepted,
                            accept_state: RpcAcceptState::Success,
                            data: RpcReplyMessage::PortmapGetport(
                                PortmapGetportReply {
                                    port: port as u32,
                                },
                            ),
                        }),
                    ),
                    peer,
                )
            })
        });

        let processor = sink.send_all(rpc_port_allocator)
            .map(|_| ())
            .map_err(|e| eprintln!("{}", e));

        tokio::run(processor);

        Ok(())
    }
}

struct AllocateRpcChannelHandler;
impl Future for AllocateRpcChannelHandler {
    type Item = u16;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match rpc_program_server() {
            Ok(port) => Ok(Async::Ready(port)),
            Err(_) => Err(Error::new(ErrorKind::AddrInUse, "failed allocating port")),
        }
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
