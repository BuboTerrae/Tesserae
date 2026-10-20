use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn encode_key(key: KeyEvent, app_cursor: bool) -> Vec<u8> {
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    let bytes = match key.code {
        KeyCode::Char(c) => {
            if ctrl {
                control_code(c)
            } else {
                let mut buf = [0u8; 4];
                c.encode_utf8(&mut buf).as_bytes().to_vec()
            }
        }
        KeyCode::Enter => b"\r".to_vec(),
        KeyCode::Tab => b"\t".to_vec(),
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => cursor_key(b'A', app_cursor),
        KeyCode::Down => cursor_key(b'B', app_cursor),
        KeyCode::Right => cursor_key(b'C', app_cursor),
        KeyCode::Left => cursor_key(b'D', app_cursor),
        KeyCode::Home => cursor_key(b'H', app_cursor),
        KeyCode::End => cursor_key(b'F', app_cursor),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::F(n) => function_key(n),
        _ => Vec::new(),
    };

    if alt && !bytes.is_empty() && key.code != KeyCode::Esc {
        let mut prefixed = Vec::with_capacity(bytes.len() + 1);
        prefixed.push(0x1b);
        prefixed.extend_from_slice(&bytes);
        prefixed
    } else {
        bytes
    }
}

fn cursor_key(letter: u8, app_cursor: bool) -> Vec<u8> {
    if app_cursor {
        vec![0x1b, b'O', letter]
    } else {
        vec![0x1b, b'[', letter]
    }
}

fn control_code(c: char) -> Vec<u8> {
    let lower = c.to_ascii_lowercase();
    match lower {
        'a'..='z' => vec![(lower as u8) & 0x1f],
        '@' | ' ' => vec![0x00],
        '[' => vec![0x1b],
        '\\' => vec![0x1c],
        ']' => vec![0x1d],
        '^' => vec![0x1e],
        '_' => vec![0x1f],
        '?' => vec![0x7f],
        _ => {
            let mut buf = [0u8; 4];
            lower.encode_utf8(&mut buf).as_bytes().to_vec()
        }
    }
}

fn function_key(n: u8) -> Vec<u8> {
    match n {
        1 => b"\x1bOP".to_vec(),
        2 => b"\x1bOQ".to_vec(),
        3 => b"\x1bOR".to_vec(),
        4 => b"\x1bOS".to_vec(),
        5 => b"\x1b[15~".to_vec(),
        6 => b"\x1b[17~".to_vec(),
        7 => b"\x1b[18~".to_vec(),
        8 => b"\x1b[19~".to_vec(),
        9 => b"\x1b[20~".to_vec(),
        10 => b"\x1b[21~".to_vec(),
        11 => b"\x1b[23~".to_vec(),
        12 => b"\x1b[24~".to_vec(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn with_mods(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn encodes_text_and_control_keys() {
        assert_eq!(encode_key(key(KeyCode::Char('a')), false), b"a".to_vec());
        assert_eq!(
            encode_key(with_mods(KeyCode::Char('c'), KeyModifiers::CONTROL), false),
            vec![0x03]
        );
        assert_eq!(encode_key(key(KeyCode::Enter), false), b"\r".to_vec());
        assert_eq!(encode_key(key(KeyCode::Backspace), false), vec![0x7f]);
    }

    #[test]
    fn encodes_arrows_for_both_cursor_modes() {
        assert_eq!(encode_key(key(KeyCode::Up), false), b"\x1b[A".to_vec());
        assert_eq!(encode_key(key(KeyCode::Up), true), b"\x1bOA".to_vec());
        assert_eq!(encode_key(key(KeyCode::Home), false), b"\x1b[H".to_vec());
        assert_eq!(encode_key(key(KeyCode::End), true), b"\x1bOF".to_vec());
    }

    #[test]
    fn encodes_alt_and_function_keys() {
        assert_eq!(
            encode_key(with_mods(KeyCode::Char('x'), KeyModifiers::ALT), false),
            b"\x1bx".to_vec()
        );
        assert_eq!(encode_key(key(KeyCode::F(5)), false), b"\x1b[15~".to_vec());
    }
}
