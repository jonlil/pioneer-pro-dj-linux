use std::fs::File;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::io;

use crate::rekordbox::Track;
use crate::library::scan_folder;

struct InnerDatabase {
    albums: Vec<String>,
    tracks: Vec<String>,
    artists: Vec<String>,
}

pub struct Database {
    inner: Arc<RwLock<InnerDatabase>>,
}

impl Database {
    pub fn new<T: AsRef<Path>>(root_folder: T) -> Self {
        let inner_db = InnerDatabase {
            albums: Vec::new(),
            tracks: Vec::new(),
            artists: Vec::new(),
        };

        let database = Self {
            inner: Arc::new(RwLock::new(inner_db)),
        };

        for track in scan_folder(&root_folder) {
            database.index(track);
        }

        database
    }

    fn index(&self, track: Track) -> Result<(), io::Error> {
        Ok(())
    }
}
