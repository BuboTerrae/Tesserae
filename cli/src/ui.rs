use crate::app::App;
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color as TuiColor, Modifier, Style},
    widgets::Clear,
};
use tesserae_core::terminal::{Attrs, Cell as CoreCell, Color, Snapshot};

pub fn render(app: &mut App, frame: &mut Frame) {
    let snapshot = app.snapshot();
    render_terminal(&snapshot, frame.area(), frame);
}

pub fn render_terminal(snapshot: &Snapshot, area: Rect, frame: &mut Frame) {
    frame.render_widget(Clear, area);
    {
        let buffer = frame.buffer_mut();
        paint(snapshot, area, buffer);
    }

    if snapshot.show_cursor {
        let x = area.x + (snapshot.cursor.x as u16).min(area.width.saturating_sub(1));
        let y = area.y + (snapshot.cursor.y as u16).min(area.height.saturating_sub(1));
        frame.set_cursor_position((x, y));
    }
}

fn paint(snapshot: &Snapshot, area: Rect, buffer: &mut Buffer) {
    for y in 0..area.height {
        for x in 0..area.width {
            let sx = x as usize;
            let sy = y as usize;
            if sx >= snapshot.cols || sy >= snapshot.rows {
                continue;
            }

            let cell = snapshot.cell(sx, sy);
            let symbol = if cell.attrs.contains(Attrs::WIDE_SPACER) {
                " ".to_string()
            } else if let Some(extra) = &cell.extra {
                format!("{}{}", cell.ch, extra.as_ref())
            } else {
                cell.ch.to_string()
            };

            let target = &mut buffer[(area.x + x, area.y + y)];
            target.set_symbol(&symbol);
            target.set_style(cell_style(cell));
        }
    }
}

fn cell_style(cell: &CoreCell) -> Style {
    let mut modifiers = Modifier::empty();
    let attrs = cell.attrs;

    if attrs.contains(Attrs::BOLD) {
        modifiers |= Modifier::BOLD;
    }
    if attrs.contains(Attrs::DIM) {
        modifiers |= Modifier::DIM;
    }
    if attrs.contains(Attrs::ITALIC) {
        modifiers |= Modifier::ITALIC;
    }
    if attrs.contains(Attrs::UNDERLINE) {
        modifiers |= Modifier::UNDERLINED;
    }
    if attrs.contains(Attrs::BLINK) {
        modifiers |= Modifier::SLOW_BLINK;
    }
    if attrs.contains(Attrs::REVERSE) {
        modifiers |= Modifier::REVERSED;
    }
    if attrs.contains(Attrs::HIDDEN) {
        modifiers |= Modifier::HIDDEN;
    }
    if attrs.contains(Attrs::STRIKETHROUGH) {
        modifiers |= Modifier::CROSSED_OUT;
    }

    Style::default()
        .fg(map_color(cell.fg))
        .bg(map_color(cell.bg))
        .add_modifier(modifiers)
}

fn map_color(color: Color) -> TuiColor {
    match color {
        Color::Default => TuiColor::Reset,
        Color::Indexed(index) => TuiColor::Indexed(index),
        Color::Rgb(r, g, b) => TuiColor::Rgb(r, g, b),
    }
}
