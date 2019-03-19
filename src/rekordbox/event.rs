use crate::rekordbox::player::Player;

#[derive(Debug, PartialEq)]
pub enum Event {
    PlayerBroadcast(Player),
    ApplicationBroadcast,
    Unknown,
    Error,
    InitiateLink,
    Tick,
}
