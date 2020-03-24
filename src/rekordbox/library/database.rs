use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockWriteGuard, RwLockReadGuard, Mutex};
use std::collections::HashMap;
use std::ops::Add;

use crate::rekordbox::MetadataTrack;
use crate::library::scan_folder;

#[derive(Debug)]
pub enum DatabaseError {
    Unknown,
}

struct ArtistTable<T: Record> {
    rows: HashMap<u32, T>,
    sequence: Sequence<u32>,
}

struct TrackTable<T: Record> {
    rows: HashMap<u32, T>,
    sequence: Sequence<u32>,
}

struct NewTrack {
    artist_id: u32,
    title: String,
    path: PathBuf,
}

pub struct Track {
    id: u32,
    artist_id: u32,
    title: String,
    path: PathBuf,
}

struct NewArtist {
    name: String,
}

#[derive(Clone)]
pub struct Artist {
    id: u32,
    name: String,
}

pub trait Record {
    fn name(&self) -> &String;
    fn id(&self) -> &u32;
}

impl Record for Track {
    fn name(&self) -> &String {
        &self.title
    }

    fn id(&self) -> &u32 {
        &self.id
    }
}

impl Record for Artist {
    fn name(&self) -> &String {
        &self.name
    }

    fn id(&self) -> &u32 {
        &self.id
    }
}

impl Insertable<NewArtist, u32> for ArtistTable<Artist> {
    fn insert(&mut self, document: NewArtist) -> u32 {
        for (id, value) in self.rows.iter() {
            if document.name == value.name {
                return *id;
            }
        }

        match self.sequence.increment() {
            Ok(id) => {
                self.rows.insert(id.clone(), Artist {
                    id,
                    name: document.name,
                });
                return id;
            },
            Err(err) => panic!("Failed inserting document into ArtistTable; error = {}", err),
        };
    }
}

impl Insertable<NewTrack, u32> for TrackTable<Track> {
    fn insert(&mut self, document: NewTrack) -> u32 {
        match self.sequence.increment() {
            Ok(id) => {
                self.rows.insert(id.clone(), Track {
                    id,
                    artist_id: document.artist_id,
                    path: document.path,
                    title: document.title,
                });
                return id;
            },
            Err(err) => panic!("Failed inserting document into ArtistTable; error = {}", err),
        };
    }
}


impl<T: Record> ArtistTable<T> {
    fn new() -> Self {
        Self {
            rows: HashMap::new(),
            sequence: Sequence::new(),
        }
    }
}

impl<T: Record> TrackTable<T> {
    fn new() -> Self {
        Self {
            rows: HashMap::new(),
            sequence: Sequence::new(),
        }
    }
}

trait Insertable<T, A> {
    fn insert(&mut self, document: T) -> A;
}

trait Queryable<T>: PrimaryKey {
    fn query(&self, id: T);
}

struct Sequence<T: PrimaryKey> {
    counter: Arc<Mutex<T>>,
}

impl<T: PrimaryKey> Sequence<T> {
    fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(T::one())),
        }
    }
}

impl<T: PrimaryKey + Copy> Sequence<T> {
    fn increment(&mut self) -> Result<T, &'static str> {
        match self.counter.clone().lock() {
            Ok(mut value) => {
                *value = *value + T::one();
                Ok(*value)
            },
            Err(_) => Err("Allocation lock for id sequence failed."),
        }
    }
}

trait PrimaryKey: Clone + Add<Output=Self> + std::hash::Hash + Eq {
    fn one() -> Self;
}

impl PrimaryKey for u32 {
    fn one() -> Self { 1 }
}

struct InnerDatabase {
    artists: ArtistTable<Artist>,
    tracks: TrackTable<Track>,
}

impl InnerDatabase {
    fn add(&mut self, track: MetadataTrack) -> Result<(), DatabaseError> {
        //self.collection.push(track);

        Ok(())
    }
}

pub struct Database {
    inner: RwLock<InnerDatabase>,
}

impl Database {
    pub fn new<T: AsRef<Path>>(root_folder: T) -> Self {
        let inner_db = InnerDatabase {
            artists: ArtistTable::new(),
            tracks: TrackTable::new(),
        };

        let database = Self {
            inner: RwLock::new(inner_db),
        };

        for track in scan_folder(&root_folder) {
            database.index(track);
        }

        database
    }

    pub fn artists(&self) -> Vec<Artist> {
        let mut ret = vec![];
        self.read(&mut |reader| {
            for (id, artist) in &reader.artists.rows {
                ret.push(artist.clone());
            }
        });

        ret
    }

    pub fn title_by_artist(&self, artist: u32) -> Vec<String> {
        let mut titles: Vec<String> = vec![];
        self.read(&mut |reader| {
            for (id, track) in &reader.tracks.rows {
                titles.push(track.title.clone());
            }
        });
        titles
    }

    fn index(&self, track: MetadataTrack) -> Result<(), DatabaseError> {
        self.write(|db| {
            let artist_id = db.artists.insert(NewArtist {
                name: track.metadata.artist,
            });
            db.tracks.insert(NewTrack {
                artist_id,
                path: track.path,
                title: track.metadata.title,
            });

            Ok(())
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

#[test]
fn it_can_insert_artists() {
    struct SomeModel;

    impl Record for SomeModel {
        fn name() {}
        fn id() {}
    }

    let mut table: ArtistTable<SomeModel> = ArtistTable::new();

    table.insert(SomeModel);
    assert_eq!(2, *table.sequence.counter.clone().lock().unwrap());
    table.insert(SomeModel);
    assert_eq!(3, *table.sequence.counter.clone().lock().unwrap());
}
