extern crate rand;

use std::net::{UdpSocket, ToSocketAddrs, SocketAddr};
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread::{self, JoinHandle};
use std::io::ErrorKind;
use std::time::Duration;
use rand::Rng;

use crate::rekordbox::message as Message;
use crate::utils::network::{PioneerNetwork, find_interface};
use super::event::{self, Event, EventParser};
use crate::rekordbox::player::{PlayerCollection};
use crate::rekordbox::{APPLICATION_NAME, SOFTWARE_IDENTIFICATION};
use super::rpc::EventHandler as RPCEventHandler;
use crate::rpc::server::{RPCServer};

pub enum Error {
    Generic(String),
    Socket(String),
}

// ClientState
//
// Provides thread safe access to stateful properties for Rekordbox::Client
pub struct ClientState {
    // If the performer has pressed the button to start the linking phase.
    linking: bool,

    // If we have discovered rekordbox compatibile network devices that we have
    // have in a recent time responded to.
    discovery: bool,

    // True when the linking & discovery phases have completed
    linked: bool,

    // Network to send Rekordbox messages to
    address: Option<PioneerNetwork>,

    // TODO: implement mutable accessor method #mut_players
    pub players: PlayerCollection,
}

// TODO: Implement macro for llvm generation of getter and setters
impl ClientState {
    pub fn new() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new( ClientState {
            address: None,
            discovery: false,
            linked: false,
            linking: false,
            players: PlayerCollection::new(),
        } ))
    }

    pub fn players(&self) -> &PlayerCollection {
        &self.players
    }

    pub fn set_linking(&mut self, value: bool) {
        self.linking = value;
    }

    pub fn set_discovery(&mut self, value: bool) {
        self.discovery = value;
    }

    pub fn is_discovery(&self) -> bool {
        self.discovery
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

type LockedClientState = Arc<RwLock<ClientState>>;
type LockedUdpSocket = Arc<Mutex<UdpSocket>>;

pub trait EventHandler {
    fn on_event(&self, event: event::Event);
}

pub struct Client {
    state: LockedClientState,
}

impl Client {
    pub fn new() -> Self {
        Self {
            state: ClientState::new(),
        }
    }

    pub fn initiate_discovery<T: EventHandler>(&self, handler: &T, state: &mut ClientState, address: &PioneerNetwork) {
        state.set_discovery(true);
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

    pub fn state(&self) -> LockedClientState {
        self.state.clone()
    }

    fn broadcast_handler(socket: UdpSocket, tx: &Sender<Event>) -> JoinHandle<()> {
        let tx = tx.clone();

        thread::spawn(move || loop {
            let mut buffer = [0u8; 512];
            match socket.recv_from(&mut buffer) {
                Ok(socket_metadata) => Self::event_parser(&buffer, socket_metadata, &tx),
                Err(_) => (),
            }
            thread::sleep(Duration::from_millis(250));
        })
    }

    fn broadcast_sender_handler(state_ref: LockedClientState) -> JoinHandle<Event> {
        thread::spawn(move || {
            loop {
                // TODO: evaluate if the this is required to read fresh data.
                //       Otherwise it would be a good idea to move the read
                //       call outside of the loop scope.
                if let Ok(state) = state_ref.read() {
                    if let Some(address) = &state.address() {
                        random_broadcast_socket(
                            address,
                            Message::ApplicationBroadcast::new(address).into()
                        );
                    }
                }

                thread::sleep(Duration::from_millis(500));
            }
        })
    }

    // This handler should be able to receive messages from the parent thread
    // It may also be good if it had support for unwraping events that it just should respond to.
    fn message_handler(socket: LockedUdpSocket, tx: Sender<Event>) -> JoinHandle<Event> {
        thread::spawn(move || {
            loop {
                let mut buffer = [0u8; 512];
                // The lock is fine here since the socket is set to non_blocking
                match socket.lock().unwrap().recv_from(&mut buffer) {
                    Ok(metadata) => Self::event_parser(&buffer, metadata, &tx),

                    // Since this socket is non_blocking we might receive OS Errors (resource
                    // not available etc.) The error kind matcher reduces the logging of that.
                    Err(ref err) if err.kind() != ErrorKind::WouldBlock => {
                        println!("Something went wrong: {}", err)
                    },
                    // Don't bother
                    _ => {},
                }
                thread::sleep(Duration::from_millis(150));
            }
        })
    }

    // TODO: Break out this to RPC::Server
    // RPC::Server should have it's own EventLoop
    fn rpc_server_handler() {
        thread::spawn(move || {
            let server = RPCServer::new();
            let handler = RPCEventHandler::new();
            server.run(handler);
        });
    }

    fn next<T: EventHandler>(
        &mut self,
        rx: &Receiver<Event>,
        socket_ref: &LockedUdpSocket,
        handler: &T
    ) {
        match rx.recv() {
            Ok(mut evt) => {
                match &mut evt {
                    Event::PlayerBroadcast(player) => {
                        match self.state().write() {
                            Ok(mut state) => {
                                if let Some(address) = find_interface(player.address()) {
                                    if state.is_discovery() == false && state.players().len() >= 2 {
                                        self.initiate_discovery(handler, &mut state, &address);
                                    }
                                    state.set_address(address);
                                }

                                // Always update player on broadcast events.
                                // DJs might configure their players during performances.
                                state.players.add_or_update(player.to_owned());
                            },
                            Err(_) => {}
                        }
                    },
                    Event::PlayerLinkingWaiting(player) => {
                        match socket_ref.lock() {
                            Ok(socket) => {
                                let message: Vec<u8> = Message::InitiateRPCState::new().into();

                                match socket.send_to(
                                    &message.as_ref(),
                                    (player.address(), 50002),
                                ) {
                                    Ok(nob) => {
                                        // This should be a package of 48 bytes
                                        eprintln!("sent package to player with bytes: {}", nob);
                                        match self.state().write() {
                                            Ok(mut state) => {
                                                player.set_linking(true);
                                                state.players.add_or_update(player.to_owned());
                                            },
                                            Err(err) => {
                                                eprintln!("{}", err.to_string());
                                            },
                                        }
                                    },
                                    _ => {},
                                }
                            },
                            Err(_) => {},
                        }
                    },
                    _ => {},
                }

                // Filter out our own broadcasts
                if evt != Event::ApplicationBroadcast {
                    handler.on_event(evt);
                }
            },
            Err(_) => {},
        };
    }

    pub fn run<T: EventHandler>(&mut self, handler: &T) -> Result<(), Error> {
        let (tx, rx) = mpsc::channel::<Event>();

        let socket = UdpSocket::bind(("0.0.0.0", 50000))
            .map_err(|err| Error::Socket(format!("{}", err)))?;
        socket.set_broadcast(true)
            .map_err(|err| Error::Generic(format!("{}", err)))?;

        // Non-blocking thread safe UdpSocket
        // TODO: Implement poison management
        let message_socket = UdpSocket::bind(("0.0.0.0", 50002))
            .map_err(|err| Error::Generic(format!("{}", err)))?;
        message_socket.set_nonblocking(true).unwrap();
        let message_socket_ref: LockedUdpSocket = Arc::new(Mutex::new(message_socket));

        let _broadcast_handler = Self::broadcast_handler(socket, &tx);
        // This broadcast handler annonces this applications presense on the network.
        let _broadcast_sender_handler = Self::broadcast_sender_handler(self.state());

        // This handler is responsible for reading packages arriving on port 50002
        let _message_handler = Self::message_handler(message_socket_ref.clone(), tx.clone());

        let _rpc_server_handler = Self::rpc_server_handler();

        loop {
            self.next(&rx, &message_socket_ref, handler);
            thread::sleep(Duration::from_millis(300));
        }
    }

    fn event_parser(buffer: &[u8], metadata: (usize, SocketAddr), sender: &Sender<Event>) {
        sender.send(EventParser::parse(&buffer[..metadata.0], metadata)).unwrap();
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
