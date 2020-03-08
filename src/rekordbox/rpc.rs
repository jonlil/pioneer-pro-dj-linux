use std::io::{Error, ErrorKind};
use crate::rpc::events::{EventHandler as RpcEventHandler, RpcResult};
use crate::rpc::PortmapServer;
use crate::rpc::packets::*;
use crate::rekordbox::ServerState;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

struct Context<'a> {
    state: &'a Arc<Mutex<ServerState>>,
    call: &'a RpcCall,
}

pub async fn server(state_ref: Arc<Mutex<ServerState>>) -> Result<(), std::io::Error> {
    let portmap_server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 50111);
    let event_handler = EventHandler::new(state_ref.clone());

    let join = tokio::task::spawn(async move {
        let server = PortmapServer::new(portmap_server_addr);
        // Start RPC server
        dbg!("Starting portmap server");
        match server.run(Arc::new(event_handler)).await {
            Ok(_) => {},
            Err(err) => {
                panic!(format!("RpcServerHandler panic: {:?}", err));
            },
        }
    });

    join.await?;

    Ok(())
}

fn mount_mnt_rpc_callback(_context: Context, _data: &MountMnt) -> Result<MountMntReply, std::io::Error> {
    Ok(MountMntReply::new(0, FileHandle::new([0x00; 32])))
}

fn mount_export_rpc_callback(context: Context) -> Result<MountExportReply, std::io::Error> {
    if let Ok(state) = context.state.lock() {
        if let Some(address) = state.address() {
            let address = format!("{}/{}", address.ip(), address.mask());

            return Ok(MountExportReply {
                export_list_entries: vec![
                    ExportListEntry::new(
                        String::from("/"),
                        vec![
                            address,
                        ],
                    ),
                ],
            })
        }
    }
    Err(Error::new(ErrorKind::InvalidInput, "Duno"))
}

pub struct EventHandler {
    state: Arc<Mutex<ServerState>>,
}

impl EventHandler {
    pub fn new(client_state: Arc<Mutex<ServerState>>) -> Self {
        EventHandler {
            state: client_state,
        }
    }
}

impl RpcEventHandler for EventHandler {
    fn on_event(&self, procedure: &RpcProcedure, call: &RpcCall) -> RpcResult {
        let context = Context {
            call: call,
            state: &self.state,
        };

        match procedure {
            RpcProcedure::MountExport => {
                Some(match mount_export_rpc_callback(context) {
                    Ok(reply) => Ok(RpcReplyMessage::MountExport(reply)),
                    Err(err) => Err(err),
                })
            },
            RpcProcedure::MountMnt(data) => {
                Some(match mount_mnt_rpc_callback(context, data) {
                    Ok(reply) => Ok(RpcReplyMessage::MountMnt(reply)),
                    Err(err) => Err(err),
                })
            },
            _ => None,
        }
    }
}
