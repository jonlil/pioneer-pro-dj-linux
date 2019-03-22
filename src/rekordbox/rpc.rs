use std::sync::{Arc, RwLock};
use std::ops::RangeInclusive;
use super::util::clone_into_array;
use super::client::ClientState;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::net::{UdpSocket, SocketAddr};
use std::io;
use crate::utils::network::PioneerNetwork;


type RPCPropertyValue = [u8; 4];

#[derive(Debug, PartialEq)]
pub enum RPC {
    Portmap(RPCCall, Portmap),
}

#[derive(Debug, PartialEq)]
pub enum Portmap {
    Getport(Getport),
}

#[derive(Debug, PartialEq)]
pub enum Getport {
    Mount(Mount),
    NFS,
}

enum RPCProcedure {
    Getport,
    Unknown,
}

enum RPCProgram {
    Portmap,
    Unknown,
}

pub struct RPCServer {
    state: Arc<RwLock<ClientState>>,
}
impl RPCServer {
    pub fn new(state: Arc<RwLock<ClientState>>) -> Self {
        Self {
            state: state,
        }
    }

    pub fn run(&self) {
        // TODO: implement rpc callback ports pool
        thread::spawn(move || {
            let socket = UdpSocket::bind(("0.0.0.0", 50111)).unwrap();
            let mut socket_port_offset = 0;

            loop {
                let mut buffer = [0u8; 512];
                match socket.recv_from(&mut buffer) {
                    Ok((number_of_bytes, source)) => {
                        match parse_rpc_message(&buffer) {
                            Ok(event) => {
                                match event {
                                    RPC::Portmap(rpc_call, portmap) => {

                                        match socket.send_to(&[
                                                0x00,
                                            ], source) {
                                            Ok(nob) => eprintln!("reply: {:?}", nob),
                                            Err(err) => eprintln!("{:?}", err.to_string()),
                                        }
                                    }
                                }
                            },
                            Err(_) => {},
                        }
                    },
                    Err(err) => eprintln!("{:?}", err)
                }
                thread::sleep(Duration::from_millis(300));
            }
        });
    }

    // This method should have access to adding events to the event loop
    pub fn create_response_socket(network: &PioneerNetwork) {
        unimplemented!();

        //thread::spawn(move || {
        //    let socket = UdpSocket::bind((network.ip(), 
        //    
        //});
    }
}

#[derive(Debug, PartialEq)]
pub struct RPCCall {
    xid: RPCPropertyValue,
    message_type: RPCPropertyValue,
    rpc_version: RPCPropertyValue,
    program: RPCPropertyValue,
    program_version: RPCPropertyValue,
    procedure: RPCPropertyValue,
    credentials: RPCCredentials,
    verifier: RPCVerifier,
}

impl RPCCall {
    fn get_program(&self) -> RPCProgram {
        if self.program == [0x00, 0x01, 0x86, 0xa0] {
            RPCProgram::Portmap
        } else {
            RPCProgram::Unknown
        }
    }

    fn get_procedure(&self) -> RPCProcedure {
        if self.procedure == [0x00, 0x00, 0x00, 0x03] {
            RPCProcedure::Getport
        } else {
            RPCProcedure::Unknown
        }
    }
}

#[derive(Debug, PartialEq)]
struct RPCVerifier {
    flavor: RPCPropertyValue,
    length: RPCPropertyValue,
}

#[derive(Debug, PartialEq)]
struct RPCCredentials {
    flavor: RPCPropertyValue,
    length: RPCPropertyValue,
    stamp: RPCPropertyValue,
    machine_name: RPCPropertyValue,
    uid: RPCPropertyValue,
    gid: RPCPropertyValue,
    aux_gid: RPCPropertyValue,
}

#[derive(Debug, PartialEq)]
struct Mount {
    program: RPCPropertyValue,
    version: RPCPropertyValue,
    protocol: RPCPropertyValue,
    port: RPCPropertyValue,
}

#[derive(Debug, PartialEq)]
enum RPCMessageType {
    Call,
    Reply,
    Unknown,
}

const RPC_MESSAGE_TYPE_POSITION: RangeInclusive<usize> = 4..=7;

fn get_message_type(message: &[u8]) -> RPCMessageType {
    if message[RPC_MESSAGE_TYPE_POSITION] == [0x00, 0x00, 0x00, 0x00] {
        RPCMessageType::Call
    } else if message[RPC_MESSAGE_TYPE_POSITION] == [0x00, 0x00, 0x00, 0x01] {
        RPCMessageType::Reply
    } else {
        RPCMessageType::Unknown
    }
}

