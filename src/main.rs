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
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;

use crate::util::event::{Event, Events};
use crate::discovery::event::{
    Event as DiscoveryEvent,
};
use crate::player::{PlayerCollection};

struct App {
    players: PlayerCollection,
    messages: Vec<String>,
}

impl App {
    fn new() -> Self {
        Self {
            players: PlayerCollection::new(),
            messages: Vec::new(),
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

    let link_socket = Arc::new(Mutex::new(
        UdpSocket::bind("0.0.0.0:50002").unwrap()
    ));
    let events = Events::new();
    let mut app = App::new();
    let mut linking: bool = false;

    loop {
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(f.size());

            {
                let events = app.players.iter().map(|player| {
                    Text::styled(
                        format!("{}: {} ({})",
                            player.number,
                            player.model,
                            player.ip()),
                        Style::default().fg(Color::White)
                    )
                });

                List::new(events)
                    .block(Block::default().borders(Borders::ALL).title("List"))
                    .start_corner(Corner::BottomLeft)
                    .render(&mut f, chunks[1]);
            }

            {
                let events = app.messages.iter().map(|message| {
                    Text::styled(format!("{}", message), Style::default().fg(Color::White)
                    )
                });

                List::new(events)
                    .block(Block::default().borders(Borders::ALL).title("List"))
                    .start_corner(Corner::BottomLeft)
                    .render(&mut f, chunks[0]);
            }
        })?;

        match events.next()? {
            Event::Input(input) => {
                if input == Key::Char('q') {
                    break;
                }

                if input == Key::Char('c') {
                    // TODO: Figure out how to store a reference to
                    // the thread (so we can stop the network flood).
                    if linking == false {
                        linking = true;
                        app.messages.push(String::from("Linking players"));
                        events.create_link_channel(&link_socket);
                    }
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
            Event::Message(string) => {
                app.messages.push(string);
            }
        };


    }

    Ok(())
}
