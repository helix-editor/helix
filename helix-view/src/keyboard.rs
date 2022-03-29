use bitflags::bitflags;

bitflags! {
    /// Represents key modifiers (shift, control, alt).
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct KeyModifiers: u8 {
        const SHIFT = 0b0000_0001;
        const CONTROL = 0b0000_0010;
        const ALT = 0b0000_0100;
        const NONE = 0b0000_0000;
    }
}

#[cfg(feature = "term")]
impl From<KeyModifiers> for crossterm::event::KeyModifiers {
    fn from(key_modifiers: KeyModifiers) -> Self {
        use crossterm::event::KeyModifiers as CKeyModifiers;

        let mut result = CKeyModifiers::NONE;

        if key_modifiers.contains(KeyModifiers::SHIFT) {
            result.insert(CKeyModifiers::SHIFT);
        }
        if key_modifiers.contains(KeyModifiers::CONTROL) {
            result.insert(CKeyModifiers::CONTROL);
        }
        if key_modifiers.contains(KeyModifiers::ALT) {
            result.insert(CKeyModifiers::ALT);
        }

        result
    }
}

#[cfg(feature = "term")]
impl From<crossterm::event::KeyModifiers> for KeyModifiers {
    fn from(val: crossterm::event::KeyModifiers) -> Self {
        use crossterm::event::KeyModifiers as CKeyModifiers;

        let mut result = KeyModifiers::NONE;

        if val.contains(CKeyModifiers::SHIFT) {
            result.insert(KeyModifiers::SHIFT);
        }
        if val.contains(CKeyModifiers::CONTROL) {
            result.insert(KeyModifiers::CONTROL);
        }
        if val.contains(CKeyModifiers::ALT) {
            result.insert(KeyModifiers::ALT);
        }

        result
    }
}

/// Represents a key.
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone, Copy, Hash)]
pub enum KeyCode {
    /// Backspace key.
    Backspace,
    /// Enter key.
    Enter,
    /// Left arrow key.
    Left,
    /// Right arrow key.
    Right,
    /// Up arrow key.
    Up,
    /// Down arrow key.
    Down,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page up key.
    PageUp,
    /// Page down key.
    PageDown,
    /// Tab key.
    Tab,
    /// Delete key.
    Delete,
    /// Insert key.
    Insert,
    /// F key.
    ///
    /// `KeyCode::F(1)` represents F1 key, etc.
    F(u8),
    /// A character.
    ///
    /// `KeyCode::Char('c')` represents `c` character, etc.
    Char(char),
    /// Null.
    Null,
    /// Escape key.
    Esc,
}

#[cfg(feature = "term")]
impl From<KeyCode> for crossterm::event::KeyCode {
    fn from(key_code: KeyCode) -> Self {
        use crossterm::event::KeyCode as CKeyCode;

        match key_code {
            KeyCode::Backspace => CKeyCode::Backspace,
            KeyCode::Enter => CKeyCode::Enter,
            KeyCode::Left => CKeyCode::Left,
            KeyCode::Right => CKeyCode::Right,
            KeyCode::Up => CKeyCode::Up,
            KeyCode::Down => CKeyCode::Down,
            KeyCode::Home => CKeyCode::Home,
            KeyCode::End => CKeyCode::End,
            KeyCode::PageUp => CKeyCode::PageUp,
            KeyCode::PageDown => CKeyCode::PageDown,
            KeyCode::Tab => CKeyCode::Tab,
            KeyCode::Delete => CKeyCode::Delete,
            KeyCode::Insert => CKeyCode::Insert,
            KeyCode::F(f_number) => CKeyCode::F(f_number),
            KeyCode::Char(character) => CKeyCode::Char(character),
            KeyCode::Null => CKeyCode::Null,
            KeyCode::Esc => CKeyCode::Esc,
        }
    }
}

#[cfg(feature = "term")]
impl From<crossterm::event::KeyCode> for KeyCode {
    fn from(val: crossterm::event::KeyCode) -> Self {
        use crossterm::event::KeyCode as CKeyCode;

        match val {
            CKeyCode::Backspace => KeyCode::Backspace,
            CKeyCode::Enter => KeyCode::Enter,
            CKeyCode::Left => KeyCode::Left,
            CKeyCode::Right => KeyCode::Right,
            CKeyCode::Up => KeyCode::Up,
            CKeyCode::Down => KeyCode::Down,
            CKeyCode::Home => KeyCode::Home,
            CKeyCode::End => KeyCode::End,
            CKeyCode::PageUp => KeyCode::PageUp,
            CKeyCode::PageDown => KeyCode::PageDown,
            CKeyCode::Tab => KeyCode::Tab,
            CKeyCode::BackTab => unreachable!("BackTab should have been handled on KeyEvent level"),
            CKeyCode::Delete => KeyCode::Delete,
            CKeyCode::Insert => KeyCode::Insert,
            CKeyCode::F(f_number) => KeyCode::F(f_number),
            CKeyCode::Char(character) => KeyCode::Char(character),
            CKeyCode::Null => KeyCode::Null,
            CKeyCode::Esc => KeyCode::Esc,
        }
    }
}
