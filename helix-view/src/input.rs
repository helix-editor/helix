//! Input event handling, currently backed by crossterm.
use anyhow::{anyhow, Error};
use helix_core::unicode::width::UnicodeWidthStr;
use serde::de::{self, Deserialize, Deserializer};
use std::fmt;

pub use crate::keyboard::{KeyCode, KeyModifiers};

#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash)]
pub struct MouseEvent {
    /// The kind of mouse event that was caused.
    pub kind: MouseEventKind,
    /// The column that the event occurred on.
    pub column: u16,
    /// The row that the event occurred on.
    pub row: u16,
    /// The key modifiers active when the event occurred.
    pub modifiers: KeyModifiers,
}

#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash)]
pub enum MouseEventKind {
    /// Pressed mouse button. Contains the button that was pressed.
    Down(MouseButton),
    /// Released mouse button. Contains the button that was released.
    Up(MouseButton),
    /// Moved the mouse cursor while pressing the contained mouse button.
    Drag(MouseButton),
    /// Moved the mouse cursor while not pressing a mouse button.
    Moved,
    /// Scrolled mouse wheel downwards (towards the user).
    ScrollDown,
    /// Scrolled mouse wheel upwards (away from the user).
    ScrollUp,
}

/// Represents a mouse button.
#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash)]
pub enum MouseButton {
    /// Left mouse button.
    Left,
    /// Right mouse button.
    Right,
    /// Middle mouse button.
    Middle,
}
/// Represents a key event.
// We use a newtype here because we want to customize Deserialize and Display.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    /// If a character was pressed, return it.
    pub fn char(&self) -> Option<char> {
        match self.code {
            KeyCode::Char(ch) => Some(ch),
            _ => None,
        }
    }
}

pub(crate) mod keys {
    pub(crate) const BACKSPACE: &str = "backspace";
    pub(crate) const ENTER: &str = "ret";
    pub(crate) const LEFT: &str = "left";
    pub(crate) const RIGHT: &str = "right";
    pub(crate) const UP: &str = "up";
    pub(crate) const DOWN: &str = "down";
    pub(crate) const HOME: &str = "home";
    pub(crate) const END: &str = "end";
    pub(crate) const PAGEUP: &str = "pageup";
    pub(crate) const PAGEDOWN: &str = "pagedown";
    pub(crate) const TAB: &str = "tab";
    pub(crate) const DELETE: &str = "del";
    pub(crate) const INSERT: &str = "ins";
    pub(crate) const NULL: &str = "null";
    pub(crate) const ESC: &str = "esc";
    pub(crate) const SPACE: &str = "space";
    pub(crate) const LESS_THAN: &str = "lt";
    pub(crate) const GREATER_THAN: &str = "gt";
    pub(crate) const PLUS: &str = "plus";
    pub(crate) const MINUS: &str = "minus";
    pub(crate) const SEMICOLON: &str = "semicolon";
    pub(crate) const PERCENT: &str = "percent";
}

impl fmt::Display for KeyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}{}{}",
            if self.modifiers.contains(KeyModifiers::SHIFT) {
                "S-"
            } else {
                ""
            },
            if self.modifiers.contains(KeyModifiers::ALT) {
                "A-"
            } else {
                ""
            },
            if self.modifiers.contains(KeyModifiers::CONTROL) {
                "C-"
            } else {
                ""
            },
        ))?;
        match self.code {
            KeyCode::Backspace => f.write_str(keys::BACKSPACE)?,
            KeyCode::Enter => f.write_str(keys::ENTER)?,
            KeyCode::Left => f.write_str(keys::LEFT)?,
            KeyCode::Right => f.write_str(keys::RIGHT)?,
            KeyCode::Up => f.write_str(keys::UP)?,
            KeyCode::Down => f.write_str(keys::DOWN)?,
            KeyCode::Home => f.write_str(keys::HOME)?,
            KeyCode::End => f.write_str(keys::END)?,
            KeyCode::PageUp => f.write_str(keys::PAGEUP)?,
            KeyCode::PageDown => f.write_str(keys::PAGEDOWN)?,
            KeyCode::Tab => f.write_str(keys::TAB)?,
            KeyCode::Delete => f.write_str(keys::DELETE)?,
            KeyCode::Insert => f.write_str(keys::INSERT)?,
            KeyCode::Null => f.write_str(keys::NULL)?,
            KeyCode::Esc => f.write_str(keys::ESC)?,
            KeyCode::Char(' ') => f.write_str(keys::SPACE)?,
            KeyCode::Char('<') => f.write_str(keys::LESS_THAN)?,
            KeyCode::Char('>') => f.write_str(keys::GREATER_THAN)?,
            KeyCode::Char('+') => f.write_str(keys::PLUS)?,
            KeyCode::Char('-') => f.write_str(keys::MINUS)?,
            KeyCode::Char(';') => f.write_str(keys::SEMICOLON)?,
            KeyCode::Char('%') => f.write_str(keys::PERCENT)?,
            KeyCode::F(i) => f.write_fmt(format_args!("F{}", i))?,
            KeyCode::Char(c) => f.write_fmt(format_args!("{}", c))?,
        };
        Ok(())
    }
}

