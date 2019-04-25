mod rekordbox;
mod utils;
mod component;
mod rpc;

extern crate rand;
extern crate pnet;

use std::io;

fn main() -> Result<(), io::Error> {
    let mut app = component::App {};

    match app.run() {
        Ok(_) => {},
        Err(err) => eprintln!("{:?}", err),
    }

    Ok(())
}
