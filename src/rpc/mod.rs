#![allow(dead_code)]
#![allow(non_snake_case)]

use std::ops::RangeInclusive;
use crate::rekordbox::util::clone_into_array;

pub mod server;
mod pooled_port;
mod packets;

#[cfg(test)]
mod test;
#[cfg(test)]
pub mod factories;

type RPCPropertyValue = [u8; 4];

pub mod NFS {
    #[derive(Debug, PartialEq)]
    pub enum Procedure {
        Lookup(Lookup),
        Getattr(Getattr),
        Read(Read),
    }

    #[derive(Debug, PartialEq)]
    pub struct Lookup;
    #[derive(Debug, PartialEq)]
    pub struct Getattr;
    #[derive(Debug, PartialEq)]
    pub struct Read;
}

pub mod Portmap {
    use super::RPCPropertyValue;
    use crate::rekordbox::util::clone_into_array;

    #[derive(Debug, PartialEq)]
    pub enum Program {
        Getport(Procedure)
    }

    #[derive(Debug, PartialEq)]
    pub enum Procedure {
        Mount(Portmap),
        NFS(Portmap),
        Unknown,
    }

    fn unpack(buffer: &[u8]) -> Portmap {
        Portmap {
            program: clone_into_array(&buffer[..=3]),
            version: clone_into_array(&buffer[4..=7]),
            protocol: clone_into_array(&buffer[8..=11]),
            port: clone_into_array(&buffer[12..=15]),
        }
    }

    pub fn unmarshall(message: &[u8]) -> Program {
        Program::Getport(match &message[3] {
            0xa5 => Procedure::Mount(unpack(&message)),
            0xa3 => Procedure::NFS(unpack(&message)),
            _ => Procedure::Unknown,
        })
    }

    #[derive(Debug, PartialEq)]
    pub struct Portmap {
        pub program: RPCPropertyValue,
        pub version: RPCPropertyValue,
        pub protocol: RPCPropertyValue,
        pub port: RPCPropertyValue,
    }

    #[cfg(test)]
    mod tests {
        use super::{Procedure, unmarshall, Program};

        #[test]
        fn it_can_unmarshal_procedures() {
            let mut buffer = [0x00; 16];
            buffer[0] = 0x00;
            buffer[1] = 0x01;
            buffer[2] = 0x86;
            buffer[3] = 0xa5;

            assert_eq!(match unmarshall(&buffer) {
                Program::Getport(program) => {
                    match program {
                        Procedure::Mount(_portmap) => true,
                        _ => false,
                    }
                }
            }, true);

            buffer[3] = 0xa3;
            assert_eq!(match unmarshall(&buffer) {
                Program::Getport(program) => {
                    match program {
                        Procedure::NFS(_portmap) => true,
                        _ => false,
                    }
                }
            }, true);
        }
    }
}

pub mod Mount {
    use super::{RPCProcedure, RPCCall};
    use crate::rekordbox::util::clone_into_array;

    #[derive(Debug, PartialEq)]
    pub enum Procedure {
        Export(Export),
        Mnt(Mnt),
        Unknown,
    }

    pub fn unmarshall(rpc_call: &RPCCall, buffer: &[u8]) -> Procedure {
        match rpc_call.procedure {
            RPCProcedure::Export => Procedure::Export(Export {}),
            RPCProcedure::Mnt => Procedure::Mnt(Mnt::unmarshall(&buffer)),
            _ => Procedure::Unknown,
        }
    }

    #[derive(Debug, PartialEq)]
    pub struct Export;
    #[derive(Debug, PartialEq)]
    pub struct Mnt {
        path: [u8; 8],
    }

