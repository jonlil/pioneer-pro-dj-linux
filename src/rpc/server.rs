use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::io::{Error, ErrorKind, Read, Write, self};
use std::thread;
use std::sync::Arc;
use tokio::prelude::*;
use tokio::net::{UdpFramed, UdpSocket};
use futures::{Future, Async, Poll};
use super::packets::*;
use super::codec::RpcBytesCodec;
use super::events::EventHandler;

struct RpcProcedureRouter<T>
    where T: EventHandler,
{
    request: RpcMessage,
    peer: SocketAddr,
    handler: Arc<T>,
}

impl<T: EventHandler> Future for RpcProcedureRouter <T>{
    type Item = (RpcMessage, SocketAddr);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let transaction_id = &self.request.xid;

        match self.request.message() {
            RpcMessageType::Call(call) => {
                Ok(Async::Ready((
                    match self.handler.handle_event(call) {
                        Ok(reply) => {
                            RpcMessage::new(
                                *transaction_id,
                                RpcMessageType::Reply(RpcReply {
                                    verifier: RpcAuth::Null,
                                    reply_state: RpcReplyState::Accepted,
                                    accept_state: RpcAcceptState::Success,
                                    data: reply,
                                })
                            )
                        },
                        _ => unimplemented!(),
                    },
                    self.peer))
                )
            },
            RpcMessageType::Reply(_) => panic!("RpcReply not allowed in call processor."),
        }
    }
}

/// Make this server handle generic program handlers.
fn rpc_program_server<T: EventHandler>(handler: Arc<T>) -> Result<u16, Box<std::error::Error>> {
    // let the OS manage port assignment
    let socket = UdpSocket::bind(&get_ipv4_socket_addr(0))?;
    let local_addr = socket.local_addr()?;

    thread::spawn(move || {
        let framed = UdpFramed::new(socket, RpcBytesCodec::new());
        let (sink, stream) = framed.split();

        let event_processor = stream.and_then(move |(rpc_msg, peer)| {
            RpcProcedureRouter {
                request: rpc_msg,
                peer: peer,
                handler: handler.clone(),
            }
        });

        tokio::run(sink
            .send_all(event_processor)
            .map(|_| ())
            .map_err(|e| eprintln!("{:?}", e))
        );
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

    pub fn run<T: EventHandler>(&self, handler: Arc<T>) -> Result<(), Box<std::error::Error>> {
        let socket = UdpSocket::bind(&self.socket_addr)?;
        let framed = UdpFramed::new(socket, RpcBytesCodec::new());
        let (sink, stream) = framed.split();

        let rpc_port_allocator = stream.and_then(move |(rpc_msg, peer)| {
            // Move this logic into AllocateRpcChannelHandler
            AllocateRpcChannelHandler {
                handler: handler.clone(),
            }.map(move |port| {
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

struct AllocateRpcChannelHandler<T>
    where T: EventHandler,
{
    handler: Arc<T>,
}

impl<T: EventHandler> Future for AllocateRpcChannelHandler<T> {
    type Item = u16;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match rpc_program_server(self.handler.clone()) {
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
    // fn mock_rpc_server() -> RpcServer<T> {
    //     RpcServer::new(SocketAddr::new(
    //         IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 50111),
    //     )
    // }

    #[test]
    fn export_mount_service() {}
}
