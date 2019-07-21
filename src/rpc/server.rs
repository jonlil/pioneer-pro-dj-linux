use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::io::{Error, ErrorKind};
use std::thread;
use std::io::{Read, Write, self};
use tokio::prelude::*;
use tokio::net::{UdpFramed, UdpSocket};
use futures::{Future, Async, Poll};
use super::packets::*;
use super::codec::RpcBytesCodec;

fn mount_mnt_rpc_callback(call: &RpcCall, data: &MountMnt) -> MountMntReply {
    MountMntReply::new(0, [0x00; 32])
}

fn mount_export_rpc_callback(_call: &RpcCall) -> MountExportReply {
    MountExportReply {
        export_list_entries: vec![
            ExportListEntry::new(
                String::from("/C/"),
                vec![
                    String::from("192.168.10.5/255.255.255.0"),
                ],
            ),
        ],
    }
}

struct RpcProcedureRouter {
    request: RpcMessage,
    peer: SocketAddr,
}

impl Future for RpcProcedureRouter {
    type Item = (RpcMessage, SocketAddr);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let transaction_id = &self.request.xid;

        match self.request.message() {
            RpcMessageType::Call(call) => {
                let reply = match call.procedure() {
                    // TODO: Create program specific enums (excluding portmap.)
                    // We don't want to handle portmap related code here.
                    RpcProcedure::MountExport => RpcReplyMessage::MountExport(mount_export_rpc_callback(call)),
                    RpcProcedure::MountMnt(data) => RpcReplyMessage::MountMnt(mount_mnt_rpc_callback(call, data)),
                    // TODO: Change ErrorKind to something that makes sense
                    _ => panic!("should map to some RPC Error."),
                };

                Ok(Async::Ready((
                    RpcMessage::new(
                        *transaction_id,
                        RpcMessageType::Reply(RpcReply {
                            verifier: RpcAuth::Null,
                            reply_state: RpcReplyState::Accepted,
                            accept_state: RpcAcceptState::Success,
                            data: reply,
                        })
                    ),
                    self.peer,
                )))
            },
            RpcMessageType::Reply(_) => panic!("RpcReply not allowed in call processor."),
        }
    }
}

/// Make this server handle generic program handlers.
fn rpc_program_server() -> Result<u16, Box<std::error::Error>> {
    // let the OS manage port assignment
    let socket = UdpSocket::bind(&get_ipv4_socket_addr(0))?;
    let local_addr = socket.local_addr()?;

    thread::spawn(move || {
        let framed = UdpFramed::new(socket, RpcBytesCodec::new());
        let (sink, stream) = framed.split();

        let event_processor = stream.and_then(|(rpc_msg, peer)| {
            RpcProcedureRouter {
                request: rpc_msg,
                peer: peer,
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