impl UnicodeWidthStr for KeyEvent {
    fn width(&self) -> usize {
        use helix_core::unicode::width::UnicodeWidthChar;
        let mut width = match self.code {
            KeyCode::Backspace => keys::BACKSPACE.len(),
            KeyCode::Enter => keys::ENTER.len(),
            KeyCode::Left => keys::LEFT.len(),
            KeyCode::Right => keys::RIGHT.len(),
            KeyCode::Up => keys::UP.len(),
            KeyCode::Down => keys::DOWN.len(),
            KeyCode::Home => keys::HOME.len(),
            KeyCode::End => keys::END.len(),
            KeyCode::PageUp => keys::PAGEUP.len(),
            KeyCode::PageDown => keys::PAGEDOWN.len(),
            KeyCode::Tab => keys::TAB.len(),
            KeyCode::Delete => keys::DELETE.len(),
            KeyCode::Insert => keys::INSERT.len(),
            KeyCode::Null => keys::NULL.len(),
            KeyCode::Esc => keys::ESC.len(),
            KeyCode::Char(' ') => keys::SPACE.len(),
            KeyCode::Char('<') => keys::LESS_THAN.len(),
            KeyCode::Char('>') => keys::GREATER_THAN.len(),
            KeyCode::Char('+') => keys::PLUS.len(),
            KeyCode::Char('-') => keys::MINUS.len(),
            KeyCode::Char(';') => keys::SEMICOLON.len(),
            KeyCode::Char('%') => keys::PERCENT.len(),
            KeyCode::F(1..=9) => 2,
            KeyCode::F(_) => 3,
            KeyCode::Char(c) => c.width().unwrap_or(0),
        };
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            width += 2;
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            width += 2;
        }
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            width += 2;
        }
        width
    }

    fn width_cjk(&self) -> usize {
        self.width()
    }
}

impl std::str::FromStr for KeyEvent {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens: Vec<_> = s.split('-').collect();
        let code = match tokens.pop().ok_or_else(|| anyhow!("Missing key code"))? {
            keys::BACKSPACE => KeyCode::Backspace,
            keys::ENTER => KeyCode::Enter,
            keys::LEFT => KeyCode::Left,
            keys::RIGHT => KeyCode::Right,
            keys::UP => KeyCode::Up,
            keys::DOWN => KeyCode::Down,
            keys::HOME => KeyCode::Home,
            keys::END => KeyCode::End,
            keys::PAGEUP => KeyCode::PageUp,
            keys::PAGEDOWN => KeyCode::PageDown,
            keys::TAB => KeyCode::Tab,
            keys::DELETE => KeyCode::Delete,
            keys::INSERT => KeyCode::Insert,
            keys::NULL => KeyCode::Null,
            keys::ESC => KeyCode::Esc,
            keys::SPACE => KeyCode::Char(' '),
            keys::LESS_THAN => KeyCode::Char('<'),
            keys::GREATER_THAN => KeyCode::Char('>'),
            keys::PLUS => KeyCode::Char('+'),
            keys::MINUS => KeyCode::Char('-'),
            keys::SEMICOLON => KeyCode::Char(';'),
            keys::PERCENT => KeyCode::Char('%'),
            single if single.chars().count() == 1 => KeyCode::Char(single.chars().next().unwrap()),
            function if function.len() > 1 && function.starts_with('F') => {
                let function: String = function.chars().skip(1).collect();
                let function = str::parse::<u8>(&function)?;
                (function > 0 && function < 13)
                    .then(|| KeyCode::F(function))
                    .ok_or_else(|| anyhow!("Invalid function key '{}'", function))?
            }
            invalid => return Err(anyhow!("Invalid key code '{}'", invalid)),
        };

