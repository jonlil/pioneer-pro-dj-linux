extern crate rand;

use std::net::{UdpSocket, ToSocketAddrs};
use std::sync::{Arc, Mutex, mpsc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread;
use std::time::Duration;
use rand::Rng;

use crate::rekordbox::message as Message;
use crate::utils::network::{PioneerNetwork, find_interface};
use crate::rekordbox::event;
use crate::rekordbox::event::EventHandler as EventParser;
use crate::rekordbox::player::{PlayerCollection};

pub enum Error {
    Generic(String),
    Socket(String),
}

// ClientState
//
// Provides thread safe access to stateful properties for Rekordbox::Client
pub struct ClientState {
    linking: bool,

    // Network to send Rekordbox messages to
    address: Option<PioneerNetwork>,

    // TODO: implement mutable accessor method #mut_players
    pub players: PlayerCollection,
}

impl ClientState {
    pub fn new() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new( ClientState {
            linking: false,
            address: None,
            players: PlayerCollection::new(),
        } ))
    }

    pub fn players(&self) -> &PlayerCollection {
        &self.players
    }

    pub fn set_linking(&mut self, value: bool) {
        self.linking = value;
    }

    pub fn is_linking(&self) -> bool {
        self.linking
    }

    pub fn set_address(&mut self, address: PioneerNetwork) {
        self.address = Some(address);
    }

    pub fn address(&self) -> &Option<PioneerNetwork> {
        &self.address
    }
}

pub trait EventHandler {
    fn on_event(&self, event: event::Event);
}

pub struct Client {
    state: Arc<RwLock<ClientState>>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            state: ClientState::new(),
        }
    }

    pub fn initiate_link<T: EventHandler>(&self, handler: &T, state: &mut ClientState, address: &PioneerNetwork) {
        state.set_linking(true);
        handler.on_event(event::Event::InitiateLink);
        let thread_sleep = Duration::from_millis(50);
        for sequence in 0x01 ..= 0x03 {
            random_broadcast_socket(&address, Message::DiscoveryInitial::new(&address, sequence).into());
            thread::sleep(thread_sleep);
        }
        for sequence in 0x01..=0x06 {
            for index in 1..=6 {
                random_broadcast_socket(&address, Message::DiscoverySequence::new(&address, sequence, index).into());
                thread::sleep(thread_sleep);
            }
        }
        random_broadcast_socket(&address, Message::ApplicationBroadcast::new(&address).into());
    }

    pub fn state(&self) -> Arc<RwLock<ClientState>> {
        self.state.clone()
    }

    pub fn run<T: EventHandler>(&mut self, handler: &T) -> Result<(), Error> {
        let (tx, rx) = mpsc::channel::<event::Event>();

        let socket = UdpSocket::bind(("0.0.0.0", 50000))
                              .map_err(|err| Error::Socket(format!("{}", err)))?;
        socket.set_broadcast(true)
              .map_err(|err| Error::Generic(format!("{}", err)))?;

        let message_socket = UdpSocket::bind(("0.0.0.0", 50002))
            .map_err(|err| Error::Generic(format!("{}", err)))?;
        message_socket.set_nonblocking(true).unwrap();

        let message_socket_ref = Arc::new(Mutex::new(message_socket));

        let _broadcast_handler = {
            let tx = tx.clone();
            thread::spawn(move || loop {
                let mut buffer = [0u8; 512];
                match socket.recv_from(&mut buffer) {
                    Ok((number_of_bytes, source)) => {
                        tx.send(EventParser::parse(&buffer[..number_of_bytes], (number_of_bytes, source))).unwrap();
                    },
                    Err(_) => (),
                }
            });
        };

        // This broadcast handler annonces this applications presense on the network.
        let _broadcast_sender_handler = {
            let state_ref = self.state();
            thread::spawn(move || {
                loop {
                    if let Ok(state) = state_ref.read() {
                        if let Some(address) = &state.address() {
                            random_broadcast_socket(address,
                               Message::ApplicationBroadcast::new(address).into());
                        }
                    }

                    thread::sleep(Duration::from_millis(500));
                }
            });
        };

        let _message_handler = {
            let _tx = tx.clone();

            // Clone message_socket_ref for thread safe access
            let socket = message_socket_ref.clone();

            thread::spawn(move || {
                loop {
                    let mut buffer = [0u8; 512];
                    match socket.lock().unwrap().recv_from(&mut buffer) {
                        Ok((number_of_bytes, source)) => {
                            if number_of_bytes != 284 {
                                eprintln!(
                                    "source: {:?}, nob: {}\n{:?}",
                                    source,
                                    number_of_bytes,
                                    String::from_utf8_lossy(&buffer[..number_of_bytes])
                                );
                            }
                        },
                        Err(_) => (),
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            });
        };

        let _portmap_handler = {
            thread::spawn(move || {
                let socket = UdpSocket::bind(("0.0.0.0", 50111)).unwrap();

                loop {
                    eprintln!("PORTMAP HANDLER");
                    let mut buffer = [0u8; 512];
                    match socket.recv_from(&mut buffer) {
                        Ok((number_of_bytes, _source)) => {
                            eprintln!("portmap package\n{:?}",
                                      String::from_utf8_lossy(&buffer[..number_of_bytes]));
                        },
                        Err(err) => eprintln!("{:?}", err)
                    }
                    thread::sleep(Duration::from_millis(300));
                }
            });
        };

        loop {
            match rx.recv() {
                Ok(mut evt) => {
                    match &mut evt {
                        event::Event::PlayerBroadcast(player) => {
                            match self.state().write() {
                                Ok(mut state) => {
                                    if let Some(address) = find_interface(player.address()) {
                                        if state.is_linking() == false && state.players().len() >= 2 {
                                            self.initiate_link(handler, &mut state, &address);
                                        }
                                        state.set_address(address);
                                        state.players.add_or_update(player.to_owned());
                                    }
                                },
                                Err(_) => {}
                            }
                        },
                        _ => (),
                    }

                    if evt != event::Event::ApplicationBroadcast {
                        handler.on_event(evt);
                    }
                },
                Err(_) => {},
            }
            thread::sleep(Duration::from_millis(300));
        }
    }
}

fn send_data<A: ToSocketAddrs>(
    socket: &UdpSocket,
    addr: A,
    data: Message::RekordboxMessageType
) {
    match socket.send_to(&data.as_ref(), addr) {
        Err(err) => eprintln!("{:?}", err.to_string()),
        _ => (),
    }
}

fn random_broadcast_socket(address: &PioneerNetwork, data: Message::RekordboxMessageType) {
    let port = rand::thread_rng().gen_range(45000, 55000);
    let socket = UdpSocket::bind((address.ip(), port)).unwrap();
    socket.set_broadcast(true).unwrap();
    send_data(&socket, (address.broadcast(), 50000), data);
}
