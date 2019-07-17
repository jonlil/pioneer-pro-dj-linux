use nom::bytes::complete::{tag, take};
use nom::number::complete::{be_u32};
use nom::IResult;
use nom::error::ErrorKind::Switch;
use bytes::{BytesMut, Bytes};
use std::convert::TryFrom;

#[derive(Debug, PartialEq)]
pub enum DecodeError {
    UnhandledMessageType,
}

trait Decode {
    type Output;
    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output>;
}

#[derive(Debug, PartialEq)]
pub struct RpcMessage {
    xid: u32,
    message: RpcMessageType,
}

impl RpcMessage {
    fn decode(input: &[u8]) -> IResult<&[u8], RpcMessage> {
        let (input, xid) = be_u32(input)?;
        let (input, message) = RpcMessageType::decode(input)?;

        Ok((input, RpcMessage {
            xid: xid,
            message: message,
        }))
    }
}

impl TryFrom<Bytes> for RpcMessage {
    type Error = &'static str;

    fn try_from(message: Bytes) -> Result<Self, Self::Error> {
        match RpcMessage::decode(&message) {
            Ok((input, message)) => Ok(message),
            _ => Err("Failed decoding Bytes into RpcMessage"),
        }
    }
}

impl TryFrom<RpcMessage> for Bytes {
    type Error = DecodeError;

    fn try_from(message: RpcMessage) -> Result<Bytes, Self::Error> {
        let mut buffer: BytesMut = BytesMut::new();

        buffer.extend(&message.xid.to_be_bytes());
        buffer.extend(Bytes::from(message.message));

        Ok(Bytes::from(buffer))
    }
}

#[derive(Debug, PartialEq)]
enum RpcAuth {
    Null,
    Unix,
    Short,
    Des,
}

impl Decode for RpcAuth {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, flavor) = be_u32(input)?;
        let (input, _length) = be_u32(input)?;

        match flavor {
            0u32 => Ok((input, RpcAuth::Null)),
            1u32 => Ok((input, RpcAuth::Unix)),
            _ => Err(nom::Err::Error((input, Switch)))
        }
    }
}

#[derive(Debug, PartialEq)]
struct RpcUnixAuth<'a> {
    stamp: u32,
    machine_name: &'a str,
    uid: u32,
    gid: u32,
    gids: u32,
}

#[derive(Debug, PartialEq)]
struct RpcCall {
    version: u32,
    program: RpcProgram,
    program_version: u32,
    procedure: RpcProcedure,
    credentials: RpcCredentials,
    verifier: RpcAuth,
}

impl Decode for RpcCall {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, version) = be_u32(input)?;
        let (input, (program, program_version)) = RpcProgram::decode(input)?;
        let (input, procedure) = be_u32(input)?;
        let (input, credentials) = RpcCredentials::decode(input)?;
        let (input, verifier) = RpcAuth::decode(input)?;
        let (input, procedure) = RpcProcedure::decode(input, &program, procedure)?;

        Ok((input, RpcCall {
            version,
            program,
            program_version,
            procedure,
            credentials,
            verifier,
        }))
    }
}

#[derive(Debug, PartialEq)]
struct RpcReply {
    verifier: RpcAuth,
    accept_state: RpcState,
    data: RpcReplyMessage,
}

impl Decode for RpcReply {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        unimplemented!()
    }
}

#[derive(Debug, PartialEq)]
struct PortmapGetportReply {
    port: u32,
}

#[derive(Debug, PartialEq)]
enum RpcReplyMessage {
    PortmapGetport(PortmapGetportReply),
}

#[derive(Debug, PartialEq)]
enum RpcState {
    Success,
}

#[derive(Debug, PartialEq)]
enum RpcMessageType {
    Call(RpcCall),
    Reply(RpcReply),
}

impl Decode for RpcMessageType {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, message_type) = be_u32(input)?;

        match message_type {
            0u32 => {
                let (input, rpc_call) = RpcCall::decode(input)?;
                Ok((input, RpcMessageType::Call(rpc_call)))
            },
            1u32 => {
                let (input, rpc_reply) = RpcReply::decode(input)?;
                Ok((input, RpcMessageType::Reply(rpc_reply)))
            },
            _ => Err(nom::Err::Error((input, Switch))),
        }
    }
}

