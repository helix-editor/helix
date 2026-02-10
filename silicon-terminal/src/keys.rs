use alacritty_terminal::term::TermMode;
use silicon_view::input::KeyEvent;
use silicon_view::keyboard::{KeyCode, KeyModifiers};

/// Convert a Silicon `KeyEvent` to the bytes that should be written to the PTY.
///
/// Returns `None` for keys that have no PTY representation (e.g. media keys).
pub fn to_esc_str(key: &KeyEvent, mode: TermMode) -> Option<Vec<u8>> {
    let app_cursor = mode.contains(TermMode::APP_CURSOR);

    let mods = key.modifiers;
    let ctrl = mods.contains(KeyModifiers::CONTROL);
    let alt = mods.contains(KeyModifiers::ALT);
    let shift = mods.contains(KeyModifiers::SHIFT);

    let bytes: Vec<u8> = match key.code {
        KeyCode::Char(c) => {
            if ctrl {
                // Ctrl+letter → control character
                let ch = c.to_ascii_lowercase();
                if ch.is_ascii_lowercase() {
                    let ctrl_char = (ch as u8) - b'a' + 1;
                    if alt {
                        vec![0x1b, ctrl_char]
                    } else {
                        vec![ctrl_char]
                    }
                } else {
                    match ch {
                        '@' | '`' | ' ' => {
                            if alt {
                                vec![0x1b, 0x00]
                            } else {
                                vec![0x00]
                            }
                        }
                        '[' => {
                            if alt {
                                vec![0x1b, 0x1b]
                            } else {
                                vec![0x1b]
                            }
                        }
                        '\\' => {
                            if alt {
                                vec![0x1b, 0x1c]
                            } else {
                                vec![0x1c]
                            }
                        }
                        ']' => {
                            if alt {
                                vec![0x1b, 0x1d]
                            } else {
                                vec![0x1d]
                            }
                        }
                        '^' | '~' => {
                            if alt {
                                vec![0x1b, 0x1e]
                            } else {
                                vec![0x1e]
                            }
                        }
                        '_' | '/' | '?' => {
                            if alt {
                                vec![0x1b, 0x1f]
                            } else {
                                vec![0x1f]
                            }
                        }
                        _ => {
                            let mut buf = [0u8; 4];
                            let s = c.encode_utf8(&mut buf);
                            let mut v = Vec::new();
                            if alt {
                                v.push(0x1b);
                            }
                            v.extend_from_slice(s.as_bytes());
                            v
                        }
                    }
                }
            } else if alt {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                let mut v = vec![0x1b];
                v.extend_from_slice(s.as_bytes());
                v
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
        }

        KeyCode::Enter => {
            if alt {
                vec![0x1b, 0x0d]
            } else {
                vec![0x0d]
            }
        }

        KeyCode::Backspace => {
            if ctrl {
                vec![0x08]
            } else if alt {
                vec![0x1b, 0x7f]
            } else {
                vec![0x7f]
            }
        }

        KeyCode::Tab => {
            if shift {
                b"\x1b[Z".to_vec()
            } else {
                vec![0x09]
            }
        }

        KeyCode::Esc => vec![0x1b],

        KeyCode::Up => arrow_key(b'A', app_cursor, ctrl, shift, alt),
        KeyCode::Down => arrow_key(b'B', app_cursor, ctrl, shift, alt),
        KeyCode::Right => arrow_key(b'C', app_cursor, ctrl, shift, alt),
        KeyCode::Left => arrow_key(b'D', app_cursor, ctrl, shift, alt),

        KeyCode::Home => {
            if app_cursor {
                b"\x1bOH".to_vec()
            } else {
                modified_csi(b'H', 1, ctrl, shift, alt)
            }
        }
        KeyCode::End => {
            if app_cursor {
                b"\x1bOF".to_vec()
            } else {
                modified_csi(b'F', 1, ctrl, shift, alt)
            }
        }

        KeyCode::Insert => tilde_key(2, ctrl, shift, alt),
        KeyCode::Delete => tilde_key(3, ctrl, shift, alt),
        KeyCode::PageUp => tilde_key(5, ctrl, shift, alt),
        KeyCode::PageDown => tilde_key(6, ctrl, shift, alt),

        KeyCode::F(n) => f_key(n, ctrl, shift, alt),

        KeyCode::Null => vec![0x00],

        // Keys with no PTY representation
        KeyCode::CapsLock
        | KeyCode::ScrollLock
        | KeyCode::NumLock
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::Menu => return None,

        // Media and modifier keys have no PTY representation
        _ => return None,
    };

    Some(bytes)
}

/// Generate arrow key sequence, respecting app cursor mode and modifiers.
fn arrow_key(code: u8, app_cursor: bool, ctrl: bool, shift: bool, alt: bool) -> Vec<u8> {
    let modifier = csi_modifier(ctrl, shift, alt);
    if modifier > 1 {
        // Modified arrow: \x1b[1;{mod}{code}
        format!("\x1b[1;{modifier}{}", code as char)
            .into_bytes()
    } else if app_cursor {
        vec![0x1b, b'O', code]
    } else {
        vec![0x1b, b'[', code]
    }
}

/// Generate a CSI sequence with possible modifier.
fn modified_csi(code: u8, base: u8, ctrl: bool, shift: bool, alt: bool) -> Vec<u8> {
    let modifier = csi_modifier(ctrl, shift, alt);
    if modifier > 1 {
        format!("\x1b[{base};{modifier}{}", code as char).into_bytes()
    } else {
        vec![0x1b, b'[', code]
    }
}

/// Generate a tilde-terminated CSI sequence: \x1b[{num}~ or \x1b[{num};{mod}~
fn tilde_key(num: u8, ctrl: bool, shift: bool, alt: bool) -> Vec<u8> {
    let modifier = csi_modifier(ctrl, shift, alt);
    if modifier > 1 {
        format!("\x1b[{num};{modifier}~").into_bytes()
    } else {
        format!("\x1b[{num}~").into_bytes()
    }
}

/// Generate function key sequence.
fn f_key(n: u8, ctrl: bool, shift: bool, alt: bool) -> Vec<u8> {
    let modifier = csi_modifier(ctrl, shift, alt);

    // F1-F4 use SS3 when unmodified, CSI with modifier otherwise
    match n {
        1 => {
            if modifier > 1 {
                format!("\x1b[1;{modifier}P").into_bytes()
            } else {
                b"\x1bOP".to_vec()
            }
        }
        2 => {
            if modifier > 1 {
                format!("\x1b[1;{modifier}Q").into_bytes()
            } else {
                b"\x1bOQ".to_vec()
            }
        }
        3 => {
            if modifier > 1 {
                format!("\x1b[1;{modifier}R").into_bytes()
            } else {
                b"\x1bOR".to_vec()
            }
        }
        4 => {
            if modifier > 1 {
                format!("\x1b[1;{modifier}S").into_bytes()
            } else {
                b"\x1bOS".to_vec()
            }
        }
        // F5-F12 use CSI with tilde
        5 => tilde_key(15, ctrl, shift, alt),
        6 => tilde_key(17, ctrl, shift, alt),
        7 => tilde_key(18, ctrl, shift, alt),
        8 => tilde_key(19, ctrl, shift, alt),
        9 => tilde_key(20, ctrl, shift, alt),
        10 => tilde_key(21, ctrl, shift, alt),
        11 => tilde_key(23, ctrl, shift, alt),
        12 => tilde_key(24, ctrl, shift, alt),
        // F13+ use extended codes
        _ => {
            let code = 24 + (n - 12);
            tilde_key(code, ctrl, shift, alt)
        }
    }
}

/// Compute the xterm modifier parameter for CSI sequences.
/// Returns 1 (no modifier) through 8 (ctrl+alt+shift).
fn csi_modifier(ctrl: bool, shift: bool, alt: bool) -> u8 {
    let mut m: u8 = 1;
    if shift {
        m += 1;
    }
    if alt {
        m += 2;
    }
    if ctrl {
        m += 4;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn key_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent { code, modifiers }
    }

    #[test]
    fn test_char() {
        assert_eq!(
            to_esc_str(&key(KeyCode::Char('a')), TermMode::empty()),
            Some(b"a".to_vec())
        );
    }

    #[test]
    fn test_ctrl_c() {
        assert_eq!(
            to_esc_str(
                &key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL),
                TermMode::empty()
            ),
            Some(vec![0x03])
        );
    }

    #[test]
    fn test_alt_char() {
        assert_eq!(
            to_esc_str(
                &key_mod(KeyCode::Char('x'), KeyModifiers::ALT),
                TermMode::empty()
            ),
            Some(vec![0x1b, b'x'])
        );
    }

    #[test]
    fn test_enter() {
        assert_eq!(
            to_esc_str(&key(KeyCode::Enter), TermMode::empty()),
            Some(vec![0x0d])
        );
    }

    #[test]
    fn test_escape() {
        assert_eq!(
            to_esc_str(&key(KeyCode::Esc), TermMode::empty()),
            Some(vec![0x1b])
        );
    }

    #[test]
    fn test_backspace() {
        assert_eq!(
            to_esc_str(&key(KeyCode::Backspace), TermMode::empty()),
            Some(vec![0x7f])
        );
    }

    #[test]
    fn test_tab() {
        assert_eq!(
            to_esc_str(&key(KeyCode::Tab), TermMode::empty()),
            Some(vec![0x09])
        );
    }

    #[test]
    fn test_shift_tab() {
        assert_eq!(
            to_esc_str(
                &key_mod(KeyCode::Tab, KeyModifiers::SHIFT),
                TermMode::empty()
            ),
            Some(b"\x1b[Z".to_vec())
        );
    }

    #[test]
    fn test_arrow_keys_normal() {
        assert_eq!(
            to_esc_str(&key(KeyCode::Up), TermMode::empty()),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            to_esc_str(&key(KeyCode::Down), TermMode::empty()),
            Some(b"\x1b[B".to_vec())
        );
        assert_eq!(
            to_esc_str(&key(KeyCode::Right), TermMode::empty()),
            Some(b"\x1b[C".to_vec())
        );
        assert_eq!(
            to_esc_str(&key(KeyCode::Left), TermMode::empty()),
            Some(b"\x1b[D".to_vec())
        );
    }

    #[test]
    fn test_arrow_keys_app_cursor() {
        assert_eq!(
            to_esc_str(&key(KeyCode::Up), TermMode::APP_CURSOR),
            Some(b"\x1bOA".to_vec())
        );
    }

    #[test]
    fn test_ctrl_arrow() {
        assert_eq!(
            to_esc_str(
                &key_mod(KeyCode::Right, KeyModifiers::CONTROL),
                TermMode::empty()
            ),
            Some(b"\x1b[1;5C".to_vec())
        );
    }

    #[test]
    fn test_f1() {
        assert_eq!(
            to_esc_str(&key(KeyCode::F(1)), TermMode::empty()),
            Some(b"\x1bOP".to_vec())
        );
    }

    #[test]
    fn test_f5() {
        assert_eq!(
            to_esc_str(&key(KeyCode::F(5)), TermMode::empty()),
            Some(b"\x1b[15~".to_vec())
        );
    }

    #[test]
    fn test_delete() {
        assert_eq!(
            to_esc_str(&key(KeyCode::Delete), TermMode::empty()),
            Some(b"\x1b[3~".to_vec())
        );
    }

    #[test]
    fn test_home_end() {
        assert_eq!(
            to_esc_str(&key(KeyCode::Home), TermMode::empty()),
            Some(b"\x1b[H".to_vec())
        );
        assert_eq!(
            to_esc_str(&key(KeyCode::End), TermMode::empty()),
            Some(b"\x1b[F".to_vec())
        );
    }

    #[test]
    fn test_page_up_down() {
        assert_eq!(
            to_esc_str(&key(KeyCode::PageUp), TermMode::empty()),
            Some(b"\x1b[5~".to_vec())
        );
        assert_eq!(
            to_esc_str(&key(KeyCode::PageDown), TermMode::empty()),
            Some(b"\x1b[6~".to_vec())
        );
    }

    #[test]
    fn test_media_keys_return_none() {
        assert_eq!(
            to_esc_str(&key(KeyCode::CapsLock), TermMode::empty()),
            None
        );
    }
}
