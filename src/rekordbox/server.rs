use std::sync::{Arc, Mutex};
use std::thread::{self};
use std::sync::mpsc::Sender;
use std::time::Duration;
use bytes::Bytes;
use std::net::{UdpSocket, ToSocketAddrs};
use futures::{try_join, TryFutureExt};

use super::player::{PlayerCollection, Player};
use crate::utils::network::{PioneerNetwork, find_interface};
use crate::rekordbox::StatusEventServer;
use crate::rekordbox::DBLibraryServer;
use crate::rekordbox::rpc_server;
use crate::rekordbox::Database;
use super::keepalive::{
    Event as KeepAliveEvent,
    KeepAliveContentType,
    KeepAlivePacket,
    KeepAlivePacketType,
    KeepAliveServer,
    KeepAliveServerOptions,
    KeepAliveMacPackage,
    KeepAliveIpPackage,
    KeepAliveStatusPackage,
    Status,
};
use super::{EventHandler};

#[derive(Debug)]
pub enum ApplicationEvent {
    InitiateLink,
    DeviceChange,
}

#[derive(Debug)]
pub struct ServerState {
    linking: bool,
    discovery: bool,
    linked: bool,
    address: Option<PioneerNetwork>,
    players: PlayerCollection,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            linking: false,
            linked: false,
            discovery: false,
            address: None,
            players: PlayerCollection::new(),
        }
    }
}

pub struct Server {
    database: Arc<Database>,
    state: Arc<Mutex<ServerState>>,
    tx: Sender<ApplicationEvent>,
    broadcast_sleep_time: Duration,
}

impl Server {
    pub fn new(database: Database, tx: Sender<ApplicationEvent>) -> Self {
        let state = Arc::new(Mutex::new(ServerState::default()));
        let database = Arc::new(database);

        Server {
            database: database,
            state: state,
            tx: tx,
            broadcast_sleep_time: Duration::from_millis(50),
        }
    }

    pub async fn run(&self) {
        dbg!("server starting");

        broadcast_sender_handler(&self.state);
        keepalive_server(&self.tx, &self.state);
        let rpc_future = rpc_server(self.state.clone())
            .map_err(|_| "Unable to start RPC Server".to_string());
        let db_library_future = DBLibraryServer::run(self.state.clone(), self.database.clone())
            .map_err(|_| "Unable to start DBLibraryServer".to_string());
        match status_event_server(&self.tx, &self.state) {
            Err(err) => {
                dbg!(err);
            },
            _ => {},
        };
        dbg!("server started");

        tokio::spawn(async move {
            try_join!(
                rpc_future,
                db_library_future,
            )
        });
    }

    pub fn initiate_mac_ip_negotiation(&self) -> Result<(), &'static str> {
        dbg!("InitiateLink");

        let thread_sleep = Duration::from_millis(50);
        match self.state.lock() {
            Ok(mut state) => {
                state.linking = true;

                if let Some(address) = &state.address {
                    for sequence in 1 ..= 3 {
                        self.broadcast_message(
                            &address,
                            KeepAliveMacPackage::new(
                                sequence,
                                address.mac_address(),
                            )
                        );
                        thread::sleep(thread_sleep);
                    }

                    for sequence in 0x01 ..= 0x06 {
                        for index in 1..= 6 {
                            self.broadcast_message(&address, KeepAliveIpPackage::new(
                                sequence,
                                index,
                                address.ip(),
                                address.mac_address(),
                            ));
                            thread::sleep(thread_sleep);
                        }
                    }

                    self.broadcast_message(&address, KeepAliveStatusPackage::new(
                        address.ip(),
                        address.mac_address(),
                        4,
                        8,
                    ));

                    Ok(())
                } else {
                    return Err("No pioneer network has been discovered")
                }
            },
            Err(err) => {
                dbg!(err);
                Err("Failed initiate linking procedure")
            },
        }
    }

    fn broadcast_message(&self, address: &PioneerNetwork, message: KeepAlivePacket) {
        send_broadcast_payload(&address, message);
        thread::sleep(self.broadcast_sleep_time);
    }
}