impl From<RpcMessageType> for Bytes {
    fn from(message: RpcMessageType) -> Self {
        let mut buffer = BytesMut::new();

        match message {
            RpcMessageType::Reply(reply) => {
                buffer.extend(1u32.to_be_bytes().as_ref());
            },
            _ => {},
        };

        Bytes::from(buffer)
    }
}

#[derive(Debug, PartialEq)]
enum RpcProgram {
    Portmap,
    Nfs,
    Mount,
}

impl Decode for RpcProgram {
    type Output = (RpcProgram, u32);

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, program) = be_u32(input)?;
        let (input, program_version) = be_u32(input)?;

        match program {
            100000u32 => Ok((input, (RpcProgram::Portmap, program_version))),
            100003u32 => Ok((input, (RpcProgram::Nfs, program_version))),
            100005u32 => Ok((input, (RpcProgram::Mount, program_version))),
            _ => Err(nom::Err::Error((input, Switch))),
        }
    }
}

#[derive(Debug, PartialEq)]
enum PortmapProcedure {
  Null,
  Set,
  Unset,
  Getport,
  Dump,
  CallResult,
}

impl Decode for PortmapProcedure {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, procedure) = be_u32(input)?;
        match procedure {
            0u32 => Ok((input, PortmapProcedure::Null)),
            1u32 => Ok((input, PortmapProcedure::Set)),
            2u32 => Ok((input, PortmapProcedure::Unset)),
            3u32 => Ok((input, PortmapProcedure::Getport)),
            4u32 => Ok((input, PortmapProcedure::Dump)),
            5u32 => Ok((input, PortmapProcedure::CallResult)),
            _ => Err(nom::Err::Error((input, Switch))),
        }
    }
}

#[derive(Debug, PartialEq)]
enum RpcProcedure {
    PortmapNull,
    PortmapSet,
    PortmapUnset,
    PortmapGetport(PortmapGetport),
    PortmapDump,
    PortmapCallResult,
    NfsNull,
    MountNull,
}

impl RpcProcedure {
    fn decode<'a>(input: &'a [u8], program: &RpcProgram, procedure: u32) -> IResult<&'a [u8], RpcProcedure> {
        match (program, procedure) {
            (RpcProgram::Portmap, 0u32) => Ok((input, RpcProcedure::PortmapNull)),
            (RpcProgram::Portmap, 1u32) => Ok((input, RpcProcedure::PortmapSet)),
            (RpcProgram::Portmap, 2u32) => Ok((input, RpcProcedure::PortmapUnset)),
            (RpcProgram::Portmap, 3u32) => {
                let (input, data) = PortmapGetport::decode(&input)?;
                Ok((input, RpcProcedure::PortmapGetport(data)))
            },
            (RpcProgram::Portmap, 4u32) => Ok((input, RpcProcedure::PortmapDump)),
            (RpcProgram::Portmap, 5u32) => Ok((input, RpcProcedure::PortmapCallResult)),
            (RpcProgram::Portmap, _) => Err(nom::Err::Error((input, Switch))),
            (RpcProgram::Nfs, _) => Err(nom::Err::Error((input, Switch))),
            (RpcProgram::Mount, _) => Err(nom::Err::Error((input, Switch))),
        }
    }
}

#[derive(Debug, PartialEq)]
struct PortmapGetport {
    version: u32,
    program: RpcProgram,
    protocol: PortmapProtocol,
    port: u32,
}

impl Decode for PortmapGetport {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output>{
        let (input, (program, version)) = RpcProgram::decode(input)?;
        let (input, protocol) = PortmapProtocol::decode(input)?;
        let (input, port) = be_u32(input)?;

        Ok((input, PortmapGetport {
            version,
            program,
            protocol,
            port,
        }))
    }
}

#[derive(Debug, PartialEq)]
enum PortmapProtocol {
    Ip,
    Udp,
}

impl Decode for PortmapProtocol {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, protocol) = be_u32(input)?;

        eprintln!("{}", protocol);


        match protocol {
            17u32 => Ok((input, PortmapProtocol::Udp)),
            _ => Err(nom::Err::Error((input, Switch))),
        }
    }
}

