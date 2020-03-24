use std::path::PathBuf;

#[derive(Debug)]
pub struct Metadata {
    pub artist: String,
    pub title: String,
    pub bpm: u16,
    pub album: String,
}

#[derive(Debug)]
pub struct MetadataTrack {
    pub metadata: Metadata,
    pub path: PathBuf,
}

impl MetadataTrack {
    pub fn new(metadata: Metadata, path: PathBuf) -> Self {
        Self {
            metadata,
            path,
        }
    }
}
