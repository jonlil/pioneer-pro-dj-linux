mod discovery;
mod util;
mod player;

use std::io;

use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, Widget};
use tui::Terminal;

use crate::util::event::{Event, Events};
use crate::discovery::event::{
    Events as DiscoveryEvents,
    Event as DiscoveryEvent,
};
use crate::player::{PlayerCollection};

struct App {
    players: PlayerCollection,
}

impl App {
    fn new() -> Self {
        Self {
            players: PlayerCollection::new(),
        }
    }

    fn update(&mut self) {  }
}

fn main() -> Result<(), failure::Error> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = Events::new();
    let discovery_events = DiscoveryEvents::new();

    let mut app = App::new();

    loop {
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Percentage(25),
                        Constraint::Percentage(25),
                        Constraint::Percentage(25),
                        Constraint::Percentage(25),
                    ]
                    .as_ref(),
                )
                .split(f.size());
        })?;

        match events.next()? {
            Event::Input(input) => {
                if input == Key::Char('q') {
                    break;
                }
            }
            Event::Tick => {
                app.update();
            }
        };

        match discovery_events.next()? {
            DiscoveryEvent::Annoncement(player) => {
                app.players.add_or_update(player);
            },
            DiscoveryEvent::Error(_) => (),
        };
    }

    Ok(())
}
