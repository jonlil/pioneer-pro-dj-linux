use std::io::{Error, ErrorKind};
use std::net::IpAddr;
use crate::rpc::events::EventHandler as RpcEventHandler;
use super::state::{LockedClientState};
use crate::rpc::packets::*;

struct Context<'a> {
    state: &'a LockedClientState,
    call: RpcCall,
}

fn mount_mnt_rpc_callback(context: Context, data: MountMnt) -> Result<MountMntReply, std::io::Error> {
    Ok(MountMntReply::new(0, [0x00; 32]))
}

fn mount_export_rpc_callback(context: Context) -> Result<MountExportReply, std::io::Error> {
    if let Ok(state) = context.state.read() {
        if let Some(address) = state.address() {
            let address = match (address.ip(), address.mask()) {
                (IpAddr::V4(ip), IpAddr::V4(mask)) => {
                    format!("{}/{}", ip, mask)
                },
                _ => panic!("IPv6 is not supported"),
            };

            return Ok(MountExportReply {
                export_list_entries: vec![
                    ExportListEntry::new(
                        String::from("/C/"),
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
    state: LockedClientState,
}

impl EventHandler {
    pub fn new(client_state: LockedClientState) -> Self {
        EventHandler {
            state: client_state,
        }
    }
}

impl RpcEventHandler for EventHandler {
    fn on_event(&self, procedure: RpcProcedure, call: RpcCall) -> Result<RpcReplyMessage, std::io::Error> {
        let context = Context {
            call: call,
            state: &self.state,
        };

        match procedure {
            RpcProcedure::MountExport => {
                match mount_export_rpc_callback(context) {
                    Ok(reply) => Ok(RpcReplyMessage::MountExport(reply)),
                    Err(err) => Err(err),
                }
            },
            RpcProcedure::MountMnt(data) => {
                match mount_mnt_rpc_callback(context, data) {
                    Ok(reply) => Ok(RpcReplyMessage::MountMnt(reply)),
                    Err(err) => Err(err),
                }
            },
            _ => Err(Error::new(ErrorKind::InvalidInput, "failed")),
        }
    }
}
