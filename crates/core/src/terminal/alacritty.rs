use std::sync::{Arc, Mutex};

use alacritty_terminal::{
    event::{Event, EventListener},
    grid::{Dimensions, Scroll},
    term::{
        Config, Term, TermMode,
        cell::{Cell as AnsiCell, Flags},
        point_to_viewport,
    },
    vte::ansi::{Color as AnsiColor, CursorShape as AnsiCursorShape, NamedColor, Processor},
};

use super::{Attrs, Cell, Color, Cursor, CursorShape, Emulator, Snapshot};

#[derive(Clone, Default)]
struct EventProxy {
    responses: Arc<Mutex<Vec<u8>>>,
    title: Arc<Mutex<Option<String>>>,
}

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        match event {
            Event::PtyWrite(text) => {
                if let Ok(mut responses) = self.responses.lock() {
                    responses.extend_from_slice(text.as_bytes());
                }
            }
            Event::Title(title) => {
                if let Ok(mut current) = self.title.lock() {
                    *current = Some(title)
                }
            }
            Event::ResetTitle => {
                if let Ok(mut current) = self.title.lock() {
                    *current = None;
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy)]
struct Dims {
    cols: usize,
    rows: usize,
}

impl Dimensions for Dims {
    fn columns(&self) -> usize {
        self.cols
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn total_lines(&self) -> usize {
        self.rows
    }
}

pub struct AlacrittyEmulator {
    term: Term<EventProxy>,
    parser: Processor,
    event: EventProxy,
}

impl AlacrittyEmulator {
    pub fn new(cols: u16, rows: u16) -> Self {
        let event = EventProxy::default();
        let dims = Dims {
            cols: cols as usize,
            rows: rows as usize,
        };
        let term = Term::new(Config::default(), &dims, event.clone());
        Self {
            term,
            parser: Processor::new(),
            event,
        }
    }
}

impl Emulator for AlacrittyEmulator {
    fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        self.term.resize(Dims {
            cols: cols as usize,
            rows: rows as usize,
        });
    }

    fn scroll(&mut self, delta: i32) {
        self.term.scroll_display(Scroll::Delta(delta));
    }

    fn snapshot(&self) -> Snapshot {
        let cols = self.term.columns();
        let rows = self.term.screen_lines();
        let history_size = self.term.history_size();

        let content = self.term.renderable_content();
        let display_offset = content.display_offset;
        let mode = content.mode;
        let cursor_point = content.cursor.point;
        let cursor_shape = content.cursor.shape;

        let mut cells = vec![Cell::default(); cols * rows];
        for indexed in content.display_iter {
            if let Some(point) = point_to_viewport(display_offset, indexed.point) {
                if point.line < rows && point.column < cols {
                    cells[point.line * cols + point.column.0] = map_cell(indexed.cell);
                }
            }
        }

        let cursor = point_to_viewport(display_offset, cursor_point)
            .map(|point| Cursor {
                x: point.column.0.min(cols.saturating_sub(1)),
                y: point.line.min(rows.saturating_sub(1)),
                shape: map_shape(cursor_shape),
            })
            .unwrap_or(Cursor {
                x: 0,
                y: 0,
                shape: CursorShape::Block,
            });

        Snapshot {
            cols,
            rows,
            cells,
            cursor,
            title: self.event.title.lock().ok().and_then(|title| title.clone()),
            alt_screen: mode.contains(TermMode::ALT_SCREEN),
            show_cursor: mode.contains(TermMode::SHOW_CURSOR),
            app_cursor: mode.contains(TermMode::APP_CURSOR),
            display_offset,
            history_size,
        }
    }

    fn take_responses(&mut self) -> Vec<u8> {
        match self.event.responses.lock() {
            Ok(mut responses) => std::mem::take(&mut *responses),
            Err(_) => Vec::new(),
        }
    }
}

fn map_cell(cell: &AnsiCell) -> Cell {
    let flags = cell.flags;
    let mut attrs = Attrs(0);

    if flags.contains(Flags::BOLD) {
        attrs.insert(Attrs::BOLD);
    }
    if flags.contains(Flags::DIM) {
        attrs.insert(Attrs::DIM);
    }
    if flags.contains(Flags::ITALIC) {
        attrs.insert(Attrs::ITALIC);
    }
    if flags.intersects(Flags::ALL_UNDERLINES) {
        attrs.insert(Attrs::UNDERLINE);
    }
    if flags.contains(Flags::INVERSE) {
        attrs.insert(Attrs::REVERSE);
    }
    if flags.contains(Flags::HIDDEN) {
        attrs.insert(Attrs::HIDDEN);
    }
    if flags.contains(Flags::STRIKEOUT) {
        attrs.insert(Attrs::STRIKETHROUGH);
    }
    if flags.contains(Flags::WIDE_CHAR) {
        attrs.insert(Attrs::WIDE);
    }
    if flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
        attrs.insert(Attrs::WIDE_SPACER);
    }

    let extra = cell
        .zerowidth()
        .map(|chars| chars.iter().copied().collect::<String>().into_boxed_str());

    Cell {
        ch: cell.c,
        extra,
        fg: map_color(cell.fg),
        bg: map_color(cell.bg),
        attrs,
    }
}

fn map_color(color: AnsiColor) -> Color {
    match color {
        AnsiColor::Spec(rgb) => Color::Rgb(rgb.r, rgb.g, rgb.b),
        AnsiColor::Indexed(index) => Color::Indexed(index),
        AnsiColor::Named(named) => match named {
            NamedColor::Black => Color::Indexed(0),
            NamedColor::Red => Color::Indexed(1),
            NamedColor::Green => Color::Indexed(2),
            NamedColor::Yellow => Color::Indexed(3),
            NamedColor::Blue => Color::Indexed(4),
            NamedColor::Magenta => Color::Indexed(5),
            NamedColor::Cyan => Color::Indexed(6),
            NamedColor::White => Color::Indexed(7),
            NamedColor::BrightBlack => Color::Indexed(8),
            NamedColor::BrightRed => Color::Indexed(9),
            NamedColor::BrightGreen => Color::Indexed(10),
            NamedColor::BrightYellow => Color::Indexed(11),
            NamedColor::BrightBlue => Color::Indexed(12),
            NamedColor::BrightMagenta => Color::Indexed(13),
            NamedColor::BrightCyan => Color::Indexed(14),
            NamedColor::BrightWhite => Color::Indexed(15),
            NamedColor::DimBlack => Color::Indexed(0),
            NamedColor::DimRed => Color::Indexed(1),
            NamedColor::DimGreen => Color::Indexed(2),
            NamedColor::DimYellow => Color::Indexed(3),
            NamedColor::DimBlue => Color::Indexed(4),
            NamedColor::DimMagenta => Color::Indexed(5),
            NamedColor::DimCyan => Color::Indexed(6),
            NamedColor::DimWhite => Color::Indexed(7),
            NamedColor::Foreground
            | NamedColor::Background
            | NamedColor::Cursor
            | NamedColor::BrightForeground
            | NamedColor::DimForeground => Color::Default,
        },
    }
}

fn map_shape(shape: AnsiCursorShape) -> CursorShape {
    match shape {
        AnsiCursorShape::Block => CursorShape::Block,
        AnsiCursorShape::Underline => CursorShape::Underline,
        AnsiCursorShape::Beam => CursorShape::Beam,
        AnsiCursorShape::HollowBlock => CursorShape::HollowBlock,
        AnsiCursorShape::Hidden => CursorShape::Hidden,
    }
}