fn parse_rpc_call(message: &[u8]) -> RPCCall {
    RPCCall {
        xid: clone_into_array(&message[0..=3]),
        message_type: clone_into_array(&message[RPC_MESSAGE_TYPE_POSITION]),
        rpc_version: clone_into_array(&message[8..=11]),
        program: clone_into_array(&message[12..=15]),
        program_version: clone_into_array(&message[16..=19]),
        procedure: clone_into_array(&message[20..=23]),
        credentials: RPCCredentials {
            flavor: clone_into_array(&message[24..=27]),
            length: clone_into_array(&message[28..=31]),
            stamp: clone_into_array(&message[32..=35]),
            machine_name: clone_into_array(&message[36..=39]),
            uid: clone_into_array(&message[40..=43]),
            gid: clone_into_array(&message[44..=47]),
            aux_gid: clone_into_array(&message[48..=51]),
        },
        verifier: RPCVerifier {
            flavor: clone_into_array(&message[52..=55]),
            length: clone_into_array(&message[56..=59]),
        }
    }
}

fn parse_rpc_program(call: RPCCall, message: &[u8]) -> Result<RPC, &'static str> {
    match (call.get_program(), call.get_procedure()) {
        (RPCProgram::Portmap, RPCProcedure::Getport) => {
            Ok(RPC::Portmap(call, Portmap::Getport(
                Getport::Mount(Mount {
                    program: clone_into_array(&message[60..=63]),
                    version: clone_into_array(&message[64..=67]),
                    protocol: clone_into_array(&message[68..=71]),
                    port: clone_into_array(&message[72..=75]),
                })
            )) )
        },
        (_, _) => Err("Unknown rpc program"),
    }
}

// parse call (message type)
pub fn parse_rpc_message(message: &[u8]) -> Result<RPC, &'static str> {
    match get_message_type(message.as_ref()) {
        RPCMessageType::Call => {
            let call = parse_rpc_call(message.as_ref());
            parse_rpc_program(call, message.as_ref())
        },
        RPCMessageType::Reply => {
            Err("err")
        },
        RPCMessageType::Unknown => {
            Err("Err")
        },
    }
}

#[cfg(test)]
mod test {
    use crate::rekordbox::rpc::{
        RPCCall,
        RPCVerifier,
        RPCCredentials,
        RPC,
        RPCMessageType,
        Getport,
        Portmap,
        Mount,
        parse_rpc_message,
        get_message_type,
    };

    #[test]
    fn it_can_match_message_type() {
        assert_eq!(get_message_type(&[
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00]
        ), RPCMessageType::Call);

        assert_eq!(get_message_type(&[
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01]
        ), RPCMessageType::Reply);
    }

    #[test]
    fn it_can_parse_rpc_message() {
        let message: Vec<u8> = vec![

            // ============= RPC START =============

            // XID
            0x00,0x00,0x00,0x01,
            // Message type
            0x00,0x00,0x00,0x00,
            // RPC Ver.
            0x00,0x00,0x00,0x02,
            // Program
            0x00,0x01,0x86,0xa0,
            // Program version
            0x00,0x00,0x00,0x02,
            // Procedure
            0x00,0x00,0x00,0x03,

            // Credentials

            // Flavor
            0x00,0x00,0x00,0x01,
            // Length
            0x00,0x00,0x00,0x14,
            // Stamp
            0x96,0x7b,0x87,0x03,
            // Machine name
            0x00,0x00,0x00,0x00,
            // UID
            0x00,0x00,0x00,0x00,
            // GID
            0x00,0x00,0x00,0x00,
            // AUX GID
            0x00,0x00,0x00,0x00,

            // Verifier

            // Flavor
            0x00,0x00,0x00,0x00,
            // Length
            0x00,0x00,0x00,0x00,

            // ========== RPC END ============

            // PORTMAP

            // Program
            0x00,0x01,0x86,0xa5,
            // Version
            0x00,0x00,0x00,0x01,
            // Protocol
            0x00,0x00,0x00,0x11,
            // Port
            0x00,0x00,0x00,0x00,
        ];

        assert_eq!(parse_rpc_message(message.as_ref()), Ok(
            RPC::Portmap(
                RPCCall {
                    xid: [0x00, 0x00, 0x00, 0x01],
                    message_type: [0x00,0x00,0x00,0x00],
                    rpc_version: [0x00,0x00,0x00,0x02],
                    program: [0x00,0x01,0x86,0xa0],
                    program_version: [0x00,0x00,0x00,0x02],
                    procedure: [0x00,0x00,0x00,0x03],
                    credentials: RPCCredentials {
                        flavor: [0x00,0x00,0x00,0x01],
                        length: [0x00,0x00,0x00,0x14],
                        stamp: [0x96,0x7b,0x87,0x03],
                        machine_name: [0x00,0x00,0x00,0x00],
                        uid: [0x00,0x00,0x00,0x00],
                        gid: [0x00,0x00,0x00,0x00],
                        aux_gid: [0x00,0x00,0x00,0x00],
                    },
                    verifier: RPCVerifier {
                        flavor: [0x00,0x00,0x00,0x00],
                        length: [0x00,0x00,0x00,0x00],
                    }
                },
                Portmap::Getport(Getport::Mount(Mount {
                    program: [0x00,0x01,0x86,0xa5],
                    version: [0x00,0x00,0x00,0x01],
                    protocol: [0x00,0x00,0x00,0x11],
                    port: [0x00,0x00,0x00,0x00],
                }))
            )
        ));
    }
}
