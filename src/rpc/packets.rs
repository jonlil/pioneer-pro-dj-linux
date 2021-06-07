use nom::number::complete::{be_u32, be_u16, le_u16, be_u8};
use nom::multi::count;
use nom::IResult;
use nom::error::{
    ErrorKind::{Switch, MapRes},
};
use bytes::{BytesMut, Bytes, BufMut};
use std::convert::TryFrom;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;

use crate::utils::parse_error;

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
            _ => Err(parse_error(input, Switch))
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

    pub fn program(&self) -> &RpcProgram {
        &self.program
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
pub struct NfsReadReply {
    pub status: NfsStatus,
    pub attributes: NfsFileAttributes,
    pub data: NfsDataWrapper,
}

impl From<NfsReadReply> for Bytes {
    fn from(data: NfsReadReply) -> Self {
        let mut buffer = BytesMut::new();

        buffer.extend(Bytes::from(data.status));
        buffer.extend(Bytes::from(data.attributes));
        buffer.extend(Bytes::from(data.data));

        buffer.freeze()
    }
}

#[derive(Debug, PartialEq)]
pub struct NfsDataWrapper {
    pub data: Vec<u8>,
}

impl From<NfsDataWrapper> for Bytes {
    fn from(data: NfsDataWrapper) -> Self {
        let mut buffer = BytesMut::new();

        buffer.put_u32(data.data.len() as u32);
        buffer.extend(&data.data);

        buffer.freeze()
    }
}

#[derive(Debug, PartialEq)]
pub enum RpcReplyMessage {
    PortmapGetport(PortmapGetportReply),
    MountExport(MountExportReply),
    MountMnt(MountMntReply),
    NfsLookup(NfsLookupReply),
    NfsGetAttr(NfsGetAttrReply),
    NfsRead(NfsReadReply),
}

impl From<RpcReplyMessage> for Bytes {
    fn from(reply_message: RpcReplyMessage) -> Bytes {
        match reply_message {
            RpcReplyMessage::PortmapGetport(reply) => Bytes::from(reply),
            RpcReplyMessage::MountExport(reply)    => Bytes::from(reply),
            RpcReplyMessage::MountMnt(reply)       => Bytes::from(reply),
            RpcReplyMessage::NfsLookup(reply)      => Bytes::from(reply),
            RpcReplyMessage::NfsGetAttr(reply)     => Bytes::from(reply),
            RpcReplyMessage::NfsRead(reply)        => Bytes::from(reply),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct PortmapGetportReply {
    pub port: u32,
}

impl From<PortmapGetportReply> for Bytes {
    fn from(reply: PortmapGetportReply) -> Bytes {
        Bytes::from(reply.port.to_be_bytes().to_vec())
    }
}


impl From<ExportListEntry> for Bytes {
    fn from(entry: ExportListEntry) -> Bytes {
        let mut buf = BytesMut::new();

        let content = entry.directory.encode_utf16()
            .into_iter()
            .flat_map(|item| { item.to_le_bytes().to_vec() })
            .collect::<Bytes>();
        buf.extend((content.len() as u32).to_be_bytes().to_vec());
        buf.put(content);
        buf.extend(OPAQUE_DATA.to_vec());

        if entry.groups.len() > 0 {
            buf.extend(VALUE_FOLLOWS.to_vec());

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
pub enum NfsStatus {
    Ok,
}

impl From<NfsStatus> for Bytes {
    fn from(status: NfsStatus) -> Bytes {
        let mut buffer = BytesMut::new();
        buffer.put_u32(match status {
            NfsStatus::Ok => 0u32,
        });
        buffer.freeze()
    }
}

#[derive(Debug, PartialEq)]
pub struct MountMntReply {
    status: u32,
    fhandle: FileHandle,
}

impl MountMntReply {
    pub fn new(status: u32, fhandle: FileHandle) -> MountMntReply {
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
        buf.extend(Bytes::from(reply.fhandle).to_vec());

        Bytes::from(buf)
    }
}

impl From<Metadata> for NfsFileAttributes {
    fn from(metadata: std::fs::Metadata) -> Self {
        Self {
            _type: match metadata.is_dir() {
                true => FileType::Directory,
                false => FileType::File,
            },
            mode: FileMode {
                name: 0x00,
                user: 0x00,
                group: 81,
                other: 24,
            },
            nlink: metadata.nlink() as u32,
            uid: metadata.uid(),
            gid: metadata.gid(),
            size: metadata.size() as u32,
            blocksize: metadata.blksize() as u32,
            rdev: metadata.rdev() as u32,
            blocks: metadata.blocks() as u32,
            fsid: 0,
            file_id: 0,
            atime: metadata.accessed().unwrap(),
            mtime: metadata.modified().unwrap(),
            ctime: metadata.created().unwrap(),
        }
    }
}

impl Default for NfsFileAttributes {
    fn default() -> Self {
        Self {
            _type: FileType::Directory,
            mode: FileMode {
                name: 0x00,
                user: 0x00,
                group: 0x00,
                other: 0x00,
            },
            nlink: 0,
            uid: 0,
            gid: 0,
            size: 0,
            blocksize: 0,
            rdev: 0,
            blocks: 0,
            fsid: 0,
            file_id: 0,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
        }
    }
}

impl Default for NfsLookupReply {
    fn default() -> Self {
        Self {
            status: NfsStatus::Ok,
            fhandle: FileHandle::new([0u8; 32]),
            attributes: NfsFileAttributes::default(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct NfsLookupReply {
    pub status: NfsStatus,
    pub fhandle: FileHandle,
    pub attributes: NfsFileAttributes,
}

/// This method transforms std::time::SystemTime (u64)
/// to a timeval type as defined in RFC 1094.
fn system_time_to_bytes(time: SystemTime) -> Bytes {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => {
            let mut buffer = BytesMut::with_capacity(8);

            // RFC 1094 use 4 bytes for representing timestamp
            let secs: [u8; 8] = n.as_secs().to_be_bytes();
            buffer.extend(secs[4..=7].to_vec());
            buffer.put_u32(0);

            buffer.freeze()
        },
        Err(_err) => Bytes::from([0u8; 8].to_vec()),
    }
}

impl From<NfsLookupReply> for Bytes {
    fn from(reply: NfsLookupReply) -> Bytes {
        let mut buffer = BytesMut::with_capacity(104);

        buffer.extend(Bytes::from(reply.status));
        buffer.extend(Bytes::from(reply.fhandle));
        buffer.extend(Bytes::from(reply.attributes));

        buffer.freeze()
    }
}

impl From<NfsFileAttributes> for Bytes {
    fn from(attributes: NfsFileAttributes) -> Bytes {
        let mut buffer = BytesMut::new();

        buffer.extend(Bytes::from(attributes._type));
        buffer.extend(Bytes::from(attributes.mode));
        buffer.put_u32(attributes.nlink);
        buffer.put_u32(attributes.uid);
        buffer.put_u32(attributes.gid);
        buffer.put_u32(attributes.size);
        buffer.put_u32(attributes.blocksize);
        buffer.put_u32(attributes.rdev);
        buffer.put_u32(attributes.blocks);
        buffer.put_u32(attributes.fsid);
        buffer.put_u32(attributes.file_id);
        buffer.extend(system_time_to_bytes(attributes.atime));
        buffer.extend(system_time_to_bytes(attributes.mtime));
        buffer.extend(system_time_to_bytes(attributes.ctime));

        buffer.freeze()
    }
}

#[derive(Debug, PartialEq)]
pub struct FileMode {
    name: u8,
    user: u8,
    group: u8,
    other: u8,
}

impl From<FileMode> for Bytes {
    fn from(file_mode: FileMode) -> Bytes {
        let mut buffer = BytesMut::with_capacity(4);
        buffer.put_u8(file_mode.name);
        buffer.put_u8(file_mode.user);
        buffer.put_u8(file_mode.group);
        buffer.put_u8(file_mode.other);
        buffer.freeze()
    }
}

#[derive(Debug, PartialEq)]
enum FileType {
    File, // 1u32
    Directory, // 2u32
}

impl From<FileType> for Bytes {
    fn from(file_type: FileType) -> Self {
        let mut buffer = BytesMut::with_capacity(4);
        buffer.put_u32(match file_type {
            FileType::File => 1u32,
            FileType::Directory => 2u32,
        });
        buffer.freeze()
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

        Bytes::from(reply_state_value.to_be_bytes().to_vec())
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

        Bytes::from(reply_state_value.to_be_bytes().to_vec())
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
            _ => Err(parse_error(input, Switch)),
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

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum RpcProgram {
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
            _ => Err(nom::Err::Failure(nom::error::Error::new(input, Switch))),
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
            _ => Err(parse_error(input, Switch)),
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
    NfsGetAttr(NfsGetAttr),
    NfsLookup(NfsLookup),
    NfsRead(NfsRead),
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
            (RpcProgram::Portmap, _)    => Err(parse_error(input, Switch)),
            (RpcProgram::Nfs, 1) => {
                let (input, data) = NfsGetAttr::decode(input)?;
                Ok((input, RpcProcedure::NfsGetAttr(data)))
            },
            (RpcProgram::Nfs, 4) => {
                let (input, data) = NfsLookup::decode(&input)?;
                Ok((input, RpcProcedure::NfsLookup(data)))
            },
            (RpcProgram::Nfs, 6) => {
                let (input, data) = NfsRead::decode(&input)?;
                Ok((input, RpcProcedure::NfsRead(data)))
            }
            (RpcProgram::Nfs, _)        => Err(parse_error(input, Switch)),
            (RpcProgram::Mount, 5u32)   => Ok((input, RpcProcedure::MountExport)),
            (RpcProgram::Mount, 1u32)   => {
                let (input, data) = MountMnt::decode(&input)?;
                Ok((input, RpcProcedure::MountMnt(data)))
            },
            (RpcProgram::Mount, _)      => Err(parse_error(input, Switch)),
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
            Err(_err) => Err(parse_error(input, Switch)),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FileHandle {
    data: Vec<u8>,
}

impl FileHandle {
    pub fn new(data: [u8; 32]) -> Self {
        Self {
            data: data.to_vec(),
        }
    }

    pub fn ino(&self) -> u64 {
        let mut data = [0u8; 8];
        for (index, value) in self.data[0..=7].into_iter().enumerate() {
            data[index] = *value;
        }
        u64::from_ne_bytes(data)
    }
}

impl Decoder for FileHandle {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, fhandle) = count(be_u8, 32)(input)?;

        Ok((input, Self { data: fhandle }))
    }
}

impl From<FileHandle> for Bytes {
    fn from(fhandle: FileHandle) -> Bytes {
        Bytes::from(fhandle.data)
    }
}

#[derive(Debug, PartialEq)]
pub struct NfsRead {
    pub fhandle: FileHandle,
    pub offset: u32,
    pub count: u32,
    pub total_count: u32,
}

impl Decoder for NfsRead {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, fhandle) = FileHandle::decode(input)?;
        let (input, offset) = be_u32(input)?;
        let (input, count) = be_u32(input)?;
        let (input, total_count) = be_u32(input)?;

        Ok((input, NfsRead {
            fhandle,
            offset,
            count,
            total_count,
        }))
    }
}

#[derive(Debug, PartialEq)]
pub struct NfsGetAttr {
    pub fhandle: FileHandle,
}

impl Decoder for NfsGetAttr {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, fhandle) = FileHandle::decode(input)?;

        Ok((input, NfsGetAttr {
            fhandle,
        }))
    }
}

#[derive(Debug, PartialEq)]
pub struct NfsFileAttributes {
    _type: FileType,
    mode: FileMode,
    nlink: u32,
    uid: u32,
    gid: u32,
    size: u32,
    blocksize: u32,
    rdev: u32,
    blocks: u32,
    fsid: u32,
    file_id: u32,
    atime: SystemTime,
    mtime: SystemTime,
    ctime: SystemTime,
}

#[derive(Debug, PartialEq)]
pub struct NfsGetAttrReply {
    pub status: NfsStatus,
    pub attributes: NfsFileAttributes,
}

impl From<NfsGetAttrReply> for Bytes {
    fn from(reply: NfsGetAttrReply) -> Bytes {
        let mut buffer = BytesMut::new();

        buffer.extend(Bytes::from(reply.status));
        buffer.extend(Bytes::from(reply.attributes));

        buffer.freeze()
    }
}

impl From<Metadata> for NfsGetAttrReply {
    fn from(input: Metadata) -> NfsGetAttrReply {
        NfsGetAttrReply {
            status: NfsStatus::Ok,
            attributes: NfsFileAttributes::from(input),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct NfsLookup {
    pub filename: PathBuf,
    pub fhandle: FileHandle,
}

impl NfsLookup {
    pub fn filename(&self) -> &PathBuf {
        &self.filename
    }
}

impl Decoder for NfsLookup {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, file_handle) = FileHandle::decode(input)?;

        let (input, length) = be_u32(input)?;
        let (input, contents) = count(le_u16, length as usize / 2)(input)?;

        let contents = String::from_utf16(&contents)
            .map_err(|_| parse_error(input, MapRes))?;

        Ok((input, NfsLookup {
            filename: Path::new(&contents).to_path_buf(),
            fhandle: file_handle,
        }))
    }
}

#[derive(Debug, PartialEq)]
pub struct PortmapGetport {
    version: u32,
    program: RpcProgram,
    protocol: PortmapProtocol,
    port: u32,
}

impl PortmapGetport {
    pub fn program(&self) -> &RpcProgram {
        &self.program
    }
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

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum PortmapProtocol {
    Ip,
    Udp,
}

impl Decoder for PortmapProtocol {
    type Output = Self;

    fn decode(input: &[u8]) -> IResult<&[u8], Self::Output> {
        let (input, protocol) = be_u32(input)?;

        match protocol {
            17u32 => Ok((input, PortmapProtocol::Udp)),
            _ => Err(parse_error(input, Switch)),
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
            _ => Err(parse_error(input,Switch)),
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
                        fhandle: FileHandle::new([0x00; 32]),
                    },
                ),
            }),
        }));
    }

    #[test]
    fn it_can_decode_lookup_call() {
        let call = Bytes::from(b"\0\0\0\"\0\0\0\0\0\0\0\x02\0\x01\x86\xa3\0\0\0\x02\0\0\0\x04\0\0\0\x01\0\0\0\x14\xf0\xbcq\x07\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x03\x01\0\0\0\0\x1bX\0\0\0\0\x11\x04\x01\0\0\0\0\x05\0\0\0\nU\0s\0e\0r\0s\0\0\0".to_vec());
        let nfs_lookup = RpcMessage::decode(&call);
        assert_eq!(nfs_lookup.is_ok(), true);
    }

    #[test]
    fn it_can_encode_nfs_lookup_reply() {
        let reply = NfsLookupReply {
            attributes: NfsFileAttributes {
                _type: FileType::File,
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(104, Bytes::from(reply).len());
    }
}

