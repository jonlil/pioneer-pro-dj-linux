use std::collections::HashMap;
use crate::rpc::{RPC, self};
use super::client::{LockedClientState};
use std::net::{UdpSocket, SocketAddr, IpAddr};
use std::io::Error;

type RPCPackage = Vec<u8>;
type Callback = fn(&UdpSocket, SocketAddr, RPC, &LockedClientState);

pub struct EventHandler {
    callbacks: HashMap<String, Callback>,
    state: LockedClientState,
    // mpsc
}

impl EventHandler {
    pub fn new(client_state: LockedClientState) -> Self {
        let mut handler = EventHandler {
            callbacks: HashMap::new(),
            state: client_state,
        };

        handler.add_callback("Export", rpc_procedure_export);
        handler.add_callback("Mnt", rpc_procedure_mnt);

        handler
    }

    fn add_callback(&mut self, name: &str, func: Callback) {
        self.callbacks.insert(name.to_string(), func);
    }
}

impl rpc::server::EventHandler for EventHandler {
    fn on_event(&self, name: &str, socket: &UdpSocket, receiver: SocketAddr, rpc_program: RPC) {
        if self.callbacks.contains_key(name) {
            self.callbacks[name](socket, receiver, rpc_program, &self.state);
        }
    }
}

const OPAQUE_DATA: [u8; 2] = [0x00, 0x00];
const VALUE_FOLLOWS: [u8; 4] = [0x00, 0x00, 0x00, 0x01];
const NO_VALUE_FOLLOWS: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

fn rpc_procedure_export(
    socket: &UdpSocket,
    receiver: SocketAddr,
    rpc_program: RPC,
    state: &LockedClientState,
) {
    // This would be nice to abstract away
    match rpc_program {
        RPC::Mount(call, _export) => {
            if let Ok(state) = state.read() {
                if let Some(address) = state.address() {
                    let mut payload: Vec<u8> = vec![];

                    payload.extend(VALUE_FOLLOWS.to_vec());

                    // Directory section
                    // Length
                    payload.extend(vec![0x00, 0x00, 0x00, 0x02]);
                    // Contents
                    payload.extend(vec![0x2f, 0x00]); // "/"
                    // fill bytes
                    payload.extend(OPAQUE_DATA.to_vec());

                    payload.extend(VALUE_FOLLOWS.to_vec());
                    match (address.ip(), address.mask()) {
                        (IpAddr::V4(ip), IpAddr::V4(mask)) => {
                            let group_value: Vec<u8> = format!(
                                "{}/{}",
                                ip,
                                mask,
                            ).bytes().collect();
                            payload.extend(vec![0x00, 0x00, 0x00, 0x1a]);
                            payload.extend(group_value);
                        },
                        _ => panic!("IPv6 is not supported"),
                    }
                    payload.extend(OPAQUE_DATA.to_vec());
                    payload.extend(NO_VALUE_FOLLOWS.to_vec());
                    payload.extend(NO_VALUE_FOLLOWS.to_vec());

                    match send_rpc_reply(&socket, &receiver, call.to_reply(vec![payload])) {
                        Ok(_) => {},
                        Err(_) => {},
                    }
                }
            }
        },
        _ => {},
    }
}

fn rpc_procedure_mnt(
    socket: &UdpSocket,
    receiver: SocketAddr,
    rpc_program: RPC,
    _state: &LockedClientState,
) {
    match rpc_program {
        RPC::Mount(call, _mnt) => {
            let mut payload: Vec<u8> = vec![];
            // Status: ok
            payload.extend(vec![0x00, 0x00, 0x00, 0x00]);
            // crc-32 file handle..
            payload.extend([0u8; 32].to_vec());

            match send_rpc_reply(&socket, &receiver, call.to_reply(vec![payload])) {
                Ok(_) => {},
                Err(_) => {},
            }
        },
        _ => panic!("Invalid RPC Routing"),
    };
}


fn send_rpc_reply(
    socket: &UdpSocket,
    receiver: &SocketAddr,
    reply: Vec<u8>,
) -> Result<(usize), Error> {
    socket.send_to(reply.as_ref(), receiver)
}
