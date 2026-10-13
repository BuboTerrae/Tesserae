pub mod app;
pub mod event;
pub mod tui;
pub mod ui;
pub mod update;

use app::App;
use event::{Event, EventHandler};
use tui::Tui;
use update::update;

use color_eyre::Result;
use ratatui::{Terminal, backend::CrosstermBackend};

fn main() -> Result<()> {
    let mut app = App::new();
    let backend = CrosstermBackend::new(std::io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.enter()?;

    while !app.exit {
        tui.draw(&mut app)?;

        match tui.events.next()? {
            Event::Tick => {}
            Event::Key(key_event) => update(&mut app, key_event),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        };
    }

    tui.exit()?;
    Ok(())
}