    impl Mnt {
        fn unmarshall(bytes: &[u8]) -> Mnt {
            Mnt {
                path: clone_into_array(&bytes[..=7]),
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum RPC {
    Portmap(RPCCall, Portmap::Program),
    NFS(RPCCall, NFS::Procedure),
    Mount(RPCCall, Mount::Procedure),
    Error(String),
}

#[derive(Debug, PartialEq)]
enum RPCProgram {
    NFS,
    Mount,
    Portmap,
    Unknown,
}

#[derive(Debug, PartialEq)]
enum RPCProcedure {
    Export,
    Getport,
    Mnt,
    Lookup,
    Getattr,
    Read,
    Unknown,
}

impl RPC {
    fn unmarshall_message(buffer: &[u8]) -> Result<(RPCCall, &[u8]), &'static str> {
        if buffer[RPC_MESSAGE_TYPE_POSITION] != [0x00, 0x00, 0x00, 0x00] {
            return Err("Server received a reply");
        }

        if buffer[8..=11] != [0x00, 0x00, 0x00, 0x02] {
            return Err("Only RPC Version 2 is supported");
        }

        let (program, procedure) = RPC::match_program(&buffer[12..=23]);
        let call = RPCCall {
            xid: clone_into_array(&buffer[0..=3]),
            message_type: clone_into_array(&buffer[RPC_MESSAGE_TYPE_POSITION]),
            rpc_version: clone_into_array(&buffer[8..=11]),
            program: program,
            program_version: clone_into_array(&buffer[16..=19]),
            procedure: procedure,
            credentials: RPCCredentials {
                flavor: clone_into_array(&buffer[24..=27]),
                length: clone_into_array(&buffer[28..=31]),
                stamp: clone_into_array(&buffer[32..=35]),
                machine_name: clone_into_array(&buffer[36..=39]),
                uid: clone_into_array(&buffer[40..=43]),
                gid: clone_into_array(&buffer[44..=47]),
                aux_gid: clone_into_array(&buffer[48..=51]),
            },
            verifier: RPCVerifier {
                flavor: clone_into_array(&buffer[52..=55]),
                length: clone_into_array(&buffer[56..=59]),
            }
        };

        Ok((call, &buffer[60..]))
    }

    // This buffer should not contain RPC Call body
    fn unmarshall_procedure(call: RPCCall, buffer: &[u8]) -> RPC {
        match call.program {
            RPCProgram::Portmap => {
                RPC::Portmap(call, Portmap::unmarshall(&buffer))
            },
            RPCProgram::Mount => {
                let procedure = Mount::unmarshall(&call, &buffer);
                RPC::Mount(call, procedure)
            },
            _ => RPC::Error(String::from("Unhandled type")),
            //RPCProgram::Mount => {},
            //RPCProgram::NFS => {},
            //RPCProgram::Unknown => {},
        }
    }

    pub fn unmarshall(buffer: &[u8]) -> RPC {
        match Self::unmarshall_message(&buffer) {
            Ok((call, procedure_buffer)) => Self::unmarshall_procedure(call, &procedure_buffer),
            Err(err) => RPC::Error(err.to_string()),
        }
    }

    fn match_program(buffer: &[u8]) -> (RPCProgram, RPCProcedure) {
        let program = &buffer[..=3];
        let _program_version = &buffer[4..=7];
        let procedure = &buffer[8..];

        if program == &[0x00, 0x01, 0x86, 0xa0] {
            (RPCProgram::Portmap, match procedure {
                &[0x00, 0x00, 0x00, 0x03] => RPCProcedure::Getport,
                _ => RPCProcedure::Unknown,
            })
        }
        else if program == &[0x00, 0x01, 0x86, 0xa3] {
            (RPCProgram::NFS, match procedure {
                &[0x00, 0x00, 0x00, 0x04] => RPCProcedure::Lookup,
                &[0x00, 0x00, 0x00, 0x01] => RPCProcedure::Getattr,
                &[0x00, 0x00, 0x00, 0x06] => RPCProcedure::Read,
                _ => RPCProcedure::Unknown,
            })
        }
        else if program == &[0x00, 0x01, 0x86, 0xa5] {
            (RPCProgram::Mount, match procedure {
                &[0x00, 0x00, 0x00, 0x01] => RPCProcedure::Mnt,
                &[0x00, 0x00, 0x00, 0x05] => RPCProcedure::Export,
                _ => RPCProcedure::Unknown,
            })
        } else {
            (RPCProgram::Unknown, RPCProcedure::Unknown)
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct RPCCall {
    xid: RPCPropertyValue,
    message_type: RPCPropertyValue,
    rpc_version: RPCPropertyValue,
    program: RPCProgram,
    program_version: RPCPropertyValue,
    procedure: RPCProcedure,
    credentials: RPCCredentials,
    verifier: RPCVerifier,
}

impl RPCCall {
    pub fn to_reply(&self, payload: Vec<Vec<u8>>) -> Vec<u8> {
        let mut data: Vec<Vec<u8>> = vec![
            self.xid.to_vec(),
            // Message type
            [0x00, 0x00, 0x00, 0x01].to_vec(),
            // Reply state
            [0u8; 4].to_vec(),

            // Verifier
            [0u8; 4].to_vec(),
            [0u8; 4].to_vec(),

            // Accept state
            [0u8; 4].to_vec(),
        ];

        for chunk in payload {
            data.push(chunk);
        }

        data.into_iter().flatten().collect()
    }
}

/// RPC data types

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
enum RPCMessageType {
    Call,
    Reply,
}

#[derive(Debug, PartialEq)]
pub enum RPCReply {
    Export(Mount::Export),
    Mnt(Mount::Mnt),
}

const RPC_MESSAGE_TYPE_POSITION: RangeInclusive<usize> = 4..=7;
