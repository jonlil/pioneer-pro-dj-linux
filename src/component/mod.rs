use std::thread;
use std::sync::mpsc::{channel, Receiver};
use crate::rekordbox::{Server, Database, Event};
use std::path::Path;

pub struct App {
    rekordbox_server: Server,
    rx: Receiver<Event>,
}

impl App {
    pub fn new<T: AsRef<Path>>(path: T) -> Self {
        let (tx, rx) = channel::<Event>();
        let database = Database::new(path);

        let rekordbox_server = Server::new(
            database,
            tx,
        );

        App {
            rekordbox_server,
            rx,
        }
    }

    pub async fn run(&mut self) {
        self.rekordbox_server.run().await;

        loop {
            self.next();
            thread::sleep(std::time::Duration::from_millis(150));
        }
    }

    fn initiate_linking(&self) {
        self.rekordbox_server.initiate_mac_ip_negotiation().unwrap();
    }

    fn next(&self) {
        match self.rx.recv() {
            Ok(event) => {
                match event {
                    Event::InitiateLink => self.initiate_linking(),
                    _ => eprintln!("Received no-op event: {:?}", event),
                };
            },
            Err(_) => {},
        };

    }
}
