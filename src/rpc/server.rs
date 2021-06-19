use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio_util::udp::UdpFramed;

use super::codec::RpcBytesCodec;
use super::events::EventHandler;
use super::packets::*;
use crate::rpc::nfs_program::RpcNfsProgramHandler;

struct RpcProcedureRouter<T>
where
    T: EventHandler,
{
    request: RpcMessage,
    peer: SocketAddr,
    handler: Arc<T>,
}

#[derive(Debug)]
enum RpcServerError {
    ProgramNotImplemented,
    ReplyNotAllowed,
    IOError(std::io::Error),
}

fn serialize_rpc_reply_message(reply: RpcReplyMessage, transaction_id: u32) -> RpcMessage {
    RpcMessage::new(
        transaction_id,
        RpcMessageType::Reply(RpcReply {
            verifier: RpcAuth::Null,
            reply_state: RpcReplyState::Accepted,
            accept_state: RpcAcceptState::Success,
            data: reply,
        }),
    )
}

fn rpc_procedure_router<T: EventHandler>(
    request: RpcMessage,
    address: SocketAddr,
    handler: Arc<T>,
) -> Result<(RpcMessage, SocketAddr), RpcServerError> {
    let transaction_id = request.xid;
    match request.message() {
        RpcMessageType::Call(call) => match handler.handle_event(&call) {
            Some(Ok(reply)) => Ok((serialize_rpc_reply_message(reply, transaction_id), address)),
            Some(Err(e)) => Err(RpcServerError::IOError(e)),
            None => Err(RpcServerError::ProgramNotImplemented),
        },
        RpcMessageType::Reply(_) => Err(RpcServerError::ReplyNotAllowed),
    }
}

/// Make this server handle generic program handlers.
///
/// This server will crash directly it is unable to process a message in either direction.
async fn rpc_program_server<T: EventHandler>(
    mut socket: UdpFramed<RpcBytesCodec>,
    handler: Arc<T>,
) -> Result<(), String> {
    while let Some(package) = socket.next().await {
        let handler = handler.clone();

        match package {
            Ok((request, address)) => {
                let message =
                    rpc_procedure_router(request, address, handler.clone()).map_err(|err| {
                        format!(
                            "failed processing RPC Message into reply; error = {:?}",
                            err
                        )
                    })?;
                socket
                    .send(message)
                    .await
                    .map_err(|_| String::from("Failed sending on socket"))?;
            }
            Err(err) => eprintln!("error decoding bytes into RPC Message; err = {}", err),
        }
    }

    Ok(())
}

pub struct PortmapServer {
    socket_addr: SocketAddr,
    programs: HashMap<(RpcProgram, u32, PortmapProtocol), u16>,
}

/// This is the Portmap server
impl PortmapServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            socket_addr: addr,
            programs: HashMap::new(),
        }
    }

    pub async fn run<T: EventHandler>(&self, handler: Arc<T>) -> Result<(), std::io::Error> {
        let socket = UdpSocket::bind(&self.socket_addr).await?;
        let mut socket = UdpFramed::new(socket, RpcBytesCodec::new());

        while let Some(result) = socket.next().await {
            match result {
                Ok((rpc_message, address)) => {
                    let handler = handler.clone();
                    let allocated_rpc_socket = UdpSocket::bind(&get_ipv4_socket_addr(0)).await?;
                    let local_addr = allocated_rpc_socket.local_addr()?;
                    let allocated_rpc_socket =
                        UdpFramed::new(allocated_rpc_socket, RpcBytesCodec::new());

                    match rpc_message.message() {
                        RpcMessageType::Call(call) => {
                            match call.procedure() {
                                RpcProcedure::PortmapGetport(getport) => {
                                    match getport.program() {
                                        RpcProgram::Nfs => {
                                            tokio::spawn(async move {
                                                let mut program_handler =
                                                    RpcNfsProgramHandler::new();
                                                program_handler.run(allocated_rpc_socket).await;
                                            });
                                        }
                                        _ => {
                                            // Spawn RPC Program in thread to handle multiple concurrent clients
                                            tokio::spawn(async move {
                                                rpc_program_server(allocated_rpc_socket, handler)
                                                    .await
                                            });
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    };

                    // Prepare portmap response
                    let portmap_response = RpcMessage::new(
                        rpc_message.transaction_id(),
                        RpcMessageType::Reply(RpcReply {
                            verifier: RpcAuth::Null,
                            reply_state: RpcReplyState::Accepted,
                            accept_state: RpcAcceptState::Success,
                            data: RpcReplyMessage::PortmapGetport(PortmapGetportReply {
                                port: local_addr.port() as u32,
                            }),
                        }),
                    );

                    socket.send((portmap_response, address)).await?
                }
                _ => {}
            };
        }

        Ok(())
    }
}

fn get_ipv4_socket_addr(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port)
}

#[cfg(test)]
mod test {
    use super::super::events::RpcResult;
    use super::*;
    use bytes::Bytes;
    use std::io::{Error, ErrorKind};
    use std::net::{IpAddr, Ipv4Addr, UdpSocket};
    use std::path::Path;

    struct Context;
    struct MockEventHandler;
    impl EventHandler for MockEventHandler {
        fn on_event(&self, procedure: &RpcProcedure, _call: &RpcCall) -> RpcResult {
            let context = Context;

            match procedure {
                RpcProcedure::MountExport => Some(match mount_export_rpc_callback(context) {
                    Ok(reply) => Ok(RpcReplyMessage::MountExport(reply)),
                    Err(err) => Err(err),
                }),
                _ => Some(Err(Error::new(ErrorKind::InvalidInput, "failed"))),
            }
        }
    }

    fn mount_export_rpc_callback(_context: Context) -> Result<MountExportReply, std::io::Error> {
        Ok(MountExportReply {
            export_list_entries: vec![ExportListEntry::new(
                String::from("/C/"),
                vec![String::from("127.0.0.1/24")],
            )],
        })
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
        let rpc_client =
            UdpSocket::bind(&client_address).expect("Failed to bind RPC Mock Client socket");
        let portmap_getport_call = Bytes::from(vec![
            0x00, 0x00, 0x00, 0x15, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01,
            0x86, 0xa0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x14, 0xfe, 0xc9, 0x98, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x86, 0xa5, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x11, 0x00, 0x00, 0x00, 0x00,
        ]);
        let mount_export_call    = b"\0\0\0\x0c\0\0\0\0\0\0\0\x02\0\x01\x86\xa5\0\0\0\x01\0\0\0\x05\0\0\0\x01\0\0\0\x14\xb0\xb61\x14\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

        assert_eq!(
            76,
            rpc_client
                .send_to(&portmap_getport_call, &portmap_server_address())
                .unwrap()
        );
        let mut buffer = [0; 512];
        let response = rpc_client.recv_from(&mut buffer);
        assert_eq!((28, portmap_server_address()), response.unwrap());

        // TODO: Implment RpcReplyMessage::decode
        // Extract allocated port from portmap reply
        let allocated_port: u16 = u16::from_be_bytes([buffer[26], buffer[27]]);
        assert_eq!(
            60,
            rpc_client
                .send_to(
                    mount_export_call,
                    &rpc_allocated_server_address(allocated_port.clone())
                )
                .unwrap()
        );
        let mut buffer = [0; 512];
        let response = rpc_client.recv_from(&mut buffer);
        assert_eq!(
            (70, rpc_allocated_server_address(allocated_port.clone())),
            response.unwrap()
        );
    }
}
