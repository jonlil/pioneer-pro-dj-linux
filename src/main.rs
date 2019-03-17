mod rekordbox;
mod utils;
mod component;

extern crate rand;
extern crate pnet;

use std::io;
use crate::rekordbox::player::{PlayerCollection};

fn main() -> Result<(), io::Error> {
    let mut app = component::App {};

    app.run();

    Ok(())
}
