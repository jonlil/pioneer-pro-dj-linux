mod discovery;
mod player;

use std::net::{UdpSocket};
use discovery::{PlayerDiscovery};
use player::PlayerIter;


fn main() -> std::io::Result<()> {
    {
        let mut socket = UdpSocket::bind("0.0.0.0:50000")?;
        let mut players = PlayerIter::new();

        // Create an Application struct
        // Thread for PlayerDiscovery
        PlayerDiscovery::run(&mut socket, &mut players);

        eprintln!("#{:?}", players);

        // Thread for UI (rendering)
        // Thread for communication with "connected players"
    } // the socket is closed here
    Ok(())
}
