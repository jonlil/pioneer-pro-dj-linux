use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::io::SeekFrom;
use std::path::Path;
use crate::rpc::packets::NfsDataWrapper;

#[derive(Debug)]
pub struct FileWrapper {
    pub inode: u64,
    pub encoded: [u8; 32],
    pub file: File,
}

pub fn get_fhandle<T: AsRef<Path>>(path: T, inode: u64) -> Result<FileWrapper, std::io::Error> {
    let file = File::open(path)?;
    let encoded_fhandle = encode_file_handler(&inode);
    Ok(FileWrapper {
        file,
        encoded: encoded_fhandle,
        inode,
    })
}

pub fn read_file_range(mut file: &File, start: u32, count: u32) -> Result<NfsDataWrapper, io::Error> {
    let mut buf: Vec<u8> = vec![0x00; count as usize];

    file.seek(SeekFrom::Start(start as u64))?;
    file.take(count as u64).read(&mut buf)?;

    Ok(NfsDataWrapper {
        data: buf,
    })
}

fn encode_file_handler(inode: &u64) -> [u8; 32] {
    let mut data = [0u8; 32];
    for (index, value) in inode.to_le_bytes().iter().enumerate() {
        data[index] = *value;
    }
    data
}
