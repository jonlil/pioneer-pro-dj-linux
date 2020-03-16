use std::path::Path;
use std::sync::{Arc, RwLock, RwLockWriteGuard, RwLockReadGuard};
use std::collections::HashMap;

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

    pub fn artists(&self) -> Vec<String> {
        let mut artists: HashMap<String, usize> = HashMap::new();
        let mut ids: usize = 0;
        self.read(&mut |reader| {
            for track in &reader.collection {
                let artist = track.metadata.artist.clone();
                if !artists.contains_key(&artist) {
                    ids += 1;
                    artists.insert(artist, ids);
                }
            }
        });

        let mut ret: Vec<String> = vec![];
        for key in artists.keys() {
            ret.push(key.to_string());
        }

        ret
    }

    pub fn title_by_artist(&self, artist: u32) -> Vec<String> {
        let mut titles: Vec<String> = vec![];
        self.read(&mut |reader| {
            for title in &reader.collection {
                titles.push(title.metadata.title.clone());
            }
        });
        titles
    }

    fn index(&self, track: Track) -> Result<(), DatabaseError> {
        self.write(|writer| {
            writer.add(track)
        })
    }

    fn tracks(&self) -> Result<Vec<Track>, DatabaseError> {
        Ok(vec![])
    }

    fn read<T>(&self, closure: &mut T)
    where
        T: FnMut(RwLockReadGuard<InnerDatabase>)
    {
        match self.inner.read() {
            Ok(reader) => closure(reader),
            Err(_err) => {},
        };
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
