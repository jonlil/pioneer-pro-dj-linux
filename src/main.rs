mod discovery;
mod player;

use std::net::{UdpSocket};
use player::{PlayerCollection};
use discovery::*;
use std::{thread, time};


struct Application {
    players: PlayerCollection,
}

impl Application {
    fn new() -> Self {
        Self {
            players: PlayerCollection::new(),
        }
    }
}

fn main() -> std::io::Result<()> {
    {
        let handler = thread::spawn(|| {
            let mut index: u8 = 0;
            let ten_millis = time::Duration::from_millis(1000);
            loop {
                eprintln!("#{:?}", index);
                thread::sleep(ten_millis);
                if index == 255 {
                    index = 0;
                }
                index += 1;
            }
        });

        // Create an Application struct
        let mut application = Application::new();

        // Thread for PlayerDiscovery
        discovery::run(&mut application.players, discovery::Options {
            listen_address: String::from("0.0.0.0:50000"),
        });
        handler.join().unwrap();

        //eprintln!("#{:?}", player_discovery.players);

        // Thread for UI (rendering)
        // Thread for communication with "connected players"
    } // the socket is closed here
    Ok(())
}
