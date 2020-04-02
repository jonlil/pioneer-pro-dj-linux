use std::sync::Arc;

use crate::rekordbox::{Artist};

type Database = Arc<crate::rekordbox::library::Database>;

pub fn number_of_artists(database: &Database) -> u32 {
    database.artists().len() as u32
}

pub fn number_of_tracks_by_artist(artist_id: u32, database: &Database) -> u32 {
    database.title_by_artist(artist_id).len() as u32
}

pub fn find_artist(artist_id: u32, database: &Database) -> Option<Artist> {
    database.get_artist(artist_id)
}
