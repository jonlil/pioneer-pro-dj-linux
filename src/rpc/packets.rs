use nom::bytes::complete::{tag, take};
use nom::number::complete::{be_u32, be_u16, le_u16};
use nom::multi::count;
use nom::IResult;
use nom::error::ErrorKind::Switch;
use bytes::{BytesMut, Bytes, BufMut};
use std::convert::TryFrom;

#[derive(Debug, PartialEq)]
pub enum Error {
    UnhandledMessageType,
}

trait Decoder {
    type Output;
    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output>;
}

#[derive(Debug, PartialEq)]
pub struct RpcMessage {
    pub xid: u32,
    message: RpcMessageType,
}

impl RpcMessage {
    pub fn new(transaction_id: u32, message: RpcMessageType) -> RpcMessage {
        RpcMessage {
            xid: transaction_id,
            message: message,
        }
    }

    fn decode(input: &[u8]) -> IResult<&[u8], RpcMessage> {
        let (input, xid) = be_u32(input)?;
        let (input, message) = RpcMessageType::decode(input)?;

        Ok((input, RpcMessage {
            xid: xid,
            message: message,
        }))
    }

    pub fn transaction_id(self) -> u32 {
        self.xid
    }

    pub fn message(&self) -> &RpcMessageType {
        &self.message
    }
}

impl TryFrom<Bytes> for RpcMessage {
    type Error = &'static str;

    fn try_from(message: Bytes) -> Result<Self, Self::Error> {
        match RpcMessage::decode(&message) {
            Ok((_input, message)) => Ok(message),
            _ => {
                eprintln!("TryFrom failed: {:?}", message);
                Err("Failed decoding Bytes into RpcMessage")
            },
        }
    }
}

impl TryFrom<RpcMessage> for Bytes {
    type Error = Error;

    fn try_from(message: RpcMessage) -> Result<Bytes, Self::Error> {
        let mut buffer: BytesMut = BytesMut::new();

        buffer.extend(&message.xid.to_be_bytes());
        buffer.extend(Bytes::from(message.message));

        Ok(Bytes::from(buffer))
    }
}

#[derive(Debug, PartialEq)]
pub enum RpcAuth {
    Null,
    Unix,
    Short,
    Des,
}

impl Decoder for RpcAuth {
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

impl From<RpcAuth> for Bytes {
    fn from(auth: RpcAuth) -> Bytes {
        let mut buffer = BytesMut::new();

        let rpc_auth = match auth {
            RpcAuth::Null => 0u32,
            _             => 16u32,
        };

        buffer.extend(rpc_auth.to_be_bytes().as_ref());
        buffer.extend(0u32.to_be_bytes().as_ref());

        Bytes::from(buffer)
    }
}

#[derive(Debug, PartialEq)]
pub struct RpcUnixAuth<'a> {
    stamp: u32,
    machine_name: &'a str,
    uid: u32,
    gid: u32,
    gids: u32,
}

#[derive(Debug, PartialEq)]
pub struct RpcCall {
    version: u32,
    program: RpcProgram,
    program_version: u32,
    procedure: RpcProcedure,
    credentials: RpcCredentials,
    verifier: RpcAuth,
}

impl RpcCall {
    pub fn procedure(&self) -> &RpcProcedure {
        &self.procedure
    }
}

impl Decoder for RpcCall {
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
pub struct RpcReply {
    pub verifier: RpcAuth,
    pub reply_state: RpcReplyState,
    pub accept_state: RpcAcceptState,
    pub data: RpcReplyMessage,
}

impl Decoder for RpcReply {
    type Output = Self;

