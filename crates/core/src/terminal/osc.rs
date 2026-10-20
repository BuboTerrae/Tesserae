use std::path::PathBuf;

const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;
const MAX_COMMAND: usize = 4;
const MAX_PAYLOAD: usize = 8192;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum State {
    #[default]
    Ground,
    Escape,
    Command,
    Payload,
    PayloadEscape,
    Ignore,
    IgnoreEscape,
}

#[derive(Debug, Default)]
pub(crate) struct Osc7Scanner {
    state: State,
    command: Vec<u8>,
    payload: Vec<u8>,
}

impl Osc7Scanner {
    pub(crate) fn feed(&mut self, bytes: &[u8]) -> Option<PathBuf> {
        let mut found = None;

        for &byte in bytes {
            match self.state {
                State::Ground => {
                    if byte == ESC {
                        self.state = State::Escape;
                    }
                }
                State::Escape => {
                    if byte == b']' {
                        self.command.clear();
                        self.payload.clear();
                        self.state = State::Command;
                    } else if byte != ESC {
                        self.state = State::Ground;
                    }
                }
                State::Command => match byte {
                    ESC => self.state = State::IgnoreEscape,
                    BEL => self.reset(),
                    b';' => {
                        if self.command.as_slice() == b"7" {
                            self.state = State::Payload;
                        } else {
                            self.state = State::Ignore;
                        }
                    }
                    b'0'..=b'9' if self.command.len() < MAX_COMMAND => {
                        self.command.push(byte);
                    }
                    _ => self.state = State::Ignore,
                },
                State::Payload => match byte {
                    BEL => {
                        if let Some(path) = parse_osc7(&self.payload) {
                            found = Some(path);
                        }
                        self.reset();
                    }
                    ESC => self.state = State::PayloadEscape,
                    _ if self.payload.len() < MAX_PAYLOAD => self.payload.push(byte),
                    _ => self.state = State::Ignore,
                },
                State::PayloadEscape => {
                    if byte == b'\\' {
                        if let Some(path) = parse_osc7(&self.payload) {
                            found = Some(path);
                        }
                        self.reset()
                    } else {
                        if self.payload.len() < MAX_PAYLOAD {
                            self.payload.push(ESC);
                        }
                        if byte == ESC {
                            continue;
                        }
                        if self.payload.len() < MAX_PAYLOAD {
                            self.payload.push(byte);
                        }
                        self.state = State::Payload
                    }
                }
                State::Ignore => {
                    if byte == BEL {
                        self.reset();
                    } else if byte == ESC {
                        self.state = State::IgnoreEscape;
                    }
                }
                State::IgnoreEscape => {
                    if byte == b'\\' {
                        self.reset();
                    } else if byte != ESC {
                        self.state = State::Ignore
                    }
                }
            }
        }

        found
    }

    fn reset(&mut self) {
        self.state = State::Ground;
        self.command.clear();
        self.payload.clear();
    }
}

fn parse_osc7(payload: &[u8]) -> Option<PathBuf> {
    let payload = std::str::from_utf8(payload).ok()?;
    let rest = payload.strip_prefix("file://")?;
    let slash = rest.find('/')?;
    let path = &rest[slash..];
    let decoded = percent_decode(path.as_bytes())?;
    let path = String::from_utf8(decoded).ok()?;
    let path = normalize_path(PathBuf::from(path));

    if path.is_absolute() { Some(path) } else { None }
}

fn normalize_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        let bytes = text.as_bytes();
        if bytes.len() >= 3 && bytes[0] == b'/' && bytes[2] == b':' {
            return PathBuf::from(&text[1..]);
        }
    }
    path
}

fn percent_decode(input: &[u8]) -> Option<Vec<u8>> {
    let mut output = Vec::with_capacity(input.len());
    let mut index = 0;

    while index < input.len() {
        if input[index] == b'%' {
            let high = hex_value(*input.get(index + 1)?)?;
            let low = hex_value(*input.get(index + 2)?)?;
            output.push((high << 4) | low);
            index += 3;
        } else {
            output.push(input[index]);
            index += 1;
        }
    }
    Some(output)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn scan(chunks: &[&[u8]]) -> Option<PathBuf> {
        let mut scanner = Osc7Scanner::default();
        let mut found = None;

        for chunk in chunks {
            if let Some(path) = scanner.feed(chunk) {
                found = Some(path);
            }
        }

        found
    }

    #[test]
    fn parses_bel_terminated_sequence() {
        assert_eq!(
            scan(&[b"\x1b]7;file:///tmp\x07"]),
            Some(PathBuf::from("/tmp"))
        );
    }

    #[test]
    fn parses_st_terminated_sequence() {
        assert_eq!(
            scan(&[b"\x1b]7;file://localhost/tmp\x1b\\"]),
            Some(PathBuf::from("/tmp"))
        );
    }

    #[test]
    fn handles_sequences_split_across_reads() {
        let input = b"\x1b]7;file:///tmp/hello%20world\x1b\\";
        let expected = Some(PathBuf::from("/tmp/hello world"));

        for split in 0..=input.len() {
            assert_eq!(
                scan(&[&input[..split], &input[split..]]),
                expected,
                "split at {split}"
            );
        }
    }

    #[test]
    fn ignores_other_osc_commands() {
        let mut scanner = Osc7Scanner::default();

        assert!(scanner.feed(b"\x1b]0;title\x07").is_none());
        assert_eq!(
            scanner.feed(b"\x1b]7;file:///tmp\x07"),
            Some(PathBuf::from("/tmp"))
        );
    }

    #[test]
    fn decodes_percent_escapes() {
        assert_eq!(
            percent_decode(b"/tmp/a%20b%2Fc"),
            Some(b"/tmp/a b/c".to_vec())
        );
        assert_eq!(percent_decode(b"%ZZ"), None);
    }

    #[test]
    fn rejects_malformed_urls() {
        assert_eq!(scan(&[b"\x1b]7;not-a-file-url\x07"]), None);
        assert_eq!(scan(&[b"\x1b]7;file://host-without-path\x07"]), None);
    }
}
