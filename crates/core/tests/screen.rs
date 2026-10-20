use tesserae_core::terminal::{Attrs, Color, Screen};

fn text(s: &Screen, y: usize) -> String {
    s.line(y).iter().map(|c| c.ch).collect()
}

#[test]
fn new_is_blank() {
    let s = Screen::new(10, 3);
    assert_eq!(text(&s, 0), "          ");
    assert_eq!(s.cursor(), (0, 0));
}

#[test]
fn prints_text() {
    let mut s = Screen::new(10, 3);
    for ch in "hello".chars() {
        s.put_char(ch);
    }
    assert_eq!(text(&s, 0), "hello     ");
    assert_eq!(s.cursor(), (5, 0));
}

#[test]
fn wraps_after_last_column() {
    let mut s = Screen::new(5, 2);
    for ch in "abcde".chars() {
        s.put_char(ch);
    }
    assert_eq!(s.cursor(), (4, 0));
    s.put_char('f');
    assert_eq!(text(&s, 1), "f    ");
    assert_eq!(s.cursor(), (1, 1));
}

#[test]
fn scrolls_on_newline_at_bottom() {
    let mut s = Screen::new(3, 2);
    for ch in "abcdef".chars() {
        s.put_char(ch)
    }
    s.carriage_return();
    s.newline();
    assert_eq!(text(&s, 0), "def");
    assert_eq!(text(&s, 1), "   ");
}

#[test]
fn erase_in_line_modes() {
    let mut s = Screen::new(5, 2);
    for ch in "abcde".chars() {
        s.put_char(ch);
    }
    for ch in "fghji".chars() {
        s.put_char(ch);
    }
    s.set_cursor(2, 0);

    s.erase_in_line(0);
    assert_eq!(text(&s, 0), "ab   ");
    assert_eq!(text(&s, 1), "fghji");

    s.erase_in_line(1);
    assert_eq!(text(&s, 0), "     ");
}

#[test]
fn erase_in_line_while_ignores_cursor() {
    let mut s = Screen::new(5, 1);
    for ch in "hello".chars() {
        s.put_char(ch);
    }
    s.set_cursor(0, 0);
    s.erase_in_line(2);
    assert_eq!(text(&s, 0), "     ");
}

#[test]
fn erase_in_display_mode_0() {
    let mut s = Screen::new(5, 2);
    for ch in "abcde".chars() {
        s.put_char(ch);
    }
    for ch in "fghji".chars() {
        s.put_char(ch);
    }
    s.set_cursor(2, 0);
    s.erase_in_display(0);
    assert_eq!(text(&s, 0), "ab   ");
    assert_eq!(text(&s, 1), "     ");
}

#[test]
fn erase_in_display_mode_1() {
    let mut s = Screen::new(5, 2);
    for ch in "abcde".chars() {
        s.put_char(ch);
    }
    for ch in "fghij".chars() {
        s.put_char(ch);
    }
    s.set_cursor(2, 1);
    s.erase_in_display(1);
    assert_eq!(text(&s, 0), "     ");
    assert_eq!(text(&s, 1), "   ij");
}

#[test]
fn resize_preserves_overlap_and_clamps_cursor() {
    let mut s = Screen::new(10, 4);
    for ch in "hello".chars() {
        s.put_char(ch);
    }
    s.set_cursor(9, 3);
    s.resize(3, 2);
    assert_eq!(s.cols(), 3);
    assert_eq!(s.rows(), 2);
    assert_eq!(text(&s, 0), "hel");
    assert_eq!(s.cursor(), (2, 1));
}

#[test]
fn sgr_sets_bold_and_color_on_new_cells() {
    let mut s = Screen::new(4, 1);
    s.apply_sgr(&[1, 31]);
    assert!(s.pen().attrs.contains(Attrs::BOLD));
    assert_eq!(s.pen().fg, Color::Indexed(1));

    s.put_char('x');
    assert!(s.cell(0, 0).attrs.contains(Attrs::BOLD));
    assert_eq!(s.cell(0, 0).fg, Color::Indexed(1));
}

#[test]
fn sgr_empty_params_resets() {
    let mut s = Screen::new(4, 1);
    s.apply_sgr(&[1, 31]);
    s.apply_sgr(&[]);
    assert_eq!(s.pen().fg, Color::Default);
    assert!(!s.pen().attrs.contains(Attrs::BOLD));
}

#[test]
fn sgr_zero_code_resets() {
    let mut s = Screen::new(4, 1);
    s.apply_sgr(&[1, 4, 31]);
    s.apply_sgr(&[0]);
    assert_eq!(s.pen().fg, Color::Default);
    assert!(!s.pen().attrs.contains(Attrs::UNDERLINE));
}

#[test]
fn sgr_bright_colors_map_to_high_indexed() {
    let mut s = Screen::new(4, 1);
    s.apply_sgr(&[91]);
    assert_eq!(s.pen().fg, Color::Indexed(9));
    s.apply_sgr(&[104]);
    assert_eq!(s.pen().bg, Color::Indexed(12));
}

#[test]
fn sgr_extend_256_color() {
    let mut s = Screen::new(4, 1);
    s.apply_sgr(&[38, 5, 200]);
    assert_eq!(s.pen().fg, Color::Indexed(200));
}

#[test]
fn sgr_extended_rgb_color() {
    let mut s = Screen::new(4, 1);
    s.apply_sgr(&[48, 2, 10, 20, 30]);
    assert_eq!(s.pen().bg, Color::Rgb(10, 20, 30));
}

#[test]
fn sgr_22_clears_bold_and_dim() {
    let mut s = Screen::new(4, 1);
    s.apply_sgr(&[1, 2]);
    s.apply_sgr(&[22]);
    assert!(!s.pen().attrs.contains(Attrs::BOLD));
    assert!(!s.pen().attrs.contains(Attrs::DIM));
}

#[test]
fn sgr_extended_color_does_not_swallow_trailing_params() {
    let mut s = Screen::new(4, 1);
    s.apply_sgr(&[38, 5, 200, 1]);
    assert_eq!(s.pen().fg, Color::Indexed(200));
    assert!(s.pen().attrs.contains(Attrs::BOLD));
}
