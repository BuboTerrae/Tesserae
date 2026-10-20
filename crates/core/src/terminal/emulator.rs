use crate::terminal::Cell;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CursorShape {
    #[default]
    Block,
    Underline,
    Beam,
    HollowBlock,
    Hidden,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Cursor {
    pub x: usize,
    pub y: usize,
    pub shape: CursorShape,
}

pub struct Snapshot {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Cell>,
    pub cursor: Cursor,
    pub title: Option<String>,
    pub alt_screen: bool,
    pub show_cursor: bool,
    pub app_cursor: bool,
    pub display_offset: usize,
    pub history_size: usize,
}

impl Snapshot {
    pub fn cell(&self, x: usize, y: usize) -> &Cell {
        &self.cells[y * self.cols + x]
    }

    pub fn line(&self, y: usize) -> &[Cell] {
        &self.cells[y * self.cols..(y + 1) * self.cols]
    }
}

pub trait Emulator: Send {
    fn feed(&mut self, bytes: &[u8]);
    fn resize(&mut self, cols: u16, rows: u16);
    fn scroll(&mut self, _delta: i32) {}
    fn snapshot(&self) -> Snapshot;
    fn take_responses(&mut self) -> Vec<u8>;
}
