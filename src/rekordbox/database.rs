use std::path::Path;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use crate::rekordbox::Track;
use crate::library::scan_folder;

#[derive(Debug)]
pub enum DatabaseError {
    Unknown,
}

#[derive(Debug)]
struct InnerDatabase {
    collection: Vec<Track>,
}

impl InnerDatabase {
    fn add(&mut self, track: Track) -> Result<(), DatabaseError> {
        self.collection.push(track);

        Ok(())
    }
}

#[derive(Debug)]
pub struct Database {
    inner: RwLock<InnerDatabase>,
}

impl Database {
    pub fn new<T: AsRef<Path>>(root_folder: T) -> Self {
        let inner_db = InnerDatabase {
            collection: vec![],
        };

        let database = Self {
            inner: RwLock::new(inner_db),
        };

        for track in scan_folder(&root_folder) {
            database.index(track);
        }

        database
    }

    fn index(&self, track: Track) -> Result<(), DatabaseError> {
        self.write(|writer| {
            writer.add(track)
        })
    }

    fn tracks(&self) -> Result<Vec<Track>, DatabaseError> {
        Ok(vec![])
    }

    fn write<T>(&self, closure: T) -> Result<(), DatabaseError>
    where
        T: FnOnce(&mut RwLockWriteGuard<InnerDatabase>) -> Result<(), DatabaseError>
    {
        match self.inner.write() {
            Ok(mut writer) => closure(&mut writer),
            Err(_err) => Err(DatabaseError::Unknown),
        }
    }
}
