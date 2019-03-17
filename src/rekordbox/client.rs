extern crate rand;

use std::net::{UdpSocket, ToSocketAddrs};
use std::sync::mpsc;
use std::thread;
use std::time;
use rand::Rng;

use crate::rekordbox::message as Message;
use crate::utils::network::{PioneerNetwork, find_interface};
use crate::rekordbox::event;
use crate::rekordbox::EventHandler as EventParser;

pub enum Error {
    Generic(String),
    Socket(String),
}

pub struct Client {
    rx: mpsc::Receiver<event::Event>,
    tx: mpsc::Sender<event::Event>,
    network: Option<PioneerNetwork>,
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
            network: None,
        }
    }

    pub fn run<T: EventHandler>(&mut self, handler: &mut T) -> Result<(), Error> {
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
                Ok(evt) => {
                    match &evt {
                        event::Event::PlayerBroadcast(player) => {
                            self.network = find_interface(player.address())
                        },
                        _ => ()
                    }

                    // Filter out ApplicationBroadcast (our own messages)
                    // Wonder if this could be set on the socket?
                    if evt != event::Event::ApplicationBroadcast {
                        handler.on_event(evt)
                    }
                },
                Err(error) => eprintln!("{:?}", error),
            }

            if let Some(network) = &self.network {
                random_broadcast_socket(&network,
                    Message::ApplicationBroadcast::new(&network).into());
            }

            thread::sleep(time::Duration::from_millis(250));
        }
    }
}

pub fn send_data<A: ToSocketAddrs>(
    socket: &UdpSocket,
    addr: A,
    data: Message::RekordboxMessageType
) {
    match socket.send_to(&data.as_ref(), addr) {
        //Ok(number_of_bytes) => eprintln!("{:?}", number_of_bytes),
        Err(err) => eprintln!("{:?}", err.to_string()),
        _ => (),
    }
}

pub fn random_broadcast_socket(address: &PioneerNetwork, data: Message::RekordboxMessageType) {
    let port = rand::thread_rng().gen_range(45000, 55000);
    let socket = UdpSocket::bind((address.ip(), port)).unwrap();
    socket.set_broadcast(true).unwrap();
    send_data(&socket, (address.broadcast(), 50000), data);
}
