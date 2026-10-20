use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Color {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Attrs(pub u16);

impl Attrs {
    pub const BOLD: Attrs = Attrs(1 << 0);
    pub const DIM: Attrs = Attrs(1 << 1);
    pub const ITALIC: Attrs = Attrs(1 << 2);
    pub const UNDERLINE: Attrs = Attrs(1 << 3);
    pub const BLINK: Attrs = Attrs(1 << 4);
    pub const REVERSE: Attrs = Attrs(1 << 5);
    pub const HIDDEN: Attrs = Attrs(1 << 6);
    pub const STRIKETHROUGH: Attrs = Attrs(1 << 7);
    pub const WIDE: Attrs = Attrs(1 << 8);
    pub const WIDE_SPACER: Attrs = Attrs(1 << 9);

    pub fn contains(self, other: Attrs) -> bool {
        self.0 & other.0 == other.0
    }
    pub fn insert(&mut self, other: Attrs) {
        self.0 |= other.0;
    }
    pub fn remove(&mut self, other: Attrs) {
        self.0 &= !other.0;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct RowFlags(pub u8);
impl RowFlags {
    pub const WRAPPED: RowFlags = RowFlags(1 << 0);
    pub fn contains(self, other: RowFlags) -> bool {
        self.0 & other.0 == other.0
    }
    pub fn insert(&mut self, other: RowFlags) {
        self.0 |= other.0;
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ch: char,
    pub extra: Option<Box<str>>,
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attrs,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            extra: None,
            fg: Color::Default,
            bg: Color::Default,
            attrs: Attrs(0),
        }
    }
}

pub struct Screen {
    cols: usize,
    rows: usize,
    cells: Vec<Cell>,
    row_flags: Vec<RowFlags>,
    cursor: (usize, usize),
    saved_cursor: (usize, usize),
    pen: Cell,
    scroll_top: usize,
    scroll_bottom: usize,
    wrap_pending: bool,
    tab_stops: Vec<bool>,
    autowrap: bool,
    scrollback: VecDeque<Vec<Cell>>,
    scrollback_limit: usize,
}

impl Screen {
    pub fn new(cols: u16, rows: u16) -> Self {
        let cols = cols as usize;
        let rows = rows as usize;
        let mut tab_stops = vec![false; cols];
        for x in (8..cols).step_by(8) {
            tab_stops[x] = true;
        }

        Self {
            cols,
            rows,
            cells: vec![Cell::default(); cols * rows],
            row_flags: vec![RowFlags::default(); rows],
            cursor: (0, 0),
            saved_cursor: (0, 0),
            pen: Cell::default(),
            scroll_top: 0,
            scroll_bottom: rows - 1,
            wrap_pending: false,
            tab_stops,
            autowrap: true,
            scrollback: VecDeque::new(),
            scrollback_limit: 1000,
        }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }
    pub fn rows(&self) -> usize {
        self.rows
    }
    pub fn cursor(&self) -> (usize, usize) {
        self.cursor
    }
    pub fn pen(&self) -> &Cell {
        &self.pen
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.cols + x
    }
    pub fn cell(&self, x: usize, y: usize) -> Cell {
        self.cells[self.index(x, y)].clone()
    }
    pub fn line(&self, y: usize) -> &[Cell] {
        &self.cells[y * self.cols..(y + 1) * self.cols]
    }
    fn blank(&self) -> Cell {
        Cell {
            bg: self.pen.bg,
            ..Cell::default()
        }
    }

    pub fn set_cursor(&mut self, x: usize, y: usize) {
        self.cursor = (x.min(self.cols - 1), y.min(self.rows - 1));
        self.wrap_pending = false;
    }

    pub fn carriage_return(&mut self) {
        self.cursor.0 = 0;
        self.wrap_pending = false;
    }

    pub fn backspace(&mut self) {
        if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
        }
        self.wrap_pending = false;
    }

    pub fn tab(&mut self) {
        let next = (self.cursor.0 / 8 + 1) * 8;
        self.cursor.0 = next.min(self.cols - 1);
        self.wrap_pending = false;
    }

    pub fn scroll_up(&mut self, n: usize) {
        if n == 0 || self.scroll_top > self.scroll_bottom {
            return;
        }
        let n = n.min(self.scroll_bottom - self.scroll_top + 1);
        let blank = self.blank();

        for y in self.scroll_top..=(self.scroll_bottom - n) {
            for x in 0..self.cols {
                let c = self.cells[self.index(x, y + n)].clone();
                let dst = self.index(x, y);
                self.cells[dst] = c;
            }
            self.row_flags[y] = self.row_flags[y + n];
        }

        for y in (self.scroll_bottom + 1 - n)..=self.scroll_bottom {
            let r = (y * self.cols)..((y + 1) * self.cols);
            self.cells[r].fill(blank.clone());
            self.row_flags[y] = RowFlags::default();
        }
    }

    pub fn scroll_down(&mut self, n: usize) {
        if n == 0 || self.scroll_top > self.scroll_bottom {
            return;
        }
        let n = n.min(self.scroll_bottom - self.scroll_top + 1);
        let blank = self.blank();

        for y in (self.scroll_top + n..=self.scroll_bottom).rev() {
            for x in 0..self.cols {
                let c = self.cells[self.index(x, y - n)].clone();
                let dst = self.index(x, y);
                self.cells[dst] = c;
            }
            self.row_flags[y] = self.row_flags[y - n];
        }

        for y in self.scroll_top..(self.scroll_top + n) {
            let r = (y * self.cols)..((y + 1) * self.cols);
            self.cells[r].fill(blank.clone());
            self.row_flags[y] = RowFlags::default();
        }
    }

    pub fn newline(&mut self) {
        let (col, row) = self.cursor;
        if row == self.scroll_bottom {
            self.scroll_up(1);
            self.cursor = (col, self.scroll_bottom);
        } else if row + 1 < self.rows {
            self.cursor.1 = row + 1;
        }
    }

    pub fn put_char(&mut self, ch: char) {
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        self.put_grapheme(s, 1);
    }

    pub fn put_grapheme(&mut self, text: &str, width: usize) {
        if width == 0 {
            return;
        }

        if self.wrap_pending {
            self.wrap_pending = false;
            self.carriage_return();
            self.newline();
        }

        if self.cursor.0 + width > self.cols {
            self.carriage_return();
            self.newline();
        }

        let (col, row) = self.cursor;

        let mut cell = self.pen.clone();
        let mut chars = text.chars();
        cell.ch = chars.next().unwrap_or(' ');
        let rest: String = chars.collect();
        cell.extra = if rest.is_empty() {
            None
        } else {
            Some(rest.into_boxed_str())
        };

        if width == 2 {
            cell.attrs.insert(Attrs::WIDE);
        }
        let idx = self.index(col, row);
        self.cells[idx] = cell;

        if width == 2 && col + 1 < self.cols {
            let mut spacer = Cell {
                bg: self.pen.bg,
                ..Cell::default()
            };
            spacer.attrs.insert(Attrs::WIDE_SPACER);
            let idx = self.index(col + 1, row);
            self.cells[idx] = spacer;
        }

        if col + width >= self.cols {
            self.cursor = (self.cols - width, row);
            self.wrap_pending = true;
        } else {
            self.cursor = (col + width, row);
        }
    }

    pub(crate) fn pen_mut(&mut self) -> &mut Cell {
        &mut self.pen
    }

    pub fn erase_in_line(&mut self, mode: u16) {
        let (col, row) = self.cursor;
        let base = row * self.cols;
        let blank = self.blank();
        match mode {
            0 => {
                for x in col..self.cols {
                    self.cells[base + x] = blank.clone();
                }
            }
            1 => {
                for x in 0..=col {
                    self.cells[base + x] = blank.clone();
                }
            }
            _ => {
                let r = base..base + self.cols;
                self.cells[r].fill(blank.clone());
            }
        }
    }

    pub fn erase_in_display(&mut self, mode: u16) {
        let (col, row) = self.cursor;
        let blank = self.blank();
        match mode {
            0 => {
                for x in col..self.cols {
                    self.cells[row * self.cols + x] = blank.clone();
                }
                for y in (row + 1)..self.rows {
                    let r = y * self.cols..(y + 1) * self.cols;
                    self.cells[r].fill(blank.clone());
                }
            }
            1 => {
                for y in 0..row {
                    let r = y * self.cols..(y + 1) * self.cols;
                    self.cells[r].fill(blank.clone());
                }
                for x in 0..=col {
                    self.cells[row * self.cols + x] = blank.clone();
                }
            }
            _ => {
                for c in &mut self.cells {
                    *c = blank.clone();
                }
            }
        }
    }

    pub fn save_cursor(&mut self) {
        self.saved_cursor = self.cursor;
    }

    pub fn restore_cursor(&mut self) {
        self.cursor = self.saved_cursor;
        self.wrap_pending = false;
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let (new_cols, new_rows) = (cols as usize, rows as usize);
        if new_cols == self.cols && new_rows == self.rows {
            return;
        }

        let mut cells = vec![Cell::default(); new_cols * new_rows];
        let copy_cols = new_cols.min(self.cols);
        let copy_rows = new_rows.min(self.rows);
        for y in 0..copy_rows {
            for x in 0..copy_cols {
                cells[y * new_cols + x] = self.cells[y * self.cols + x].clone();
            }
        }

        let mut row_flags = vec![RowFlags::default(); new_rows];
        row_flags[..copy_rows].copy_from_slice(&self.row_flags[..copy_rows]);

        self.cells = cells;
        self.cols = new_cols;
        self.rows = new_rows;
        self.row_flags = row_flags;
        self.scroll_top = 0;
        self.scroll_bottom = new_rows - 1;
        self.cursor = (
            self.cursor.0.min(new_cols - 1),
            self.cursor.1.min(new_rows - 1),
        );
        self.wrap_pending = false;
    }

    pub fn reset_pen(&mut self) {
        self.pen = Cell::default();
    }

    pub fn apply_sgr(&mut self, params: &[u16]) {
        if params.is_empty() {
            self.reset_pen();
            return;
        }

        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => self.reset_pen(),
                1 => self.pen.attrs.insert(Attrs::BOLD),
                2 => self.pen.attrs.insert(Attrs::DIM),
                3 => self.pen.attrs.insert(Attrs::ITALIC),
                4 => self.pen.attrs.insert(Attrs::UNDERLINE),
                5 => self.pen.attrs.insert(Attrs::BLINK),
                7 => self.pen.attrs.insert(Attrs::REVERSE),
                8 => self.pen.attrs.insert(Attrs::HIDDEN),
                9 => self.pen.attrs.insert(Attrs::STRIKETHROUGH),
                21 => self.pen.attrs.remove(Attrs::BOLD),
                22 => {
                    self.pen.attrs.remove(Attrs::BOLD);
                    self.pen.attrs.remove(Attrs::DIM);
                }
                23 => self.pen.attrs.remove(Attrs::ITALIC),
                24 => self.pen.attrs.remove(Attrs::UNDERLINE),
                25 => self.pen.attrs.remove(Attrs::BLINK),
                27 => self.pen.attrs.remove(Attrs::REVERSE),
                28 => self.pen.attrs.remove(Attrs::HIDDEN),
                29 => self.pen.attrs.remove(Attrs::STRIKETHROUGH),
                30..=37 => self.pen.fg = Color::Indexed((params[i] - 30) as u8),
                39 => self.pen.fg = Color::Default,
                40..=47 => self.pen.bg = Color::Indexed((params[i] - 40) as u8),
                49 => self.pen.bg = Color::Default,
                90..=97 => self.pen.fg = Color::Indexed((params[i] - 90 + 8) as u8),
                100..=107 => self.pen.bg = Color::Indexed((params[i] - 100 + 8) as u8),
                38 | 48 => {
                    let is_fg = params[i] == 38;
                    if let Some((color, extra)) = Self::parse_extended_color(&params[i + 1..]) {
                        if is_fg {
                            self.pen.fg = color
                        } else {
                            self.pen.bg = color
                        }
                        i += extra;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn parse_extended_color(params: &[u16]) -> Option<(Color, usize)> {
        match *params.first()? {
            5 => Some((Color::Indexed(*params.get(1)? as u8), 2)),
            2 => {
                let r = *params.get(1)? as u8;
                let g = *params.get(2)? as u8;
                let b = *params.get(3)? as u8;
                Some((Color::Rgb(r, g, b), 4))
            }
            _ => None,
        }
    }

    pub fn scroll_region(&self) -> (usize, usize) {
        (self.scroll_top, self.scroll_bottom)
    }

    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        if top >= bottom || bottom >= self.rows {
            self.reset_scroll_region();
        } else {
            self.scroll_top = top;
            self.scroll_bottom = bottom;
        }
        self.cursor = (0, self.scroll_top);
        self.wrap_pending = false;
    }

    pub fn reset_scroll_region(&mut self) {
        self.scroll_top = 0;
        self.scroll_bottom = self.rows - 1;
    }

    pub fn insert_lines(&mut self, n: usize) {
        let row = self.cursor.1;
        if row < self.scroll_top || row > self.scroll_bottom {
            return;
        }
        let n = n.min(self.scroll_bottom - row + 1);
        if n == 0 {
            return;
        }
        let blank = self.blank();

        if n <= self.scroll_bottom - row {
            for y in (row + n..=self.scroll_bottom).rev() {
                for x in 0..self.cols {
                    let c = self.cells[self.index(x, y - n)].clone();
                    let dst = self.index(x, y);
                    self.cells[dst] = c;
                }
                self.row_flags[y] = self.row_flags[y - n];
            }
        }

        for y in row..(row + n) {
            let r = y * self.cols..(y + 1) * self.cols;
            self.cells[r].fill(blank.clone());
            self.row_flags[y] = RowFlags::default();
        }
    }

    pub fn delete_lines(&mut self, n: usize) {
        let row = self.cursor.1;
        if row < self.scroll_top || row > self.scroll_bottom {
            return;
        }

        let n = n.min(self.scroll_bottom - row + 1);
        if n == 0 {
            return;
        }
        let blank = self.blank();

        if n <= self.scroll_bottom - row {
            for y in row..=(self.scroll_bottom - n) {
                for x in 0..self.cols {
                    let c = self.cells[self.index(x, y + n)].clone();
                    let dst = self.index(x, y);
                    self.cells[dst] = c;
                }
                self.row_flags[y] = self.row_flags[y + n];
            }
        }

        for y in (self.scroll_bottom + 1 - n)..=self.scroll_bottom {
            let r = y * self.cols..(y + 1) * self.cols;
            self.cells[r].fill(blank.clone());
            self.row_flags[y] = RowFlags::default();
        }
    }
}
