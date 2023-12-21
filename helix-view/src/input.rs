//! Input event handling, currently backed by crossterm.
use anyhow::{anyhow, Error};
use helix_core::unicode::{segmentation::UnicodeSegmentation, width::UnicodeWidthStr};
use serde::de::{self, Deserialize, Deserializer};
use std::fmt;

pub use crate::keyboard::{KeyCode, KeyModifiers, MediaKeyCode, ModifierKeyCode};

#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Hash)]
pub enum Event {
    FocusGained,
    FocusLost,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Paste(String),
    Resize(u16, u16),
    IdleTimeout,
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
    /// Scrolled mouse wheel leftwards.
    ScrollLeft,
    /// Scrolled mouse wheel rightwards.
    ScrollRight,
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
    // TODO: crossterm now supports kind & state if terminal supports kitty's extended protocol
}

impl KeyEvent {
    /// If a character was pressed, return it.
    pub fn char(&self) -> Option<char> {
        match self.code {
            KeyCode::Char(ch) => Some(ch),
            _ => None,
        }
    }

    /// Format the key in such a way that a concatenated sequence
    /// of keys can be read easily.
    ///
    /// ```
    /// # use std::str::FromStr;
    /// # use helix_view::input::KeyEvent;
    ///
    /// let k = KeyEvent::from_str("w").unwrap().key_sequence_format();
    /// assert_eq!(k, "w");
    ///
    /// let k = KeyEvent::from_str("C-w").unwrap().key_sequence_format();
    /// assert_eq!(k, "<C-w>");
    ///
    /// let k = KeyEvent::from_str(" ").unwrap().key_sequence_format();
    /// assert_eq!(k, "<space>");
    /// ```
    pub fn key_sequence_format(&self) -> String {
        let s = self.to_string();
        if s.graphemes(true).count() > 1 {
            format!("<{}>", s)
        } else {
            s
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
    pub(crate) const MINUS: &str = "minus";
    pub(crate) const LESS_THAN: &str = "lt";
    pub(crate) const GREATER_THAN: &str = "gt";
    pub(crate) const CAPS_LOCK: &str = "capslock";
    pub(crate) const SCROLL_LOCK: &str = "scrolllock";
    pub(crate) const NUM_LOCK: &str = "numlock";
    pub(crate) const PRINT_SCREEN: &str = "printscreen";
    pub(crate) const PAUSE: &str = "pause";
    pub(crate) const MENU: &str = "menu";
    pub(crate) const KEYPAD_BEGIN: &str = "keypadbegin";
    pub(crate) const PLAY: &str = "play";
    pub(crate) const PAUSE_MEDIA: &str = "pausemedia";
    pub(crate) const PLAY_PAUSE: &str = "playpause";
    pub(crate) const REVERSE: &str = "reverse";
    pub(crate) const STOP: &str = "stop";
    pub(crate) const FAST_FORWARD: &str = "fastforward";
    pub(crate) const REWIND: &str = "rewind";
    pub(crate) const TRACK_NEXT: &str = "tracknext";
    pub(crate) const TRACK_PREVIOUS: &str = "trackprevious";
    pub(crate) const RECORD: &str = "record";
    pub(crate) const LOWER_VOLUME: &str = "lowervolume";
    pub(crate) const RAISE_VOLUME: &str = "raisevolume";
    pub(crate) const MUTE_VOLUME: &str = "mutevolume";
    pub(crate) const LEFT_SHIFT: &str = "leftshift";
    pub(crate) const LEFT_CONTROL: &str = "leftcontrol";
    pub(crate) const LEFT_ALT: &str = "leftalt";
    pub(crate) const LEFT_SUPER: &str = "leftsuper";
    pub(crate) const LEFT_HYPER: &str = "lefthyper";
    pub(crate) const LEFT_META: &str = "leftmeta";
    pub(crate) const RIGHT_SHIFT: &str = "rightshift";
    pub(crate) const RIGHT_CONTROL: &str = "rightcontrol";
    pub(crate) const RIGHT_ALT: &str = "rightalt";
    pub(crate) const RIGHT_SUPER: &str = "rightsuper";
    pub(crate) const RIGHT_HYPER: &str = "righthyper";
    pub(crate) const RIGHT_META: &str = "rightmeta";
    pub(crate) const ISO_LEVEL_3_SHIFT: &str = "isolevel3shift";
    pub(crate) const ISO_LEVEL_5_SHIFT: &str = "isolevel5shift";
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
            KeyCode::Char('-') => f.write_str(keys::MINUS)?,
            KeyCode::Char('<') => f.write_str(keys::LESS_THAN)?,
            KeyCode::Char('>') => f.write_str(keys::GREATER_THAN)?,
            KeyCode::F(i) => f.write_fmt(format_args!("F{}", i))?,
            KeyCode::Char(c) => f.write_fmt(format_args!("{}", c))?,
            KeyCode::CapsLock => f.write_str(keys::CAPS_LOCK)?,
            KeyCode::ScrollLock => f.write_str(keys::SCROLL_LOCK)?,
            KeyCode::NumLock => f.write_str(keys::NUM_LOCK)?,
            KeyCode::PrintScreen => f.write_str(keys::PRINT_SCREEN)?,
            KeyCode::Pause => f.write_str(keys::PAUSE)?,
            KeyCode::Menu => f.write_str(keys::MENU)?,
            KeyCode::KeypadBegin => f.write_str(keys::KEYPAD_BEGIN)?,
            KeyCode::Media(MediaKeyCode::Play) => f.write_str(keys::PLAY)?,
            KeyCode::Media(MediaKeyCode::Pause) => f.write_str(keys::PAUSE_MEDIA)?,
            KeyCode::Media(MediaKeyCode::PlayPause) => f.write_str(keys::PLAY_PAUSE)?,
            KeyCode::Media(MediaKeyCode::Stop) => f.write_str(keys::STOP)?,
            KeyCode::Media(MediaKeyCode::Reverse) => f.write_str(keys::REVERSE)?,
            KeyCode::Media(MediaKeyCode::FastForward) => f.write_str(keys::FAST_FORWARD)?,
            KeyCode::Media(MediaKeyCode::Rewind) => f.write_str(keys::REWIND)?,
            KeyCode::Media(MediaKeyCode::TrackNext) => f.write_str(keys::TRACK_NEXT)?,
            KeyCode::Media(MediaKeyCode::TrackPrevious) => f.write_str(keys::TRACK_PREVIOUS)?,
            KeyCode::Media(MediaKeyCode::Record) => f.write_str(keys::RECORD)?,
            KeyCode::Media(MediaKeyCode::LowerVolume) => f.write_str(keys::LOWER_VOLUME)?,
            KeyCode::Media(MediaKeyCode::RaiseVolume) => f.write_str(keys::RAISE_VOLUME)?,
            KeyCode::Media(MediaKeyCode::MuteVolume) => f.write_str(keys::MUTE_VOLUME)?,
            KeyCode::Modifier(ModifierKeyCode::LeftShift) => f.write_str(keys::LEFT_SHIFT)?,
            KeyCode::Modifier(ModifierKeyCode::LeftControl) => f.write_str(keys::LEFT_CONTROL)?,
            KeyCode::Modifier(ModifierKeyCode::LeftAlt) => f.write_str(keys::LEFT_ALT)?,
            KeyCode::Modifier(ModifierKeyCode::LeftSuper) => f.write_str(keys::LEFT_SUPER)?,
            KeyCode::Modifier(ModifierKeyCode::LeftHyper) => f.write_str(keys::LEFT_HYPER)?,
            KeyCode::Modifier(ModifierKeyCode::LeftMeta) => f.write_str(keys::LEFT_META)?,
            KeyCode::Modifier(ModifierKeyCode::RightShift) => f.write_str(keys::RIGHT_SHIFT)?,
            KeyCode::Modifier(ModifierKeyCode::RightControl) => f.write_str(keys::RIGHT_CONTROL)?,
            KeyCode::Modifier(ModifierKeyCode::RightAlt) => f.write_str(keys::RIGHT_ALT)?,
            KeyCode::Modifier(ModifierKeyCode::RightSuper) => f.write_str(keys::RIGHT_SUPER)?,
            KeyCode::Modifier(ModifierKeyCode::RightHyper) => f.write_str(keys::RIGHT_HYPER)?,
            KeyCode::Modifier(ModifierKeyCode::RightMeta) => f.write_str(keys::RIGHT_META)?,
            KeyCode::Modifier(ModifierKeyCode::IsoLevel3Shift) => {
                f.write_str(keys::ISO_LEVEL_3_SHIFT)?
            }
            KeyCode::Modifier(ModifierKeyCode::IsoLevel5Shift) => {
                f.write_str(keys::ISO_LEVEL_5_SHIFT)?
            }
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
            KeyCode::Char('-') => keys::MINUS.len(),
            KeyCode::F(1..=9) => 2,
            KeyCode::F(_) => 3,
            KeyCode::Char(c) => c.width().unwrap_or(0),
            KeyCode::CapsLock => keys::CAPS_LOCK.len(),
            KeyCode::ScrollLock => keys::SCROLL_LOCK.len(),
            KeyCode::NumLock => keys::NUM_LOCK.len(),
            KeyCode::PrintScreen => keys::PRINT_SCREEN.len(),
            KeyCode::Pause => keys::PAUSE.len(),
            KeyCode::Menu => keys::MENU.len(),
            KeyCode::KeypadBegin => keys::KEYPAD_BEGIN.len(),
            KeyCode::Media(MediaKeyCode::Play) => keys::PLAY.len(),
            KeyCode::Media(MediaKeyCode::Pause) => keys::PAUSE_MEDIA.len(),
            KeyCode::Media(MediaKeyCode::PlayPause) => keys::PLAY_PAUSE.len(),
            KeyCode::Media(MediaKeyCode::Stop) => keys::STOP.len(),
            KeyCode::Media(MediaKeyCode::Reverse) => keys::REVERSE.len(),
            KeyCode::Media(MediaKeyCode::FastForward) => keys::FAST_FORWARD.len(),
            KeyCode::Media(MediaKeyCode::Rewind) => keys::REWIND.len(),
            KeyCode::Media(MediaKeyCode::TrackNext) => keys::TRACK_NEXT.len(),
            KeyCode::Media(MediaKeyCode::TrackPrevious) => keys::TRACK_PREVIOUS.len(),
            KeyCode::Media(MediaKeyCode::Record) => keys::RECORD.len(),
            KeyCode::Media(MediaKeyCode::LowerVolume) => keys::LOWER_VOLUME.len(),
            KeyCode::Media(MediaKeyCode::RaiseVolume) => keys::RAISE_VOLUME.len(),
            KeyCode::Media(MediaKeyCode::MuteVolume) => keys::MUTE_VOLUME.len(),
            KeyCode::Modifier(ModifierKeyCode::LeftShift) => keys::LEFT_SHIFT.len(),
            KeyCode::Modifier(ModifierKeyCode::LeftControl) => keys::LEFT_CONTROL.len(),
            KeyCode::Modifier(ModifierKeyCode::LeftAlt) => keys::LEFT_ALT.len(),
            KeyCode::Modifier(ModifierKeyCode::LeftSuper) => keys::LEFT_SUPER.len(),
            KeyCode::Modifier(ModifierKeyCode::LeftHyper) => keys::LEFT_HYPER.len(),
            KeyCode::Modifier(ModifierKeyCode::LeftMeta) => keys::LEFT_META.len(),
            KeyCode::Modifier(ModifierKeyCode::RightShift) => keys::RIGHT_SHIFT.len(),
            KeyCode::Modifier(ModifierKeyCode::RightControl) => keys::RIGHT_CONTROL.len(),
            KeyCode::Modifier(ModifierKeyCode::RightAlt) => keys::RIGHT_ALT.len(),
            KeyCode::Modifier(ModifierKeyCode::RightSuper) => keys::RIGHT_SUPER.len(),
            KeyCode::Modifier(ModifierKeyCode::RightHyper) => keys::RIGHT_HYPER.len(),
            KeyCode::Modifier(ModifierKeyCode::RightMeta) => keys::RIGHT_META.len(),
            KeyCode::Modifier(ModifierKeyCode::IsoLevel3Shift) => keys::ISO_LEVEL_3_SHIFT.len(),
            KeyCode::Modifier(ModifierKeyCode::IsoLevel5Shift) => keys::ISO_LEVEL_5_SHIFT.len(),
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
            keys::MINUS => KeyCode::Char('-'),
            keys::LESS_THAN => KeyCode::Char('<'),
            keys::GREATER_THAN => KeyCode::Char('>'),
            keys::CAPS_LOCK => KeyCode::CapsLock,
            keys::SCROLL_LOCK => KeyCode::ScrollLock,
            keys::NUM_LOCK => KeyCode::NumLock,
            keys::PRINT_SCREEN => KeyCode::PrintScreen,
            keys::PAUSE => KeyCode::Pause,
            keys::MENU => KeyCode::Menu,
            keys::KEYPAD_BEGIN => KeyCode::KeypadBegin,
            keys::PLAY => KeyCode::Media(MediaKeyCode::Play),
            keys::PAUSE_MEDIA => KeyCode::Media(MediaKeyCode::Pause),
            keys::PLAY_PAUSE => KeyCode::Media(MediaKeyCode::PlayPause),
            keys::STOP => KeyCode::Media(MediaKeyCode::Stop),
            keys::REVERSE => KeyCode::Media(MediaKeyCode::Reverse),
            keys::FAST_FORWARD => KeyCode::Media(MediaKeyCode::FastForward),
            keys::REWIND => KeyCode::Media(MediaKeyCode::Rewind),
            keys::TRACK_NEXT => KeyCode::Media(MediaKeyCode::TrackNext),
            keys::TRACK_PREVIOUS => KeyCode::Media(MediaKeyCode::TrackPrevious),
            keys::RECORD => KeyCode::Media(MediaKeyCode::Record),
            keys::LOWER_VOLUME => KeyCode::Media(MediaKeyCode::LowerVolume),
            keys::RAISE_VOLUME => KeyCode::Media(MediaKeyCode::RaiseVolume),
            keys::MUTE_VOLUME => KeyCode::Media(MediaKeyCode::MuteVolume),
            keys::LEFT_SHIFT => KeyCode::Modifier(ModifierKeyCode::LeftShift),
            keys::LEFT_CONTROL => KeyCode::Modifier(ModifierKeyCode::LeftControl),
            keys::LEFT_ALT => KeyCode::Modifier(ModifierKeyCode::LeftAlt),
            keys::LEFT_SUPER => KeyCode::Modifier(ModifierKeyCode::LeftSuper),
            keys::LEFT_HYPER => KeyCode::Modifier(ModifierKeyCode::LeftHyper),
            keys::LEFT_META => KeyCode::Modifier(ModifierKeyCode::LeftMeta),
            keys::RIGHT_SHIFT => KeyCode::Modifier(ModifierKeyCode::RightShift),
            keys::RIGHT_CONTROL => KeyCode::Modifier(ModifierKeyCode::RightControl),
            keys::RIGHT_ALT => KeyCode::Modifier(ModifierKeyCode::RightAlt),
            keys::RIGHT_SUPER => KeyCode::Modifier(ModifierKeyCode::RightSuper),
            keys::RIGHT_HYPER => KeyCode::Modifier(ModifierKeyCode::RightHyper),
            keys::RIGHT_META => KeyCode::Modifier(ModifierKeyCode::RightMeta),
            keys::ISO_LEVEL_3_SHIFT => KeyCode::Modifier(ModifierKeyCode::IsoLevel3Shift),
            keys::ISO_LEVEL_5_SHIFT => KeyCode::Modifier(ModifierKeyCode::IsoLevel5Shift),
            single if single.chars().count() == 1 => KeyCode::Char(single.chars().next().unwrap()),
            function if function.len() > 1 && function.starts_with('F') => {
                let function: String = function.chars().skip(1).collect();
                let function = str::parse::<u8>(&function)?;
                (function > 0 && function < 25)
                    .then_some(KeyCode::F(function))
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
            crossterm::event::Event::FocusGained => Self::FocusGained,
            crossterm::event::Event::FocusLost => Self::FocusLost,
            crossterm::event::Event::Paste(s) => Self::Paste(s),
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
            crossterm::event::MouseEventKind::Up(button) => Self::Up(button.into()),
            crossterm::event::MouseEventKind::Drag(button) => Self::Drag(button.into()),
            crossterm::event::MouseEventKind::Moved => Self::Moved,
            crossterm::event::MouseEventKind::ScrollDown => Self::ScrollDown,
            crossterm::event::MouseEventKind::ScrollUp => Self::ScrollUp,
            crossterm::event::MouseEventKind::ScrollLeft => Self::ScrollLeft,
            crossterm::event::MouseEventKind::ScrollRight => Self::ScrollRight,
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
    fn from(
        crossterm::event::KeyEvent {
            code, modifiers, ..
        }: crossterm::event::KeyEvent,
    ) -> Self {
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
                kind: crossterm::event::KeyEventKind::Press,
                state: crossterm::event::KeyEventState::NONE,
            }
        } else {
            crossterm::event::KeyEvent {
                code: code.into(),
                modifiers: modifiers.into(),
                kind: crossterm::event::KeyEventKind::Press,
                state: crossterm::event::KeyEventState::NONE,
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
            keys.push(if c == "-" { keys::MINUS } else { c });
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

        assert_eq!(
            str::parse::<KeyEvent>("%").unwrap(),
            KeyEvent {
                code: KeyCode::Char('%'),
                modifiers: KeyModifiers::NONE
            }
        );

        assert_eq!(
            str::parse::<KeyEvent>(";").unwrap(),
            KeyEvent {
                code: KeyCode::Char(';'),
                modifiers: KeyModifiers::NONE
            }
        );

        assert_eq!(
            str::parse::<KeyEvent>(">").unwrap(),
            KeyEvent {
                code: KeyCode::Char('>'),
                modifiers: KeyModifiers::NONE
            }
        );

        assert_eq!(
            str::parse::<KeyEvent>("<").unwrap(),
            KeyEvent {
                code: KeyCode::Char('<'),
                modifiers: KeyModifiers::NONE
            }
        );

        assert_eq!(
            str::parse::<KeyEvent>("+").unwrap(),
            KeyEvent {
                code: KeyCode::Char('+'),
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

        assert_eq!(
            str::parse::<KeyEvent>("A-C-+").unwrap(),
            KeyEvent {
                code: KeyCode::Char('+'),
                modifiers: KeyModifiers::ALT | KeyModifiers::CONTROL
            }
        );
    }

    #[test]
    fn parsing_nonsensical_keys_fails() {
        assert!(str::parse::<KeyEvent>("F25").is_err());
        assert!(str::parse::<KeyEvent>("F0").is_err());
        assert!(str::parse::<KeyEvent>("aaa").is_err());
        assert!(str::parse::<KeyEvent>("S-S-a").is_err());
        assert!(str::parse::<KeyEvent>("C-A-S-C-1").is_err());
        assert!(str::parse::<KeyEvent>("FU").is_err());
        assert!(str::parse::<KeyEvent>("123").is_err());
        assert!(str::parse::<KeyEvent>("S--").is_err());
        assert!(str::parse::<KeyEvent>("S-percent").is_err());
    }

    #[test]
    fn parsing_unsupported_named_keys() {
        assert!(str::parse::<KeyEvent>("plus").is_err());
        assert!(str::parse::<KeyEvent>("percent").is_err());
        assert!(str::parse::<KeyEvent>("semicolon").is_err());
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

        assert_eq!(
            parse_macro(":w aa-bb.txt<ret>").ok(),
            Some(vec![
                KeyEvent {
                    code: KeyCode::Char(':'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('w'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char(' '),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('-'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('b'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('b'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('.'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('t'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('x'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyEvent {
                    code: KeyCode::Char('t'),
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
