use std::thread;
use std::sync::mpsc;

use crate::discovery::listener::DiscoveryListener;
use crate::player::{Player};

#[derive(Debug)]
pub enum Event {
    Annoncement(Player),
    Error(String),
}

pub struct Events {
    rx: mpsc::Receiver<Event>,
    handler: thread::JoinHandle<()>,
}

impl Events {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        let player_discovery_channel = {
            let discover_listener = DiscoveryListener::new();
            let tx = tx.clone();
            thread::spawn(move || loop {
                tx.send(discover_listener.receive()).unwrap()
            })
        };

        Self {
            rx: rx,
            handler: player_discovery_channel,
        }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }
}
