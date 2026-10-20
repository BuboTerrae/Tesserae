use color_eyre::Result;
use ratatui::crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, panic};

use crate::{app::App, event::EventHandler, ui};

pub type CrosstermTerminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stderr>>;

pub struct Tui {
    terminal: CrosstermTerminal,
    pub events: EventHandler,
    active: bool,
}

impl Tui {
    pub fn new(terminal: CrosstermTerminal, events: EventHandler) -> Self {
        Self {
            terminal,
            events,
            active: false,
        }
    }

    pub fn enter(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        self.active = true;
        execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;

        let panic_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic| {
            Self::reset().expect("failed to reset the terminal");
            panic_hook(panic);
        }));

        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    fn reset() -> Result<()> {
        terminal::disable_raw_mode()?;
        execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(())
    }

    pub fn exit(&mut self) -> Result<()> {
        if self.active {
            Self::reset()?;
            self.terminal.show_cursor()?;
            self.active = false;
        }
        Ok(())
    }

    pub fn draw(&mut self, app: &mut App) -> Result<()> {
        self.terminal.draw(|frame| ui::render(app, frame))?;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        if self.active {
            let _ = Self::reset();
            let _ = self.terminal.show_cursor();
        }
    }
}