    fn decode(_input: &[u8]) -> IResult<&[u8], Self::Output> {
        unimplemented!()
    }
}

impl From<RpcReply> for Bytes {
    fn from(reply: RpcReply) -> Bytes {
        let mut buffer = BytesMut::new();

        buffer.extend(Bytes::from(reply.verifier));
        buffer.extend(Bytes::from(reply.reply_state));
        buffer.extend(Bytes::from(reply.accept_state));
        buffer.extend(Bytes::from(reply.data));

        Bytes::from(buffer)
    }
}

#[derive(Debug, PartialEq)]
pub enum RpcReplyMessage {
    PortmapGetport(PortmapGetportReply),
    MountExport(MountExportReply),
    MountMnt(MountMntReply),
}

impl From<RpcReplyMessage> for Bytes {
    fn from(reply_message: RpcReplyMessage) -> Bytes {
        match reply_message {
            RpcReplyMessage::PortmapGetport(reply) => Bytes::from(reply),
            RpcReplyMessage::MountExport(reply)    => Bytes::from(reply),
            RpcReplyMessage::MountMnt(reply)       => Bytes::from(reply),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct PortmapGetportReply {
    pub port: u32,
}

impl From<PortmapGetportReply> for Bytes {
    fn from(reply: PortmapGetportReply) -> Bytes {
        Bytes::from(reply.port.to_be_bytes().as_ref())
    }
}


impl From<ExportListEntry> for Bytes {
    fn from(entry: ExportListEntry) -> Bytes {
        let mut buf = BytesMut::new();

        let content = entry.directory.encode_utf16()
            .into_iter()
            .flat_map(|item| { item.to_le_bytes().to_vec() })
            .collect::<Bytes>();
        buf.put((content.len() as u32).to_be_bytes().to_vec());
        buf.put(content);
        buf.put(OPAQUE_DATA.to_vec());

        if entry.groups.len() > 0 {
            buf.put(VALUE_FOLLOWS.to_vec());

            for group in entry.groups {
                buf.extend((group.len() as u32).to_be_bytes().to_vec());
                buf.extend(group.as_bytes());
                buf.extend(OPAQUE_DATA.to_vec());
            }
        }

        buf.extend(NO_VALUE_FOLLOWS.to_vec());

        Bytes::from(buf)
    }
}

#[derive(Debug, PartialEq)]
pub struct MountExportReply {
    pub export_list_entries: Vec<ExportListEntry>,
}

const VALUE_FOLLOWS: [u8; 4] = [0x00, 0x00, 0x00, 0x01];
const NO_VALUE_FOLLOWS: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
const OPAQUE_DATA: [u8; 2] = [0x00, 0x00];

impl From<MountExportReply> for Bytes {
    fn from(reply: MountExportReply) -> Bytes {
        let mut buf = BytesMut::new();

        if reply.export_list_entries.len() > 0 {
            buf.extend(VALUE_FOLLOWS.to_vec());

            for entry in reply.export_list_entries {
                buf.extend(Bytes::from(entry));
            }
        }

        buf.extend(NO_VALUE_FOLLOWS.to_vec());
        Bytes::from(buf)
    }
}

#[derive(Debug, PartialEq)]
pub struct ExportListEntry {
    directory: String,
    groups: Vec<String>,
}

impl ExportListEntry {
    pub fn new(directory: String, groups: Vec<String>) -> ExportListEntry {
        ExportListEntry {
            directory,
            groups,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct MountMntReply {
    status: u32,
    fhandle: [u8; 32],
}

impl MountMntReply {
    pub fn new(status: u32, fhandle: [u8; 32]) -> MountMntReply {
        MountMntReply {
            status,
            fhandle,
        }
    }
}

impl From<MountMntReply> for Bytes {
    fn from(reply: MountMntReply) -> Bytes {
        let mut buf = BytesMut::new();

        buf.extend(reply.status.to_be_bytes().to_vec());
        buf.extend(reply.fhandle.to_vec());

        Bytes::from(buf)
    }
}

#[derive(Debug, PartialEq)]
pub enum RpcAcceptState {
    Success,
}

impl From<RpcAcceptState> for Bytes {
    fn from(state: RpcAcceptState) -> Bytes {
        let reply_state_value = match state {
            RpcAcceptState::Success => 0u32,
        };

        Bytes::from(reply_state_value.to_be_bytes().as_ref())
    }
}

#[derive(Debug, PartialEq)]
pub enum RpcReplyState {
    Accepted,
}

impl From<RpcReplyState> for Bytes {
    fn from(state: RpcReplyState) -> Bytes {
        let reply_state_value = match state {
            RpcReplyState::Accepted => 0u32,
        };

        Bytes::from(reply_state_value.to_be_bytes().as_ref())
    }
}

#[derive(Debug, PartialEq)]
pub enum RpcMessageType {
    Call(RpcCall),
    Reply(RpcReply),
}

impl Decoder for RpcMessageType {
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
                buffer.extend(Bytes::from(reply));
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

impl Decoder for RpcProgram {
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

impl Decoder for PortmapProcedure {
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
pub enum RpcProcedure {
    PortmapNull,
    PortmapSet,
    PortmapUnset,
    PortmapGetport(PortmapGetport),
    PortmapDump,
    PortmapCallResult,
    NfsNull,
    MountMnt(MountMnt),
    MountExport,
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
            (RpcProgram::Portmap, _)    => Err(nom::Err::Error((input, Switch))),
            (RpcProgram::Nfs, _)        => Err(nom::Err::Error((input, Switch))),
            (RpcProgram::Mount, 5u32)   => Ok((input, RpcProcedure::MountExport)),
            (RpcProgram::Mount, 1u32)   => {
                let (input, data) = MountMnt::decode(&input)?;
                Ok((input, RpcProcedure::MountMnt(data)))
            },
            (RpcProgram::Mount, _)      => Err(nom::Err::Error((input, Switch))),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct MountMnt {
    paths: Vec<String>,
}

impl Decoder for MountMnt {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, length) = be_u32(input)?;
        let (input, contents) = count(le_u16, length as usize / 2)(input)?;
        let (input, _fill_bytes) = be_u16(input)?;

        match String::from_utf16(&contents) {
            Ok(contents) => {
                Ok((input, MountMnt {
                    paths: vec![
                        contents,
                    ]
                }))
            },
            // TODO: change this ErrorTag to something relevant.
            Err(_err) => Err(nom::Err::Error((input, Switch))),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct PortmapGetport {
    version: u32,
    program: RpcProgram,
    protocol: PortmapProtocol,
    port: u32,
}

impl Decoder for PortmapGetport {
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

impl Decoder for PortmapProtocol {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, protocol) = be_u32(input)?;

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

impl Decoder for NfsProcedure {
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

impl Decoder for RpcCredentials {
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
    use bytes::Bytes;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_decoding_portmap_nfs_call() {
        let call = Bytes::from(vec![
            0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x86, 0xa0,
            0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x14,
            0x25, 0x37, 0x91, 0x2c, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x86, 0xa3,
            0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x11,
            0x00, 0x00, 0x00, 0x00
        ]);

        assert_eq!(
            Ok((&[][..], RpcMessage {
                xid: 259u32,
                message: RpcMessageType::Call(RpcCall {
                    version: 2,
                    program: RpcProgram::Portmap,
                    program_version: 2,
                    procedure: RpcProcedure::PortmapGetport(PortmapGetport {
                        version: 2,
                        program: RpcProgram::Nfs,
                        protocol: PortmapProtocol::Udp,
                        port: 0,
                    }),
                    credentials: RpcCredentials {
                        flavor: 1,
                        length: 20,
                        stamp: 624398636,
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
                reply_state: RpcReplyState::Accepted,
                accept_state: RpcAcceptState::Success,
                data: RpcReplyMessage::PortmapGetport(
                    PortmapGetportReply {
                        port: 36251,
                    },
                ),
            }),
        }))
    }

    #[test]
    fn test_decoding_mount_export_call() {
        let call = b"\0\0\0\x0c\0\0\0\0\0\0\0\x02\0\x01\x86\xa5\0\0\0\x01\0\0\0\x05\0\0\0\x01\0\0\0\x14\xb0\xb61\x14\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

        assert_eq!(
            Ok((&[][..], RpcMessage {
                xid: 12u32,
                message: RpcMessageType::Call(RpcCall {
                    version: 2,
                    program: RpcProgram::Mount,
                    program_version: 1,
                    procedure: RpcProcedure::MountExport,
                    credentials: RpcCredentials {
                        flavor: 1,
                        length: 20,
                        stamp: 2964730132,
                        machine_name: 0u32,
                        uid: 0u32,
                        gid: 0u32,
                        aux_gid: 0u32,
                    },
                    verifier: RpcAuth::Null,
                }),
            })),
            RpcMessage::decode(&Bytes::from(call.to_vec())),
        );
    }

    /// Test to verify that we can encode RpcReply of type MountExport to bytes
    #[test]
    fn test_encoding_mount_export_reply() {
        let reply = Bytes::from(vec![
            0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x06,
            0x2f, 0x00, 0x43, 0x00, 0x2f, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x1a,
            0x31, 0x39, 0x32, 0x2e, 0x31, 0x36, 0x38, 0x2e,
            0x31, 0x30, 0x2e, 0x35, 0x2f, 0x32, 0x35, 0x35,
            0x2e, 0x32, 0x35, 0x35, 0x2e, 0x32, 0x35, 0x35,
            0x2e, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ]);

        assert_eq!(Ok(reply), Bytes::try_from(RpcMessage {
            xid: 22,
            message: RpcMessageType::Reply(RpcReply {
                verifier: RpcAuth::Null,
                reply_state: RpcReplyState::Accepted,
                accept_state: RpcAcceptState::Success,
                data: RpcReplyMessage::MountExport(
                    MountExportReply {
                        export_list_entries: vec![
                            ExportListEntry {
                                directory: String::from("/C/"),
                                groups: vec![
                                    String::from("192.168.10.5/255.255.255.0"),
                                ],
                            },
                        ],
                    },
                ),
            }),
        }));
    }

    #[test]
    fn test_decoding_mount_mnt_call() {
        let call = b"\0\0\0\x16\0\0\0\0\0\0\0\x02\0\x01\x86\xa5\0\0\0\x01\0\0\0\x01\0\0\0\x01\0\0\0\x14\xb0\x8c\xe1\x04\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x06/\0C\0/\0\0\x11";

        assert_eq!(
            Ok((&[][..], RpcMessage {
                xid: 22u32,
                message: RpcMessageType::Call(RpcCall {
                    version: 2,
                    program: RpcProgram::Mount,
                    program_version: 1,
                    procedure: RpcProcedure::MountMnt(
                        MountMnt {
                            paths: vec![String::from("/C/")],
                        },
                    ),
                    credentials: RpcCredentials {
                        flavor: 1,
                        length: 20,
                        stamp: 2962022660,
                        machine_name: 0u32,
                        uid: 0u32,
                        gid: 0u32,
                        aux_gid: 0u32,
                    },
                    verifier: RpcAuth::Null,
                }),
            })),
            RpcMessage::decode(&Bytes::from(call.to_vec())),
        );
    }

    #[test]
    fn test_encoding_mount_mnt_reply() {
        let reply = Bytes::from(vec![
            0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00
        ]);

        assert_eq!(Ok(reply), Bytes::try_from(RpcMessage {
            xid: 25,
            message: RpcMessageType::Reply(RpcReply {
                verifier: RpcAuth::Null,
                reply_state: RpcReplyState::Accepted,
                accept_state: RpcAcceptState::Success,
                data: RpcReplyMessage::MountMnt(
                    MountMntReply {
                        status: 0,
                        fhandle: [0x00; 32],
                    },
                ),
            }),
        }));
    }
}