#[derive(Debug, PartialEq)]
enum NfsProcedure {
    Null,
    Getattr,
    Sattrargs,
    Root,
    Lookup,
    Readlink,
    Read,
    Writecache,
    Write,
    Create,
    Remove,
    Rename,
    Link,
    Symlink,
    Mkdir,
    Rmdir,
    Readdir,
    Statfs,
}

impl Decode for NfsProcedure {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, procedure) = be_u32(input)?;
        match procedure {
            0 => { Ok((input, NfsProcedure::Null)) },
            1 => { Ok((input, NfsProcedure::Getattr)) },
            2 => { Ok((input, NfsProcedure::Sattrargs)) },
            3 => { Ok((input, NfsProcedure::Root)) },
            4 => { Ok((input, NfsProcedure::Lookup)) },
            5 => { Ok((input, NfsProcedure::Readlink)) },
            6 => { Ok((input, NfsProcedure::Read)) },
            7 => { Ok((input, NfsProcedure::Writecache)) },
            8 => { Ok((input, NfsProcedure::Write)) },
            9 => { Ok((input, NfsProcedure::Create)) },
            10 => { Ok((input, NfsProcedure::Remove)) },
            11 => { Ok((input, NfsProcedure::Rename)) },
            12 => { Ok((input, NfsProcedure::Link)) },
            13 => { Ok((input, NfsProcedure::Symlink)) },
            14 => { Ok((input, NfsProcedure::Mkdir)) },
            15 => { Ok((input, NfsProcedure::Rmdir)) },
            16 => { Ok((input, NfsProcedure::Readdir)) },
            17 => { Ok((input, NfsProcedure::Statfs)) },
            _ => Err(nom::Err::Error((input, Switch))),
        }
    }
}

#[derive(Debug, PartialEq)]
struct RpcCredentials {
    flavor: u32,
    length: u32,
    stamp: u32,
    machine_name: u32,
    uid: u32,
    gid: u32,
    aux_gid: u32,
}

impl Decode for RpcCredentials {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, flavor) = be_u32(input)?;
        let (input, length) = be_u32(input)?;
        let (input, stamp) = be_u32(input)?;
        let (input, machine_name) = be_u32(input)?;
        let (input, uid) = be_u32(input)?;
        let (input, gid) = be_u32(input)?;
        let (input, aux_gid) = be_u32(input)?;

        Ok((input, RpcCredentials {
            flavor,
            length,
            stamp,
            machine_name,
            uid,
            gid,
            aux_gid,
        }))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::RPC;
    use bytes::Bytes;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_decoding_portmap_getport_call() {
        let call = Bytes::from(vec![
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

        assert_eq!(
            Ok((&[][..], RpcMessage {
                xid: 21u32,
                message: RpcMessageType::Call(RpcCall {
                    version: 2,
                    program: RpcProgram::Portmap,
                    program_version: 2,
                    procedure: RpcProcedure::PortmapGetport(PortmapGetport {
                        version: 1,
                        program: RpcProgram::Mount,
                        protocol: PortmapProtocol::Udp,
                        port: 0,
                    }),
                    credentials: RpcCredentials {
                        flavor: 1,
                        length: 20,
                        stamp: 4274624529,
                        machine_name: 0u32,
                        uid: 0u32,
                        gid: 0u32,
                        aux_gid: 0u32,
                    },
                    verifier: RpcAuth::Null,
                }),
            })),
            RpcMessage::decode(&call),
        );
    }

    #[test]
    fn test_encoding_portmap_getport_reply() {
        let reply = Bytes::from(vec![
            0x00, 0x00, 0x00, 0x15, 0x00, 0x00, 0x00, 0x01, 
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
            0x00, 0x00, 0x8d, 0x9b
        ]);

        assert_eq!(Ok(reply), Bytes::try_from(RpcMessage {
            xid: 21,
            message: RpcMessageType::Reply(RpcReply {
                verifier: RpcAuth::Null,
                accept_state: RpcState::Success,
                data: RpcReplyMessage::PortmapGetport(
                    PortmapGetportReply {
                        port: 36251,
                    },
                ),
            }),
        }))
    }
}
