use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::io::{Error, ErrorKind, Read, Write, self};
use std::thread;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::prelude::*;
use tokio::net::{UdpFramed, UdpSocket};
use tokio::runtime::current_thread::Runtime;
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
                        Err(err) => {
                            eprintln!("RpcProcedureRouter::Error {:#?}\n{:#?}", err, call);
                            panic!();
                        },
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
        let mut runtime = Runtime::new().unwrap();
        let framed = UdpFramed::new(socket, RpcBytesCodec::new());
        let (sink, stream) = framed.split();

        let event_processor = stream.and_then(move |(rpc_msg, peer)| {
            RpcProcedureRouter {
                request: rpc_msg,
                peer: peer,
                handler: handler.clone(),
            }
        });

        runtime.block_on(sink
            .send_all(event_processor)
            .map(|_| ())
            .map_err(|e| eprintln!("RpcProgramServer::Error {:?}", e))
        )
    });

    Ok(local_addr.port())
}

pub struct RpcServer {
    socket_addr: SocketAddr,
    programs: HashMap<(
        RpcProgram,
        u32,
        PortmapProtocol,
    ), u16>,
}

/// This is the Portmap server
impl RpcServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            socket_addr: addr,
            programs: HashMap::new(),
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
    use std::net::{IpAddr, Ipv4Addr, UdpSocket};
    use bytes::Bytes;

    struct Context;
    struct MockEventHandler;
    impl EventHandler for MockEventHandler {
        fn on_event(&self, procedure: &RpcProcedure, _call: &RpcCall) -> Result<RpcReplyMessage, std::io::Error> {
            let context = Context;

            match procedure {
                RpcProcedure::MountExport => {
                    match mount_export_rpc_callback(context) {
                        Ok(reply) => Ok(RpcReplyMessage::MountExport(reply)),
                        Err(err) => Err(err),
                    }
                },
                _ => Err(Error::new(ErrorKind::InvalidInput, "failed")),
            }
        }
    }

    fn mount_export_rpc_callback(_context: Context) -> Result<MountExportReply, std::io::Error> {
        Ok(MountExportReply {
            export_list_entries: vec![
                ExportListEntry::new(
                    String::from("/C/"),
                    vec![
                        String::from("127.0.0.1/24"),
                    ],
                ),
            ],
        })
    }

    fn mock_rpc_server() {
        let server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 50111);
        let server = RpcServer::new(server_address.clone());
        thread::spawn(move || {
            server.run(Arc::new(MockEventHandler)).unwrap();
        });
    }

    fn portmap_server_address<'a>() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 50111)
    }

    fn rpc_allocated_server_address(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
    }

    //#[test]
    fn export_mount_service() {
        let client_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 50222);
        let mut rpc_client = UdpSocket::bind(&client_address).expect("Failed to bind RPC Mock Client socket");
        let portmap_getport_call = Bytes::from(vec![
            0x00, 0x00, 0x00, 0x15, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x86, 0xa0,
            0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x14,
            0xfe, 0xc9, 0x98, 0x11, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x86, 0xa5,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x11,
            0x00, 0x00, 0x00, 0x00
        ]);
        let mount_export_call    = b"\0\0\0\x0c\0\0\0\0\0\0\0\x02\0\x01\x86\xa5\0\0\0\x01\0\0\0\x05\0\0\0\x01\0\0\0\x14\xb0\xb61\x14\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let _mock_rpc_server = mock_rpc_server();

        assert_eq!(76, rpc_client.send_to(&portmap_getport_call, &portmap_server_address()).unwrap());
        let mut buffer = [0; 512];
        let response = rpc_client.recv_from(&mut buffer);
        assert_eq!((28, portmap_server_address()), response.unwrap());

        // TODO: Implment RpcReplyMessage::decode
        // Extract allocated port from portmap reply
        let allocated_port: u16 = u16::from_be_bytes([buffer[26], buffer[27]]);
        assert_eq!(60, rpc_client.send_to(mount_export_call, &rpc_allocated_server_address(allocated_port.clone())).unwrap());
        let mut buffer = [0; 512];
        let response = rpc_client.recv_from(&mut buffer);
        assert_eq!((70, rpc_allocated_server_address(allocated_port.clone())), response.unwrap());
    }
}