        let mut modifiers = KeyModifiers::empty();
        for token in tokens {
            let flag = match token {
                "S" => KeyModifiers::SHIFT,
                "A" => KeyModifiers::ALT,
                "C" => KeyModifiers::CONTROL,
                _ => return Err(anyhow!("Invalid key modifier '{}-'", token)),
            };

            if modifiers.contains(flag) {
                return Err(anyhow!("Repeated key modifier '{}-'", token));
            }
            modifiers.insert(flag);
        }

        Ok(KeyEvent { code, modifiers })
    }
}

impl<'de> Deserialize<'de> for KeyEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

#[cfg(feature = "term")]
impl From<crossterm::event::Event> for Event {
    fn from(event: crossterm::event::Event) -> Self {
        match event {
            crossterm::event::Event::Key(key) => Self::Key(key.into()),
            crossterm::event::Event::Mouse(mouse) => Self::Mouse(mouse.into()),
            crossterm::event::Event::Resize(w, h) => Self::Resize(w, h),
        }
    }
}

#[cfg(feature = "term")]
impl From<crossterm::event::MouseEvent> for MouseEvent {
    fn from(
        crossterm::event::MouseEvent {
            kind,
            column,
            row,
            modifiers,
        }: crossterm::event::MouseEvent,
    ) -> Self {
        Self {
            kind: kind.into(),
            column,
            row,
            modifiers: modifiers.into(),
        }
    }
}

#[cfg(feature = "term")]
impl From<crossterm::event::MouseEventKind> for MouseEventKind {
    fn from(kind: crossterm::event::MouseEventKind) -> Self {
        match kind {
            crossterm::event::MouseEventKind::Down(button) => Self::Down(button.into()),
            crossterm::event::MouseEventKind::Up(button) => Self::Down(button.into()),
            crossterm::event::MouseEventKind::Drag(button) => Self::Drag(button.into()),
            crossterm::event::MouseEventKind::Moved => Self::Moved,
            crossterm::event::MouseEventKind::ScrollDown => Self::ScrollDown,
            crossterm::event::MouseEventKind::ScrollUp => Self::ScrollUp,
        }
    }
}

#[cfg(feature = "term")]
impl From<crossterm::event::MouseButton> for MouseButton {
    fn from(button: crossterm::event::MouseButton) -> Self {
        match button {
            crossterm::event::MouseButton::Left => MouseButton::Left,
            crossterm::event::MouseButton::Right => MouseButton::Right,
            crossterm::event::MouseButton::Middle => MouseButton::Middle,
        }
    }
}

#[cfg(feature = "term")]
impl From<crossterm::event::KeyEvent> for KeyEvent {
    fn from(crossterm::event::KeyEvent { code, modifiers }: crossterm::event::KeyEvent) -> Self {
        if code == crossterm::event::KeyCode::BackTab {
            // special case for BackTab -> Shift-Tab
            let mut modifiers: KeyModifiers = modifiers.into();
            modifiers.insert(KeyModifiers::SHIFT);
            Self {
                code: KeyCode::Tab,
                modifiers,
            }
        } else {
            Self {
                code: code.into(),
                modifiers: modifiers.into(),
            }
        }
    }
}

#[cfg(feature = "term")]
impl From<KeyEvent> for crossterm::event::KeyEvent {
    fn from(KeyEvent { code, modifiers }: KeyEvent) -> Self {
        if code == KeyCode::Tab && modifiers.contains(KeyModifiers::SHIFT) {
            // special case for Shift-Tab -> BackTab
            let mut modifiers = modifiers;
            modifiers.remove(KeyModifiers::SHIFT);
            crossterm::event::KeyEvent {
                code: crossterm::event::KeyCode::BackTab,
                modifiers: modifiers.into(),
            }
        } else {
            crossterm::event::KeyEvent {
                code: code.into(),
                modifiers: modifiers.into(),
            }
        }
    }
}

