use std::net::UdpSocket;
use std::sync::mpsc;
use std::thread;

use crate::rekordbox::event;
use crate::rekordbox::EventHandler as EventParser;

pub enum Message {
    Bytes
}
pub enum Error {
    Generic(String),
    Socket(String),
}

pub struct Client {
    rx: mpsc::Receiver<event::Event>,
    tx: mpsc::Sender<event::Event>,
}

pub trait EventHandler {
    fn on_event(&self, event: event::Event);
}

impl Client {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<event::Event>();

        Self {
            tx: tx,
            rx: rx,
        }
    }

    pub fn run<T: EventHandler>(&self, handler: &mut T) -> Result<(), Error> {
        let socket = UdpSocket::bind(("0.0.0.0", 50000))
                              .map_err(|err| Error::Socket(format!("{}", err)))?;
        socket.set_broadcast(true)
              .map_err(|err| Error::Generic(format!("{}", err)))?;

        {
            let tx = self.tx.clone();
            thread::spawn(move || loop {
                let mut buffer = [0u8; 512];
                match socket.recv_from(&mut buffer) {
                    Ok((number_of_bytes, source)) => {
                        tx.send(EventParser::parse(&buffer[..number_of_bytes], (number_of_bytes, source))).unwrap();
                    },
                    Err(_) => (),
                }
            });
        }

        loop {
            match self.rx.recv() {
                Ok(eeh) => handler.on_event(eeh),
                Err(error) => eprintln!("{:?}", error),
            }
        }
    }
}
