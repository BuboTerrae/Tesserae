pub mod app;
pub mod event;
pub mod input;
pub mod tui;
pub mod ui;
pub mod update;

use app::App;
use event::{Event, EventHandler};
use tui::Tui;
use update::{update, update_mouse};

use color_eyre::Result;
use ratatui::{Terminal, backend::CrosstermBackend, crossterm::terminal};
use tesserae_core::pty::PtySession;

fn main() -> Result<()> {
    color_eyre::install()?;

    let (cols, rows) = terminal::size()?;
    let shell = PtySession::default_shell();
    let mut app = App::new(&shell, cols, rows)?;

    let backend = CrosstermBackend::new(std::io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(33);
    let mut tui = Tui::new(terminal, events);
    tui.enter()?;

    while !app.exit {
        tui.draw(&mut app)?;

        match tui.events.next()? {
            Event::Tick => {
                if app.child_exited()? {
                    app.quit();
                }
            }
            Event::Key(key_event) => update(&mut app, key_event)?,
            Event::Mouse(mouse_event) => update_mouse(&mut app, mouse_event),
            Event::Resize(w, h) => app.resize(w, h)?,
        };
    }

    tui.exit()?;
    Ok(())
}
