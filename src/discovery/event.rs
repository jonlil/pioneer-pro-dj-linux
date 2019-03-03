use crate::player::{Player};

#[derive(Debug)]
pub enum Event {
    Annoncement(Player),
    Error(String),
}
