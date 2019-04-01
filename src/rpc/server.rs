extern crate byteorder;

use byteorder::{WriteBytesExt, BigEndian};
use std::net::{UdpSocket, SocketAddr};
use crate::rekordbox::rpc_socket_pool::Pool;
use super::{RPC, Portmap, RPCCall, RPCReply, Mount};
use std::io::Error;
use std::time::Duration;
use std::thread;
use std::sync::Arc;

pub struct RPCServer {
    port_pool: Pool,
    socket: UdpSocket,
}

pub trait EventHandler: Send + Sync + 'static {
    fn on_event(&self, name: &str, rpc_program: RPC) -> Option<RPCReply>;
}

impl RPCServer {
    pub fn new() -> Self {
        Self {
            port_pool: Pool::new(4096..=4104).unwrap(),
            socket: UdpSocket::bind(("0.0.0.0", 50111)).unwrap(),
        }
    }

    pub fn run<T: EventHandler>(&self, handler: T) {
        let handler_ref = Arc::new(handler);

        loop {
            // We should only receive portmapping events here.
            // This should be routed to the correct module.
            match self.recv() {
                Ok((rpc_call, source)) => self.portmap_router(rpc_call, source, handler_ref.clone()),
                Err(err) => eprintln!("{:?}", err.to_string()),
            }

            thread::sleep(Duration::from_millis(300));
        }
    }

    fn portmap_router<T: EventHandler>(
        &self,
        call: RPC,
        receiver: SocketAddr,
        handler: Arc<T>,
    ) {
        match call {
            RPC::Portmap(rpc_call, Portmap::Program::Getport(program)) => {
                match program {
                    Portmap::Procedure::Mount(portmap) => {
                        self.create_response_handler(rpc_call, receiver, handler);
                    },
                    Portmap::Procedure::NFS(portmap) => {
                        eprintln!("Portmapping NFS: {:?}", portmap);
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

    pub fn create_response_handler<T: EventHandler>(
        &self,
        call: RPCCall,
        receiver: SocketAddr,
        handler: Arc<T>,
    ) {
        let reply_port = self.port_pool.get().unwrap();
        let allocated_socket = UdpSocket::bind(("0.0.0.0", reply_port.get_port())).unwrap();
        let data: Vec<u8> = vec![
            call.xid.to_vec(),
            [0x00, 0x00, 0x00, 0x01].to_vec(),
            [0u8; 4].to_vec(),
            [0u8; 4].to_vec(),
            [0u8; 4].to_vec(),
            [0u8; 4].to_vec(),
            vec![0x00, 0x00], convert_u16_to_two_u8s_be(reply_port.get_port()),
        ].into_iter().flatten().collect();

        match self.socket.send_to(data.as_ref(), receiver) {
            Ok(nob) => eprintln!("{:?}", nob),
            Err(error) => eprintln!("{:?}", error),
        }

        let _procedure_callback_handler = {
            thread::spawn(move || {
                allocated_socket.set_read_timeout(Some(Duration::from_millis(2000))).unwrap();

                let mut buffer = [0u8; 512];
                match allocated_socket.recv(&mut buffer) {
                    Ok(number_of_bytes) => {
                        let rpc = RPC::unmarshall(&buffer[..number_of_bytes]);
                        match &rpc {
                            RPC::Mount(call, Mount::Procedure::Export(export)) => {
                                handler.on_event("Export", rpc);
                            },
                            _ => {},
                        }
                    },
                    Err(error) => eprintln!("{:?}", error.to_string()),
                }
            });
        };
    }
}

fn convert_u16_to_two_u8s_be(integer: u16) -> Vec<u8> {
    let mut res = vec![];
    res.write_u16::<BigEndian>(integer).unwrap();
    res
}
