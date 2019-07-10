extern crate byteorder;

use byteorder::{WriteBytesExt, BigEndian};
use std::net::{UdpSocket, SocketAddr};
use super::pooled_port::Pool;
use super::{RPC, Portmap, RPCCall, Mount};
use std::io::Error;
use std::time::Duration;
use std::thread;
use std::sync::Arc;

pub struct RPCServer {
    port_pool: Pool,
    socket: UdpSocket,
    handler: Arc<EventHandler>,
}

pub trait EventHandler: Send + Sync + 'static {
    fn on_event(&self, name: &str, socket: &UdpSocket, receiver: SocketAddr, rpc_program: RPC);
}

impl RPCServer {
    pub fn new<T: EventHandler>(handler: T) -> Self {
        Self {
            port_pool: Pool::new(4096..=4104).unwrap(),
            socket: UdpSocket::bind(("0.0.0.0", 50111)).unwrap(),
            handler: Arc::new(handler),
        }
    }

    pub fn run(&self) {
        loop {
            match self.recv() {
                Ok((rpc_call, source)) => self.portmap_router(rpc_call, source),
                Err(err) => eprintln!("{:?}", err.to_string()),
            }

            thread::sleep(Duration::from_millis(300));
        }
    }

    fn portmap_router(
        &self,
        call: RPC,
        receiver: SocketAddr,
    ) {
        match call {
            RPC::Portmap(rpc_call, Portmap::Program::Getport(program)) => {
                match program {
                    Portmap::Procedure::Mount(_portmap) => {
                        self.create_response_handler(rpc_call, receiver);
                    },
                    Portmap::Procedure::NFS(_portmap) => {
                        self.create_response_handler(rpc_call, receiver);
                    },
                    Portmap::Procedure::Unknown => {},
                }
            },
            _ => {
                eprintln!("{:?}", "portmap event");
            }
        }
    }

    fn recv(&self) -> Result<(RPC, SocketAddr), Error> {
        let mut buffer = [0u8; 512];
        match self.socket.recv_from(&mut buffer) {
            Ok((number_of_bytes, source)) => {
                Ok((RPC::unmarshall(&buffer[..number_of_bytes]), source))
            },
            Err(err) => Err(err),
        }
    }

    fn allocate_socket(reply_port: u16) -> Result<UdpSocket, Error> {
        match UdpSocket::bind(("0.0.0.0", reply_port)) {
            Ok(socket) => {
                match socket.set_read_timeout(Some(Duration::from_millis(5000))) {
                    Ok(_) => Ok(socket),
                    Err(err) => Err(err),
                }
            },
            Err(err) => Err(err),
        }
    }

    pub fn create_response_handler(&self, call: RPCCall, receiver: SocketAddr) {
        let _procedure_callback_handler = {
            let handler = self.handler.clone();
            let reply_port = self.port_pool.get().unwrap();
            let reply = call.to_reply(vec![
                vec![0x00, 0x00],
                // TODO: Implement support for le.
                convert_u16_to_two_u8s_be(reply_port.get_port())
            ]);

            // Possible todo: Implement thread pooling to avoid too many concurrent threads
            thread::spawn(move || {
                match Self::allocate_socket(reply_port.get_port()) {
                    Ok(socket) => PortmapProgramHandler::detect(&socket, |rpc| {
                        let call_event_handler = |method: &str, rpc: RPC| {
                            handler.on_event(method, &socket, receiver, rpc);
                        };

                        match &rpc {
                            RPC::Mount(_call, Mount::Procedure::Export(_export)) => call_event_handler("Export", rpc),
                            RPC::Mount(_call, Mount::Procedure::Mnt(_mnt)) => call_event_handler("Mnt", rpc),
                            RPC::Error(err) => eprintln!("PortmapProgramHandler errored: {:?}", err),
                            _ => {
                                eprintln!("Received non-implemented RPC Program: {:?}", rpc);
                            },
                        };
                    }),
                    Err(err) => eprintln!("{:?}", err),
                }
            });

            match self.socket.send_to(reply.as_ref(), receiver) {
                Ok(_) => {},
                Err(error) => eprintln!("{:?}", error),
            }
        };
    }
}

struct PortmapProgramHandler;
impl PortmapProgramHandler {
    pub fn detect<F>(socket: &UdpSocket, callback: F) where
        F: FnOnce(RPC) {
        let mut buffer = [0u8; 512];
        match socket.recv(&mut buffer) {
            Ok(bytes) => callback(RPC::unmarshall(&buffer[..bytes])),
            Err(err) => callback(RPC::Error(err.to_string())),
        };
    }
}

pub fn convert_u16_to_two_u8s_be(integer: u16) -> Vec<u8> {
    let mut res = vec![];
    res.write_u16::<BigEndian>(integer).unwrap();
    res
}
