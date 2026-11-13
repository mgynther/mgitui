//! Decoding bytes read from the terminal into key presses.

use crate::log::{Log, LogLevel};

/// A key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A printable character.
    Char(char),
    /// Enter.
    Enter,
    /// Escape.
    Esc,
    /// Backspace.
    Backspace,
    /// A function key, `F(1)` for F1.
    F(u8),
    /// A control character, `Ctrl('c')` for Ctrl+C.
    Ctrl(char),
}

/// Decodes the bytes of one read from the terminal. A lone escape byte is the Esc key; an
/// escape byte followed by more bytes starts an escape sequence. Unknown sequences are logged
/// and skipped.
#[must_use]
pub fn decode(bytes: &[u8], log: &Log<'_>) -> Vec<Key> {
    let mut keys: Vec<Key> = Vec::new();
    let mut rest: &[u8] = bytes;
    while let Some((&first, tail)) = rest.split_first() {
        let used: usize = match first {
            0x1b => decode_escape(tail, &mut keys, log),
            b'\r' | b'\n' => push(&mut keys, Key::Enter),
            0x7f | 0x08 => push(&mut keys, Key::Backspace),
            0x00..=0x1f => push(&mut keys, Key::Ctrl(char::from(first + 0x60))),
            _ => decode_char(rest, &mut keys, log),
        };
        rest = rest.get(used..).unwrap_or_default();
    }
    keys
}

fn push(keys: &mut Vec<Key>, key: Key) -> usize {
    keys.push(key);
    1
}

/// Decodes an escape sequence whose bytes after the escape byte are `tail`. Returns the number
/// of bytes used, including the escape byte.
fn decode_escape(tail: &[u8], keys: &mut Vec<Key>, log: &Log<'_>) -> usize {
    let length: usize = match tail {
        [b'[', b'[', _, ..] => 3,
        [b'O', _, ..] => 2,
        [b'[', rest @ ..] => rest
            .iter()
            .position(|b: &u8| (0x40..=0x7e).contains(b))
            .map_or(rest.len() + 1, |end: usize| end + 2),
        _ => 0,
    };
    let sequence: &[u8] = tail.get(..length).unwrap_or(tail);
    match sequence {
        [] => keys.push(Key::Esc),
        b"OP" | b"[11~" | b"[[A" => keys.push(Key::F(1)),
        b"OQ" | b"[12~" | b"[[B" => keys.push(Key::F(2)),
        b"OR" | b"[13~" | b"[[C" => keys.push(Key::F(3)),
        _ => log(
            LogLevel::Debug,
            &format!(
                "unknown key sequence ESC {}",
                String::from_utf8_lossy(sequence)
            ),
        ),
    }
    1 + sequence.len()
}

/// Decodes one UTF-8 character at the start of `bytes`. Returns the number of bytes used.
fn decode_char(bytes: &[u8], keys: &mut Vec<Key>, log: &Log<'_>) -> usize {
    let length: usize = match bytes.first() {
        Some(0xc0..=0xdf) => 2,
        Some(0xe0..=0xef) => 3,
        Some(0xf0..=0xf7) => 4,
        _ => 1,
    };
    let encoded: &[u8] = bytes.get(..length).unwrap_or(bytes);
    match std::str::from_utf8(encoded)
        .ok()
        .and_then(|s: &str| s.chars().next())
    {
        Some(ch) => keys.push(Key::Char(ch)),
        None => log(LogLevel::Debug, &format!("invalid input bytes {encoded:?}")),
    }
    encoded.len()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::{Key, decode};
    use crate::log::LogLevel;

    fn keys(bytes: &[u8]) -> (Vec<Key>, Vec<String>) {
        let logged: RefCell<Vec<String>> = RefCell::new(Vec::new());
        let log = |level: LogLevel, message: &str| {
            logged.borrow_mut().push(format!("{level:?}: {message}"));
        };
        let decoded: Vec<Key> = decode(bytes, &log);
        (decoded, logged.into_inner())
    }

    #[test]
    fn plain_keys() {
        assert_eq!(
            keys(b"jk\r\n\x7f\x08\x03"),
            (
                vec![
                    Key::Char('j'),
                    Key::Char('k'),
                    Key::Enter,
                    Key::Enter,
                    Key::Backspace,
                    Key::Backspace,
                    Key::Ctrl('c'),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn unicode_characters() {
        assert_eq!(
            keys("ä€😀".as_bytes()),
            (
                vec![Key::Char('ä'), Key::Char('€'), Key::Char('😀')],
                vec![]
            )
        );
    }

    #[test]
    fn invalid_bytes_are_skipped() {
        assert_eq!(
            keys(b"\xffa\xc3"),
            (
                vec![Key::Char('a')],
                vec![
                    "Debug: invalid input bytes [255]".to_string(),
                    "Debug: invalid input bytes [195]".to_string(),
                ]
            )
        );
    }

    #[test]
    fn lone_escape_is_esc() {
        assert_eq!(keys(b"\x1b"), (vec![Key::Esc], vec![]));
        assert_eq!(keys(b"\x1bj"), (vec![Key::Esc, Key::Char('j')], vec![]));
    }

    #[test]
    fn function_keys_in_all_terminal_variants() {
        for (bytes, n) in [
            (&b"\x1bOP"[..], 1),
            (b"\x1b[11~", 1),
            (b"\x1b[[A", 1),
            (b"\x1bOQ", 2),
            (b"\x1b[12~", 2),
            (b"\x1b[[B", 2),
            (b"\x1bOR", 3),
            (b"\x1b[13~", 3),
            (b"\x1b[[C", 3),
        ] {
            assert_eq!(keys(bytes), (vec![Key::F(n)], vec![]), "{bytes:?}");
        }
    }

    #[test]
    fn unknown_sequences_are_logged_and_skipped() {
        assert_eq!(
            keys(b"\x1b[Aj\x1b[1;5"),
            (
                vec![Key::Char('j')],
                vec![
                    "Debug: unknown key sequence ESC [A".to_string(),
                    "Debug: unknown key sequence ESC [1;5".to_string(),
                ]
            )
        );
    }
}
