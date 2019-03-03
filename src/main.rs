mod discovery;
mod util;
mod player;

use std::io;

use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout, Corner};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Widget, List, Text};
use tui::Terminal;

use crate::util::event::{Event, Events};
use crate::discovery::event::{
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

    let mut app = App::new();

    loop {
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(f.size());

            {
                let events = app.players.iter().map(|player| {
                    Text::styled(
                        format!("{}: {}", player.number, player.model),
                        Style::default().fg(Color::White)
                    )
                });

                List::new(events)
                    .block(Block::default().borders(Borders::ALL).title("List"))
                    .start_corner(Corner::BottomLeft)
                    .render(&mut f, chunks[1]);
            }
        })?;

        match events.next()? {
            Event::Input(input) => {
                if input == Key::Char('q') {
                    break;
                }
            }
            Event::Tick => {
                app.update();
            },
            Event::Discovery(evnt) => {
                match evnt {
                    DiscoveryEvent::Annoncement(player) => app.players.add_or_update(player),
                    _ => (),
                }
            }
        };
    }

    Ok(())
}