pub fn parse_macro(keys_str: &str) -> anyhow::Result<Vec<KeyEvent>> {
    use anyhow::Context;
    let mut keys_res: anyhow::Result<_> = Ok(Vec::new());
    let mut i = 0;
    while let Ok(keys) = &mut keys_res {
        if i >= keys_str.len() {
            break;
        }
        if !keys_str.is_char_boundary(i) {
            i += 1;
            continue;
        }

        let s = &keys_str[i..];
        let mut end_i = 1;
        while !s.is_char_boundary(end_i) {
            end_i += 1;
        }
        let c = &s[..end_i];
        if c == ">" {
            keys_res = Err(anyhow!("Unmatched '>'"));
        } else if c != "<" {
            keys.push(c);
            i += end_i;
        } else {
            match s.find('>').context("'>' expected") {
                Ok(end_i) => {
                    keys.push(&s[1..end_i]);
                    i += end_i + 1;
                }
                Err(err) => keys_res = Err(err),
            }
        }
    }
    keys_res.and_then(|keys| keys.into_iter().map(str::parse).collect())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parsing_unmodified_keys() {
        assert_eq!(
            str::parse::<KeyEvent>("backspace").unwrap(),
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE
            }
        );

        assert_eq!(
            str::parse::<KeyEvent>("left").unwrap(),
            KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE
            }
        );

        assert_eq!(
            str::parse::<KeyEvent>(",").unwrap(),
            KeyEvent {
                code: KeyCode::Char(','),
                modifiers: KeyModifiers::NONE
            }
        );

        assert_eq!(
            str::parse::<KeyEvent>("w").unwrap(),
            KeyEvent {
                code: KeyCode::Char('w'),
                modifiers: KeyModifiers::NONE
            }
        );

        assert_eq!(
            str::parse::<KeyEvent>("F12").unwrap(),
            KeyEvent {
                code: KeyCode::F(12),
                modifiers: KeyModifiers::NONE
            }
        );
    }

    #[test]
    fn parsing_modified_keys() {
        assert_eq!(
            str::parse::<KeyEvent>("S-minus").unwrap(),
            KeyEvent {
                code: KeyCode::Char('-'),
                modifiers: KeyModifiers::SHIFT
            }
        );

        assert_eq!(
            str::parse::<KeyEvent>("C-A-S-F12").unwrap(),
            KeyEvent {
                code: KeyCode::F(12),
                modifiers: KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT
            }
        );

        assert_eq!(
            str::parse::<KeyEvent>("S-C-2").unwrap(),
            KeyEvent {
                code: KeyCode::Char('2'),
                modifiers: KeyModifiers::SHIFT | KeyModifiers::CONTROL
            }
        );
    }

    #[test]
    fn parsing_nonsensical_keys_fails() {
        assert!(str::parse::<KeyEvent>("F13").is_err());
        assert!(str::parse::<KeyEvent>("F0").is_err());
        assert!(str::parse::<KeyEvent>("aaa").is_err());
        assert!(str::parse::<KeyEvent>("S-S-a").is_err());
        assert!(str::parse::<KeyEvent>("C-A-S-C-1").is_err());
        assert!(str::parse::<KeyEvent>("FU").is_err());
        assert!(str::parse::<KeyEvent>("123").is_err());
        assert!(str::parse::<KeyEvent>("S--").is_err());
    }

    #[test]
    fn parsing_valid_macros() {
        assert_eq!(
            parse_macro("xdo").ok(),
            Some(vec![
                KeyEvent {
                    code: KeyCode::Char('x'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('o'),
                    modifiers: KeyModifiers::NONE,
                },
            ]),
        );

        assert_eq!(
            parse_macro("<C-w>v<C-w>h<C-o>xx<A-s>").ok(),
            Some(vec![
                KeyEvent {
                    code: KeyCode::Char('w'),
                    modifiers: KeyModifiers::CONTROL,
                },
                KeyEvent {
                    code: KeyCode::Char('v'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('w'),
                    modifiers: KeyModifiers::CONTROL,
                },
                KeyEvent {
                    code: KeyCode::Char('h'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('o'),
                    modifiers: KeyModifiers::CONTROL,
                },
                KeyEvent {
                    code: KeyCode::Char('x'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('x'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('s'),
                    modifiers: KeyModifiers::ALT,
                },
            ])
        );

        assert_eq!(
            parse_macro(":o foo.bar<ret>").ok(),
            Some(vec![
                KeyEvent {
                    code: KeyCode::Char(':'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('o'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char(' '),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('f'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('o'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('o'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('.'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('b'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('r'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE,
                },
            ])
        );
    }

    #[test]
    fn parsing_invalid_macros_fails() {
        assert!(parse_macro("abc<C-").is_err());
        assert!(parse_macro("abc>123").is_err());
        assert!(parse_macro("wd<foo>").is_err());
    }
}
