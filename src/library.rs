use walkdir::{DirEntry, WalkDir};
use std::path::{Path, PathBuf};
use id3::{Tag, v1 as id3v1};
use std::fs::File;
use std::ffi::OsStr;
use std::io;
use crate::rekordbox::{Metadata, Track};

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

fn extract_id3v2(tag: Tag) -> Metadata {
    Metadata {
        artist: tag.artist().unwrap_or("").to_string(),
        title: tag.title().unwrap_or("").to_string(),
        bpm: 0,
        album: tag.album().unwrap_or("").to_string(),
    }
}

fn extract_id3v1<'a>(tag: id3v1::Tag) -> Metadata {
    Metadata {
        artist: tag.artist,
        title: tag.title,
        bpm: 0,
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

fn metadata_extractor(entry: DirEntry) -> Option<(Metadata, PathBuf)> {
    let metadata = match Tag::read_from_path(entry.path()) {
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

    Some((metadata, entry.path().to_path_buf()))
}

pub fn scan_folder<T: AsRef<Path>>(path: T) -> Vec<Track> {
    mp3_files_iterator(path)
        .filter_map(metadata_extractor)
        .map(|(metadata, path)| Track::new(metadata, path))
        .collect()
}