fn status_event_server(
    tx: &Sender<ApplicationEvent>,
    state: &Arc<Mutex<ServerState>>,
) -> Result<(), &'static str> {
    let _tx = tx.clone();
    let _state = state.clone();

    let status_event_server = StatusEventServer::bind()?;

    thread::spawn(move || status_event_server.run());

    Ok(())
}

fn keepalive_server(tx: &Sender<ApplicationEvent>, state: &Arc<Mutex<ServerState>>) {
    let tx = tx.clone();
    let state = state.clone();

    let keepalive_listener = KeepAliveServer::bind(
        KeepAliveServerOptions::default(),
    ).expect("Failed to bind keepalive server");

    thread::spawn(move || {
        keepalive_listener.run(KeepaliveEventHandler {
            tx: &tx,
            state: state,
        });
    });
}

fn broadcast_sender_handler(state: &Arc<Mutex<ServerState>>) {
    let state = state.clone();

    thread::spawn(move || {
        loop {
            if let Ok(state) = state.lock() {
                if let Some(address) = &state.address {
                    send_broadcast_payload(&address, KeepAliveStatusPackage::new(
                        address.ip(),
                        address.mac_address(),
                        1,
                        0,
                    ));
                }
            }

            thread::sleep(Duration::from_millis(500));
        }
    });
}

/// Struct that acts on KeepAliveEvent.
///
/// It also provides an normalized interface to events via the Sender<ApplicationEvent>.
struct KeepaliveEventHandler<'a> {
    tx: &'a Sender<ApplicationEvent>,
    state: Arc<Mutex<ServerState>>,
}

impl<'a> EventHandler<KeepAliveEvent> for KeepaliveEventHandler<'a> {
    fn on_event(&self, event: KeepAliveEvent) {
        let (event, _peer) = event;
        match (&event.kind(), &event.content(), event.model().to_string() != &String::from("rekordbox")) {
            (KeepAlivePacketType::Status, KeepAliveContentType::Status(status), true) => {
                match self.state.clone().lock() {
                    Ok(mut state) => handle_keepalive_status(&event, status, &mut state, &self.tx),
                    Err(_) => {},
                };
            },
            _ => {},
        };
    }
}

fn handle_keepalive_status(
    event: &KeepAlivePacket,
    status: &Status,
    state: &mut ServerState,
    tx: &Sender<ApplicationEvent>,
) {
    let previous_number_of_players = state.players.len();
    state.synchronize_player_broadcast_network(find_interface(status.ip_addr()));
    state.players.add_or_update(Player::new(
        event.model().to_string().clone(),
        status.player_number().to_owned(),
        status.ip_addr().to_owned(),
    ));

    // TODO: Implement logic for reacting on all type of device changes
    if previous_number_of_players != state.players.len() {
        if let Err(err) = tx.send(ApplicationEvent::DeviceChange) {
            eprintln!("Failed to emit ApplicationEvent::DeviceChange with error: {:?}", err);
        }
    }

    if state.is_linking_possible() {
        if let Err(err) = tx.send(ApplicationEvent::InitiateLink) {
            eprintln!("Failed to emit ApplicationEvent::InitiateLink with error: {:?}", err);
        }
    }
}

impl ServerState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Method to call when a network containing rekordbox equipment has been identified
    fn synchronize_player_broadcast_network(&mut self, network: Option<PioneerNetwork>) {
        // This might not be a safe operation since it might force the application to join an other
        // hostile network. Would be pretty simple to use this as an attack vector.
        // TODO: Implement safe guard before allowing the network to change
        self.address = network;
    }

    fn is_linking_possible(&self) -> bool {
        if self.players.len() > 0 {
            if self.linking == false {
                return true;
            } else {
                return false;
            }
        }

        false
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

fn send_broadcast_payload<A: Into<Bytes>>(
    address: &PioneerNetwork,
    data: A,
) {
    let socket = UdpSocket::bind((address.ip(), 0)).unwrap();
    socket.set_broadcast(true).unwrap();

    send_data(&socket, (address.broadcast(), 50000), data.into())
}

fn send_data<A: ToSocketAddrs>(
    socket: &UdpSocket,
    addr: A,
    data: Bytes,
) {
    match socket.send_to(&data.as_ref(), addr) {
        Err(err) => eprintln!("{:?}", err.to_string()),
        _ => (),
    }
}
