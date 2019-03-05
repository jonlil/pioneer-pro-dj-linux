use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::net::{UdpSocket, IpAddr};
use std::sync::{Arc, Mutex};

use termion::event::Key;
use termion::input::TermRead;

use crate::discovery::listener::{DiscoveryListener};
use crate::discovery::event::{Event as DiscoveryEvent};
use crate::player::{PlayerCollection, Player};

pub enum Event {
    Input(Key),
    Tick,
    Discovery(DiscoveryEvent),
    Message(String),
}

/// A small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
    input_handle: thread::JoinHandle<()>,
    tick_handle: thread::JoinHandle<()>,
    player_discovery_handle: thread::JoinHandle<()>,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub exit_key: Key,
    pub tick_rate: Duration,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            exit_key: Key::Char('q'),
            tick_rate: Duration::from_millis(250),
        }
    }
}

impl Events {
    pub fn new() -> Events {
        Events::with_config(Config::default())
    }

    pub fn with_config(config: Config) -> Events {
        let (tx, rx) = mpsc::channel();
        let input_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let stdin = io::stdin();
                for evt in stdin.keys() {
                    match evt {
                        Ok(key) => {
                            if let Err(_) = tx.send(Event::Input(key)) {
                                return;
                            }
                            if key == config.exit_key {
                                return;
                            }
                        }
                        Err(_) => {}
                    }
                }
            })
        };
        let tick_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let tx = tx.clone();
                loop {
                    tx.send(Event::Tick).unwrap();
                    thread::sleep(config.tick_rate);
                }
            })
        };
        let player_discovery_handle = {
            let discover_listener = DiscoveryListener::new();
            let tx = tx.clone();
            thread::spawn(move || loop {
                match discover_listener.receive() {
                    DiscoveryEvent::Annoncement(player) => {
                        tx.send(Event::Discovery(DiscoveryEvent::Annoncement(player))).unwrap();
                    },
                    DiscoveryEvent::Error(message) => tx.send(Event::Message(message)).unwrap()
                }
            })
        };
        Events {
            rx,
            tx,
            input_handle,
            tick_handle,
            player_discovery_handle,
        }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }

    pub fn create_link_channel(
        &self,
        socket: &Arc<Mutex<UdpSocket>>
    ) -> thread::JoinHandle<()> {
        {
            let socket_ref = socket.clone();
            let tx = self.tx.clone();
            thread::spawn(move || loop {
                let mut socket = socket_ref.lock().unwrap();
                match Player::link(&mut socket) {
                    Ok(nob) => tx.send(Event::Message(nob.to_string())).unwrap(),
                    Err(error) => tx.send(Event::Message(error.to_string())).unwrap()
                };
                thread::sleep(Duration::from_millis(300));
            })
        }
    }
}
