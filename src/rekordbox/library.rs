use std::net::{TcpStream, Shutdown};
use std::io::Read;

pub fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0u8; 1024];

    match stream.read(&mut buffer) {
        Ok(size) => {
            eprintln!("{:?}", String::from_utf8_lossy(&buffer[0..size]));
        },
        Err(_) => {
            println!("An error occurred, terminating connection with {}", stream.peer_addr().unwrap());
            stream.shutdown(Shutdown::Both).unwrap();
        }
    }
}
