use std::thread;
use super::rekordbox::{Server, Database, Event};
use std::sync::mpsc::{channel, Receiver};

pub struct App {
    rekordbox_server: Server,
    rx: Receiver<Event>,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = channel::<Event>();
        let rekordbox_server = Server::new(Database::new(), tx);

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
