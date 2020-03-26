use std::path::PathBuf;

#[derive(Debug)]
pub struct Metadata {
    pub artist: String,
    pub title: String,
    pub bpm: Option<u32>,
    pub album: String,
}

#[derive(Debug)]
pub struct MetadataTrack {
    pub metadata: Metadata,
    pub path: PathBuf,
    pub size: u32,
}

impl MetadataTrack {
    pub fn new(metadata: Metadata, path: PathBuf, size: u32) -> Self {
        Self {
            metadata,
            path,
            size,
        }
    }
}
