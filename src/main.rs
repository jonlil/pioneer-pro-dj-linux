use std::net::{Ipv4Addr, UdpSocket, SocketAddr};
use std::str;

fn extract_device_data_from_udp_package(source: &SocketAddr, data: &[u8]) -> PioneerPlayer {
    //println!("Player: #{:?}", str::from_utf8(&data[35..36]).unwrap());

    PioneerPlayer {
        model: str::from_utf8(&data[12..19]).unwrap().to_owned(),
        address: source.to_owned(),
    }
}

#[derive(Debug)]
struct PioneerPlayer {
    model: String,
    address: SocketAddr,
}

fn main() -> std::io::Result<()> {
    {
        let mut socket = UdpSocket::bind("0.0.0.0:50000")?;

        // TODO: solve this very naive implementation
        loop {
            let mut buf = [0; 100];
            let (number_of_bytes, src) = socket.recv_from(&mut buf)?;

            let buf = &mut buf[..number_of_bytes];
            if number_of_bytes == 54 {
                println!("#{:?}", extract_device_data_from_udp_package(&src, &buf));
            }
        }
    } // the socket is closed here
    Ok(())
}
