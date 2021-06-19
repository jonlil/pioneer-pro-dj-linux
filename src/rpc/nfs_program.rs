use std::collections::HashMap;
use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use futures::{SinkExt, StreamExt};
use tokio_util::udp::UdpFramed;

use crate::rpc::codec::RpcBytesCodec;
use crate::rpc::fs::get_fhandle;
use crate::rpc::fs::read_file_range;
use crate::rpc::packets::{self as rpc_packages, NfsFileAttributes, NfsLookupReply, NfsStatus, *};

pub struct RpcNfsProgramHandler {
    path: PathBuf,
    file_handlers: HashMap<u64, File>,
}

#[derive(Debug)]
pub enum NfsProcedureError {
    FileDoesNotExist,
    StaleFileHandle,
    NotImplemented,
}

impl From<std::io::Error> for NfsProcedureError {
    fn from(_error: std::io::Error) -> NfsProcedureError {
        NfsProcedureError::FileDoesNotExist
    }
}

impl RpcNfsProgramHandler {
    pub fn new() -> Self {
        Self {
            path: PathBuf::from("/"),
            file_handlers: HashMap::new(),
        }
    }

    fn reset_path(&mut self) {
        self.path = PathBuf::from("/");
    }

    pub fn lookup(
        &mut self,
        lookup: &rpc_packages::NfsLookup,
    ) -> Result<NfsLookupReply, NfsProcedureError> {
        let mut temp_path = self.path.clone();
        temp_path.push(lookup.filename());

        match std::fs::metadata(temp_path.as_path()) {
            Ok(metadata) => {
                self.path = temp_path;
                let fwrapper = get_fhandle(self.path.as_path(), metadata.ino())?;

                if metadata.is_file() {
                    self.file_handlers.insert(fwrapper.inode, fwrapper.file);
                    self.reset_path();
                }

                Ok(NfsLookupReply {
                    attributes: NfsFileAttributes::from(metadata),
                    fhandle: FileHandle::new(fwrapper.encoded),
                    status: NfsStatus::Ok,
                })
            }
            Err(err) => {
                self.reset_path();
                Err(err.into())
            }
        }
    }

    pub fn getattr(
        &mut self,
        arguments: &NfsGetAttr,
    ) -> Result<NfsGetAttrReply, NfsProcedureError> {
        let inode = arguments.fhandle.ino();
        match self.file_handlers.get(&inode) {
            Some(file) => {
                let metadata = file.metadata()?;
                Ok(NfsGetAttrReply {
                    status: NfsStatus::Ok,
                    attributes: NfsFileAttributes::from(metadata),
                })
            }
            None => Err(NfsProcedureError::StaleFileHandle),
        }
    }

    pub fn read(&mut self, arguments: &NfsRead) -> Result<NfsReadReply, NfsProcedureError> {
        let inode = arguments.fhandle.ino();
        match self.file_handlers.get_mut(&inode) {
            Some(mut file) => {
                let data = read_file_range(&mut file, arguments.offset, arguments.count)?;
                let metadata = file.metadata()?;

                Ok(NfsReadReply {
                    status: NfsStatus::Ok,
                    attributes: NfsFileAttributes::from(metadata),
                    data,
                })
            }
            None => Err(NfsProcedureError::StaleFileHandle),
        }
    }

    fn call_procedure(&mut self, call: &RpcCall) -> Result<RpcReplyMessage, NfsProcedureError> {
        match call.procedure() {
            RpcProcedure::NfsLookup(args) => Ok(RpcReplyMessage::NfsLookup(self.lookup(args)?)),
            RpcProcedure::NfsGetAttr(args) => Ok(RpcReplyMessage::NfsGetAttr(self.getattr(args)?)),
            RpcProcedure::NfsRead(args) => Ok(RpcReplyMessage::NfsRead(self.read(args)?)),
            _ => Err(NfsProcedureError::NotImplemented),
        }
    }

    pub async fn run(&mut self, mut socket: UdpFramed<RpcBytesCodec>) {
        while let Some(package) = socket.next().await {
            match package {
                Ok((rpc_message, address)) => match rpc_message.message() {
                    RpcMessageType::Call(call) => {
                        match self.call_procedure(&call) {
                            Ok(rpc_reply) => {
                                let package = (
                                    RpcMessage::new(
                                        rpc_message.transaction_id(),
                                        RpcMessageType::Reply(RpcReply {
                                            verifier: RpcAuth::Null,
                                            reply_state: RpcReplyState::Accepted,
                                            accept_state: RpcAcceptState::Success,
                                            data: rpc_reply,
                                        }),
                                    ),
                                    address,
                                );
                                socket.send(package).await;
                            }
                            Err(err) => {
                                eprintln!("{:?}", err);
                            }
                        };
                    }
                    _ => {}
                },
                Err(_err) => {}
            }
        }
    }
}
