use std::path::PathBuf;

#[derive(Debug)]
pub struct Metadata {
    pub artist: String,
    pub title: String,
    pub bpm: u16,
    pub album: String,
}

#[derive(Debug)]
pub struct Track {
    pub metadata: Metadata,
    pub path: PathBuf,
}

impl Track {
    pub fn new(metadata: Metadata, path: PathBuf) -> Self {
        Self {
            metadata,
            path,
        }
    }
}
