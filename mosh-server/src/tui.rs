use std::{io, sync::mpsc};
use tui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use termion::event::Key;
use termion::input::TermRead;
use tui::style::{Color, Style};

pub fn run(sessions: crate::session::Store) -> Result<(), Box<dyn std::error::Error>> {
    use termion::raw::IntoRawMode;
    use tui::backend::TermionBackend;

    let stdout = std::io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let keys = Keys::new();
    loop {
        terminal.draw(|f| {
            let block = Block::default()
                .title("Mosh server")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Reset).fg(Color::Red));

            if sessions.len() == 0 {
                let paragraph = Paragraph::new("No sessions").block(block);
                f.render_widget(paragraph, f.size());
                return;
            } else {
                f.render_widget(block, f.size());
            }

            let session_contraints =
                std::iter::repeat(Constraint::Percentage(100 / sessions.len() as u16))
                    .take(sessions.len())
                    .collect::<Vec<_>>();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(session_contraints)
                .split(f.size());

            let sessions = sessions.iter().map(|entry| {
                let block = Block::default()
                    .title(format!("Session {}", entry.key()))
                    .borders(Borders::TOP | Borders::BOTTOM)
                    .style(Style::default().bg(Color::DarkGray));
                let paragraph = Paragraph::new(entry.state.lock().to_owned()).block(block);
                paragraph
            });

            for (i, session) in sessions.enumerate() {
                f.render_widget(session, chunks[i as usize]);
            }
        })?;

        match keys.next()? {
            Key::Char('q') => {
                break;
            }
            _ => {}
        };
    }
    Ok(())
}

/// A small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Keys {
    rx: mpsc::Receiver<Key>,
}

impl Keys {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let stdin = io::stdin();
            for evt in stdin.keys() {
                if let Ok(key) = evt {
                    if let Err(err) = tx.send(key) {
                        eprintln!("{}", err);
                        return;
                    }
                }
            }
        });

        Self { rx }
    }

    pub fn next(&self) -> Result<Key, mpsc::RecvError> {
        self.rx.recv()
    }
}
