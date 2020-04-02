#![allow(dead_code)]

// TODO: Make private
pub const SOFTWARE_IDENTIFICATION: [u8; 10] = [
    0x51,0x73,0x70,0x74,0x31,0x57,0x6d,0x4a,0x4f,0x4c
];

// TODO: Make private
pub const APPLICATION_NAME: [u8; 20] = [
    0x4c,0x69,0x6e,0x75,0x78,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00
];

pub mod server;
pub mod player;
pub mod util;

// Internal mods
mod library;
mod packets;
mod db_field;
mod db_request_type;
mod db_message_argument;
mod rpc;
mod status_event_server;
mod keepalive;

// tests
#[cfg(test)]
mod fixtures;

pub trait EventHandler<T> {
    fn on_event(&self, event: T);
}

use status_event_server::StatusEventServer;
pub use server::{Server, ServerState};
pub use server::ApplicationEvent as Event;
use rpc::server as rpc_server;
use library::DBLibraryServer;
pub use packets::DBMessage;
pub use library::model::{MetadataTrack, Metadata};
pub use library::database::{Track, Artist, Record};
pub use library::database::Database;
