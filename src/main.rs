extern crate rand;

use std::thread;
use std::sync::{Arc, Mutex, mpsc};
use std::net::{UdpSocket, ToSocketAddrs};
use std::io;
use std::time::{Duration};
use rand::Rng;

const SOFTWARE_IDENTIFICATION: [u8; 10] = [
    0x51,0x73,0x70,0x74,0x31,0x57,0x6d,0x4a,0x4f,0x4c
];

const APPLICATION_NAME: [u8; 20] = [
    0x4c,0x69,0x6e,0x75,0x78,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00
];

const MAC_ADDRESS: [u8; 6] = [
    0xac,0x87,0xa3,0x35,0xbc,0x4d
];

pub fn send_data<A: ToSocketAddrs>(
    socket: &UdpSocket,
    addr: A,
    data: PioneerMessage
) {


    match socket.send_to(&to_socket_package(data).as_ref(), addr) {
        Ok(number_of_bytes) => {
            eprintln!("{:?}", number_of_bytes);
        }
        Err(err) => {
            eprintln!("{:?}", err.to_string());
        }
    }
}

fn to_socket_package(item: Vec<Vec<u8>>) -> Vec<u8> {
    item.into_iter().flatten().collect::<Vec<u8>>()
}

pub struct BroadcastClient {
    socket: Arc<Mutex<UdpSocket>>
}

impl BroadcastClient {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Self {
        Self {
            socket: Arc::new(Mutex::new(broadcastable_socket(addr)))
        }
    }
}

fn random_broadcast_socket(address: &'static str) -> UdpSocket {
    let port = rand::thread_rng().gen_range(45000, 55000);
    let socket = UdpSocket::bind(format!("{}:{}", address, port)).unwrap();
    socket.set_broadcast(true).unwrap();

    socket
}

fn broadcastable_socket<A: ToSocketAddrs>(addr: A) -> UdpSocket {
    let socket = UdpSocket::bind(addr).expect("Failed to bind broadcast socket");
    socket.set_broadcast(true).expect("Failed enabling SO_BROADCAST");

    socket
}

fn main() -> Result<(), io::Error> {
    let broadcast_client = BroadcastClient::new("169.254.224.232:50000");

    let listener_handler = {
        let threehoundred_millis = Duration::from_millis(300);
        let listener_socket = UdpSocket::bind("169.254.224.232:50002")?;

        thread::spawn(move || {
            let mut buffer = [0u8; 512];
            match listener_socket.recv_from(&mut buffer) {
                Ok((number_of_bytes, source)) => {
                    eprintln!("{}: {}, {:?}", source, number_of_bytes, &buffer[..number_of_bytes]);
                }
                Err(error) => {
                    eprintln!("Failed reading broadcast socket: #{:?}", error);
                }
            }

        })
    };

    let broadcast_handler = {
        let socket_ref = broadcast_client.socket.clone();
        thread::spawn(move || {
            let threehoundred_millis = Duration::from_millis(10000);
            loop {
                let mut buffer = [0u8; 512];
                match socket_ref.lock().unwrap().recv_from(&mut buffer) {
                    Ok((number_of_bytes, source)) => {
                        eprintln!("{}: {}", source, number_of_bytes);
                    }
                    Err(error) => {
                        eprintln!("Failed reading broadcast socket: #{:?}", error);
                    }
                }
                thread::sleep(threehoundred_millis);
            }
        })
    };

    let player_communication_handler = {
        let thread_sleep = Duration::from_millis(300);
        thread::spawn(move || {
            for sequence in 0x01 ..= 0x03 {
                let mut socket = random_broadcast_socket("169.254.224.232");
                send_data(&socket, "169.254.255.255:50000", Message::phase1(sequence));
                thread::sleep(thread_sleep);
            }
            for sequence in 0x01..=0x06 {
                for index in 1..=6 {
                    let mut socket = random_broadcast_socket("169.254.224.232");
                    send_data(&socket, "169.254.255.255:50000", Message::phase2(sequence, index));
                    thread::sleep(thread_sleep);
                }
            }
            send_data(&random_broadcast_socket("169.254.224.232"), "169.254.255.255:50000", Message::broadcast());

        }).join();
        eprintln!("{}", "I just eneded this work");
    };

    let tick_handler = thread::spawn(move || {
        let threehoundred_millis = Duration::from_millis(300);
        loop {
            thread::sleep(threehoundred_millis);
            eprintln!("{}", "Application tick");
        }
    }).join();

    Ok(())
}

type PioneerMessage = Vec<Vec<u8>>;

struct Message;
impl Message {
    pub fn broadcast() -> PioneerMessage {
        vec![
            SOFTWARE_IDENTIFICATION.to_vec(),
            vec![0x29],
            APPLICATION_NAME.to_vec(),
            vec![0x01,0x01,0x11],
            vec![0x00,0x38,0x11],
            vec![0x00,0x00,0xc0,0x00,0x10,0x00,
                0x00,0x00,0x00,0x00,0x00,0x00,
                0x10,0x00,0x00,0x00,0x09,0xff,
                0x00
            ],
        ]

    }

    pub fn phase1(sequence: u8) -> PioneerMessage {
        vec![
            SOFTWARE_IDENTIFICATION.to_vec(),
            vec![0x00,0x00],
            APPLICATION_NAME.to_vec(),
            vec![0x01, 0x03, 0x00, 0x2c, sequence, 0x04],
            MAC_ADDRESS.to_vec(),
        ]
    }

    pub fn phase2(sequence: u8, index: i32) -> PioneerMessage {
        vec![
            SOFTWARE_IDENTIFICATION.to_vec(),
            vec![0x02, 0x00],
            APPLICATION_NAME.to_vec(),
            vec![0x01, 0x03, 0x00],
            vec![0x32, 0xa9, 0xfe, 0x30, 0xe7 ],
            MAC_ADDRESS.to_vec(),
            vec![Self::map_sequence_byte(index), sequence],
            vec![0x04, 0x01]
        ]
    }

    fn map_sequence_byte(index: i32) -> u8 {
        if index == 1 {
            0x11
        } else if index == 2 {
            0x12
        } else if index == 3 {
            0x29
        } else if index == 4 {
            0x2a
        } else if index == 5 {
            0x2b
        } else {
            0x2c
        }
    }
}
