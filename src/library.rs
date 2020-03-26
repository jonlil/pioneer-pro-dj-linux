use walkdir::{DirEntry, WalkDir};
use std::path::{Path, PathBuf};
use id3::{Tag, v1 as id3v1};
use std::fs::{File, metadata};
use std::os::unix::fs::MetadataExt;
use std::ffi::OsStr;
use std::io;
use crate::rekordbox::{
    Metadata,
    MetadataTrack as Track,
};

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with("."))
         .unwrap_or(false)
}

fn is_regular_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
}

fn has_mp3_extension(entry: &DirEntry) -> bool {
    entry.path().extension() == Some(OsStr::new("mp3"))
}

fn mp3_files_iterator<T: AsRef<Path>>(t: T) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(t)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|e| e.ok())
        .filter(is_regular_file)
        .filter(has_mp3_extension)
}

fn extract_bpm(tag: &Tag) -> Option<u32> {
    match tag.get("TBPM") {
        Some(frame) => {
            match frame.content() {
                id3::Content::Text(text) => {
                    match text.parse::<u32>() {
                        Ok(value) => Some(value * 100),
                        _ => None,
                    }
                },
                _ => None,
            }
        },
        _ => None,
    }
}

fn extract_id3v2(tag: Tag) -> Metadata {
    Metadata {
        artist: tag.artist().unwrap_or("").to_string(),
        title: tag.title().unwrap_or("").to_string(),
        bpm: extract_bpm(&tag),
        album: tag.album().unwrap_or("").to_string(),
    }
}

fn extract_id3v1<'a>(tag: id3v1::Tag) -> Metadata {
    Metadata {
        artist: tag.artist,
        title: tag.title,
        bpm: None,
        album: tag.album,
    }
}

#[derive(Debug)]
pub enum LibraryError {
    ParseError,
    FileNotFound,
    Other(std::io::ErrorKind),
}

impl From<LibraryError> for std::io::Error {
    fn from(_error: LibraryError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "Parse error")
    }
}

impl From<std::io::Error> for LibraryError {
    fn from(error: std::io::Error) -> LibraryError {
        match error.kind() {
            io::ErrorKind::NotFound => LibraryError::FileNotFound,
            rest => LibraryError::Other(rest),
        }
    }
}

fn metadata_extractor(entry: DirEntry) -> Option<(Metadata, PathBuf, u32)> {
    let extracted_metadata = match Tag::read_from_path(entry.path()) {
        Ok(tag) => extract_id3v2(tag),
        Err(_) => {
            match File::open(entry.path()) {
                Ok(file) => {
                    match id3v1::Tag::read_from(file) {
                        Ok(tag) => extract_id3v1(tag),
                        Err(_err) => return None,
                    }
                },
                Err(_) => return None,
            }
        },
    };

    match metadata(entry.path()) {
        Ok(attributes) => {
            Some((
                extracted_metadata,
                entry.path().to_path_buf(),
                attributes.size() as u32,
            ))
        },
        _ => None,
    }
}

pub fn scan_folder<T: AsRef<Path>>(path: T) -> Vec<Track> {
    mp3_files_iterator(path)
        .filter_map(metadata_extractor)
        .map(|(metadata, path, file_size)| Track::new(metadata, path, file_size))
        .collect()
}
