#![allow(dead_code)]

// TODO: Make private
pub const SOFTWARE_IDENTIFICATION: [u8; 10] = [
    0x51,0x73,0x70,0x74,0x31,0x57,0x6d,0x4a,0x4f,0x4c
];

// TODO: Make private
pub const APPLICATION_NAME: [u8; 20] = [
    0x4c,0x69,0x6e,0x75,0x78,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00
];

pub mod client;
pub mod event;
pub mod player;
pub mod util;

// Internal mods
mod message;
mod library;
mod packets;
mod state;
mod db_field;
mod db_request_type;
mod db_message_argument;
mod metadata_type;
mod rpc;
mod db_codec;
mod status_event_server;

// tests
#[cfg(test)]
mod fixtures;
