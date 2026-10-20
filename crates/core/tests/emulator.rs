use tesserae_core::terminal::{AlacrittyEmulator, Attrs, Color, Emulator, Snapshot};

fn line_text(snapshot: &Snapshot, y: usize) -> String {
    snapshot
        .line(y)
        .iter()
        .map(|cell| cell.ch)
        .collect::<String>()
        .trim_end()
        .to_string()
}

#[test]
fn feeds_text_and_moves_cursor() {
    let mut emulator = AlacrittyEmulator::new(20, 5);
    emulator.feed(b"hello");
    let snapshot = emulator.snapshot();
    assert_eq!(line_text(&snapshot, 0), "hello");
    assert_eq!(snapshot.cursor.x, 5);
    assert_eq!(snapshot.cursor.y, 0);
}

#[test]
fn sgr_sets_bold_and_red() {
    let mut emulator = AlacrittyEmulator::new(20, 5);
    emulator.feed(b"\x1b[1;31mX");
    let snapshot = emulator.snapshot();
    let cell = snapshot.cell(0, 0);
    assert_eq!(cell.ch, 'X');
    assert_eq!(cell.fg, Color::Indexed(1));
    assert!(cell.attrs.contains(Attrs::BOLD));
}

#[test]
fn wide_char_has_spacer() {
    let mut emulator = AlacrittyEmulator::new(20, 5);
    emulator.feed("界".as_bytes());
    let snapshot = emulator.snapshot();
    assert_eq!(snapshot.cell(0, 0).ch, '界');
    assert!(snapshot.cell(0, 0).attrs.contains(Attrs::WIDE));
    assert!(snapshot.cell(1, 0).attrs.contains(Attrs::WIDE_SPACER));
}

#[test]
fn resize_keeps_text() {
    let mut emulator = AlacrittyEmulator::new(20, 5);
    emulator.feed(b"abc");
    emulator.resize(10, 2);
    let snapshot = emulator.snapshot();
    assert_eq!(snapshot.cols, 10);
    assert_eq!(snapshot.rows, 2);
    assert_eq!(line_text(&snapshot, 0), "abc");
}

#[test]
fn reports_cursor_position() {
    let mut emulator = AlacrittyEmulator::new(20, 5);
    emulator.feed(b"\x1b[2;3H");
    emulator.feed(b"\x1b[6n");
    assert_eq!(emulator.take_responses(), b"\x1b[2;3R".to_vec());
}

#[test]
fn toggles_alt_screen() {
    let mut emulator = AlacrittyEmulator::new(20, 5);
    emulator.feed(b"\x1b[?1049h");
    assert!(emulator.snapshot().alt_screen);
    emulator.feed(b"\x1b[?1049l");
    assert!(!emulator.snapshot().alt_screen);
}

#[test]
fn captures_title() {
    let mut emulator = AlacrittyEmulator::new(20, 5);
    emulator.feed(b"\x1b]0;hello\x07");
    assert_eq!(emulator.snapshot().title.as_deref(), Some("hello"));
}

#[test]
fn keeps_scrollback_and_scrolls() {
    let mut emulator = AlacrittyEmulator::new(10, 3);
    for line in 0..10 {
        emulator.feed(format!("line{line}\r\n").as_bytes());
    }
    let snapshot = emulator.snapshot();
    assert!(snapshot.history_size > 0);
    assert_eq!(snapshot.display_offset, 0);
    emulator.scroll(1);
    assert_eq!(emulator.snapshot().display_offset, 1);
}

#[test]
fn tab_advances_to_next_stop() {
    let mut emulator = AlacrittyEmulator::new(20, 5);
    emulator.feed(b"\t");
    assert_eq!(emulator.snapshot().cursor.x, 8);
}

#[test]
fn tracks_application_cursor_mode() {
    let mut emulator = AlacrittyEmulator::new(20, 5);
    assert!(!emulator.snapshot().app_cursor);
    emulator.feed(b"\x1b[?1h");
    assert!(emulator.snapshot().app_cursor);
    emulator.feed(b"\x1b[?1l");
    assert!(!emulator.snapshot().app_cursor);
}

#[test]
fn cursor_row_is_not_clamped_to_columns() {
    let mut emulator = AlacrittyEmulator::new(10, 30);
    emulator.feed(b"\x1b[20;1H");
    let snapshot = emulator.snapshot();
    assert_eq!(snapshot.cursor.x, 0);
    assert_eq!(snapshot.cursor.y, 19);
}
