use crate::utils::network::PioneerNetwork;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use crate::rekordbox;
use crate::rekordbox::event::Event as RekordboxEvent;
use crate::utils::network::find_interface;

pub enum Event {
    Tick,
}

pub struct Events {
    rx: mpsc::Receiver<Event>,
    input_handle: thread::JoinHandle<()>,
    tick_handle: thread::JoinHandle<()>,
}

impl Events {
    pub fn run() {
        let (tx, rx) = mpsc::channel();
        let tick_rate = Duration::from_millis(250);

        let tx = tx.clone();
        let tick_handler = {
            thread::spawn(move || {
                let tx = tx.clone();

                loop {
                    tx.send(Event::Tick).unwrap();
                    thread::sleep(tick_rate);
                }
            })
        };
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }
}

struct RekordboxEventHandler {
    tx: mpsc::Sender<RekordboxEvent>,
}

impl rekordbox::client::EventHandler for RekordboxEventHandler {
    fn on_event(&self, event: rekordbox::event::Event) {
        self.tx.send(event).unwrap();
    }
}

pub struct App {
    pub network: Option<PioneerNetwork>,
    pub players: rekordbox::player::PlayerCollection,
}

impl App {
    pub fn run(&mut self) -> Result<(), rekordbox::client::Error> {
        let (tx, rx) = mpsc::channel::<RekordboxEvent>();
        let mut rekordbox_client = rekordbox::client::Client::new();

        thread::spawn(move || {
            rekordbox_client.run(&mut RekordboxEventHandler { tx: tx });
        });

        loop {
            match rx.recv() {
                Ok(evnt) => {
                    match evnt {
                        RekordboxEvent::PlayerBroadcast(player) => {
                            self.players.add_or_update(player);
                        }
                        _ => { eprintln!("{:?}", evnt) }
                    }
                }
                _ => ()
            }

            eprintln!("{:?}", self.players);
        }
    }
}
