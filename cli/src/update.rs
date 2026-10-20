use color_eyre::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

use crate::{app::App, input::encode_key};

pub fn update(app: &mut App, key: KeyEvent) -> Result<()> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('q' | 'Q'))
    {
        app.quit();
        return Ok(());
    }

    let snapshot = app.snapshot();
    if snapshot.display_offset > 0 {
        app.scroll(-(snapshot.display_offset as i32));
    }

    let bytes = encode_key(key, snapshot.app_cursor);
    if !bytes.is_empty() {
        app.write(&bytes)?;
    }

    Ok(())
}

pub fn update_mouse(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollUp => app.scroll(3),
        MouseEventKind::ScrollDown => app.scroll(-3),
        _ => {}
    }
}
