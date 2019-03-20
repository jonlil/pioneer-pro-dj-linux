use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use crate::rekordbox;
use crate::rekordbox::event::Event as RekordboxEvent;

struct RekordboxEventHandler {
    tx: mpsc::Sender<RekordboxEvent>,
}

impl rekordbox::client::EventHandler for RekordboxEventHandler {
    fn on_event(&self, event: rekordbox::event::Event) {
        self.tx.send(event).unwrap();
    }
}

pub struct App;

impl App {
    pub fn run(&mut self) -> Result<(), rekordbox::client::Error> {
        let (tx, rx) = mpsc::channel::<RekordboxEvent>();

        let mut rekordbox_client = rekordbox::client::Client::new();
        let rekordbox_state = rekordbox_client.state();

        let _rekordbox_handler = {
            let tx = tx.clone();
            thread::spawn(move || {
                let _result = rekordbox_client.run(&RekordboxEventHandler {
                    tx: tx,
                });
            })
        };


        let _tick_handler = {
            let tx = tx.clone();
            thread::spawn(move || {
                loop {
                    tx.send(RekordboxEvent::Tick).unwrap();
                    thread::sleep(Duration::from_millis(250));
                }
            });
        };

        loop {
            match rx.recv() {
                Ok(evnt) => {
                    match &evnt {
                        RekordboxEvent::PlayerBroadcast(player) => {
                            let state = rekordbox_state.read().unwrap();
                            eprintln!("{:?}", state.players());
                        }
                        _ => {}
                    }
                }
                _ => ()
            }

            // Hey, don't steal my CPU.
            thread::sleep(Duration::from_millis(250));
        }
    }
}
