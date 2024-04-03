use bitflags::bitflags;

bitflags! {
    /// Represents key modifiers (shift, control, alt).
    #[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone, Copy, Hash)]
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

/// Represents a media key (as part of [`KeyCode::Media`]).
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone, Copy, Hash)]
pub enum MediaKeyCode {
    /// Play media key.
    Play,
    /// Pause media key.
    Pause,
    /// Play/Pause media key.
    PlayPause,
    /// Reverse media key.
    Reverse,
    /// Stop media key.
    Stop,
    /// Fast-forward media key.
    FastForward,
    /// Rewind media key.
    Rewind,
    /// Next-track media key.
    TrackNext,
    /// Previous-track media key.
    TrackPrevious,
    /// Record media key.
    Record,
    /// Lower-volume media key.
    LowerVolume,
    /// Raise-volume media key.
    RaiseVolume,
    /// Mute media key.
    MuteVolume,
}

#[cfg(feature = "term")]
impl From<MediaKeyCode> for crossterm::event::MediaKeyCode {
    fn from(media_key_code: MediaKeyCode) -> Self {
        use crossterm::event::MediaKeyCode as CMediaKeyCode;

        match media_key_code {
            MediaKeyCode::Play => CMediaKeyCode::Play,
            MediaKeyCode::Pause => CMediaKeyCode::Pause,
            MediaKeyCode::PlayPause => CMediaKeyCode::PlayPause,
            MediaKeyCode::Reverse => CMediaKeyCode::Reverse,
            MediaKeyCode::Stop => CMediaKeyCode::Stop,
            MediaKeyCode::FastForward => CMediaKeyCode::FastForward,
            MediaKeyCode::Rewind => CMediaKeyCode::Rewind,
            MediaKeyCode::TrackNext => CMediaKeyCode::TrackNext,
            MediaKeyCode::TrackPrevious => CMediaKeyCode::TrackPrevious,
            MediaKeyCode::Record => CMediaKeyCode::Record,
            MediaKeyCode::LowerVolume => CMediaKeyCode::LowerVolume,
            MediaKeyCode::RaiseVolume => CMediaKeyCode::RaiseVolume,
            MediaKeyCode::MuteVolume => CMediaKeyCode::MuteVolume,
        }
    }
}

#[cfg(feature = "term")]
impl From<crossterm::event::MediaKeyCode> for MediaKeyCode {
    fn from(val: crossterm::event::MediaKeyCode) -> Self {
        use crossterm::event::MediaKeyCode as CMediaKeyCode;

        match val {
            CMediaKeyCode::Play => MediaKeyCode::Play,
            CMediaKeyCode::Pause => MediaKeyCode::Pause,
            CMediaKeyCode::PlayPause => MediaKeyCode::PlayPause,
            CMediaKeyCode::Reverse => MediaKeyCode::Reverse,
            CMediaKeyCode::Stop => MediaKeyCode::Stop,
            CMediaKeyCode::FastForward => MediaKeyCode::FastForward,
            CMediaKeyCode::Rewind => MediaKeyCode::Rewind,
            CMediaKeyCode::TrackNext => MediaKeyCode::TrackNext,
            CMediaKeyCode::TrackPrevious => MediaKeyCode::TrackPrevious,
            CMediaKeyCode::Record => MediaKeyCode::Record,
            CMediaKeyCode::LowerVolume => MediaKeyCode::LowerVolume,
            CMediaKeyCode::RaiseVolume => MediaKeyCode::RaiseVolume,
            CMediaKeyCode::MuteVolume => MediaKeyCode::MuteVolume,
        }
    }
}

/// Represents a media key (as part of [`KeyCode::Modifier`]).
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone, Copy, Hash)]
pub enum ModifierKeyCode {
    /// Left Shift key.
    LeftShift,
    /// Left Control key.
    LeftControl,
    /// Left Alt key.
    LeftAlt,
    /// Left Super key.
    LeftSuper,
    /// Left Hyper key.
    LeftHyper,
    /// Left Meta key.
    LeftMeta,
    /// Right Shift key.
    RightShift,
    /// Right Control key.
    RightControl,
    /// Right Alt key.
    RightAlt,
    /// Right Super key.
    RightSuper,
    /// Right Hyper key.
    RightHyper,
    /// Right Meta key.
    RightMeta,
    /// Iso Level3 Shift key.
    IsoLevel3Shift,
    /// Iso Level5 Shift key.
    IsoLevel5Shift,
}

#[cfg(feature = "term")]
impl From<ModifierKeyCode> for crossterm::event::ModifierKeyCode {
    fn from(modifier_key_code: ModifierKeyCode) -> Self {
        use crossterm::event::ModifierKeyCode as CModifierKeyCode;

        match modifier_key_code {
            ModifierKeyCode::LeftShift => CModifierKeyCode::LeftShift,
            ModifierKeyCode::LeftControl => CModifierKeyCode::LeftControl,
            ModifierKeyCode::LeftAlt => CModifierKeyCode::LeftAlt,
            ModifierKeyCode::LeftSuper => CModifierKeyCode::LeftSuper,
            ModifierKeyCode::LeftHyper => CModifierKeyCode::LeftHyper,
            ModifierKeyCode::LeftMeta => CModifierKeyCode::LeftMeta,
            ModifierKeyCode::RightShift => CModifierKeyCode::RightShift,
            ModifierKeyCode::RightControl => CModifierKeyCode::RightControl,
            ModifierKeyCode::RightAlt => CModifierKeyCode::RightAlt,
            ModifierKeyCode::RightSuper => CModifierKeyCode::RightSuper,
            ModifierKeyCode::RightHyper => CModifierKeyCode::RightHyper,
            ModifierKeyCode::RightMeta => CModifierKeyCode::RightMeta,
            ModifierKeyCode::IsoLevel3Shift => CModifierKeyCode::IsoLevel3Shift,
            ModifierKeyCode::IsoLevel5Shift => CModifierKeyCode::IsoLevel5Shift,
        }
    }
}

#[cfg(feature = "term")]
impl From<crossterm::event::ModifierKeyCode> for ModifierKeyCode {
    fn from(val: crossterm::event::ModifierKeyCode) -> Self {
        use crossterm::event::ModifierKeyCode as CModifierKeyCode;

        match val {
            CModifierKeyCode::LeftShift => ModifierKeyCode::LeftShift,
            CModifierKeyCode::LeftControl => ModifierKeyCode::LeftControl,
            CModifierKeyCode::LeftAlt => ModifierKeyCode::LeftAlt,
            CModifierKeyCode::LeftSuper => ModifierKeyCode::LeftSuper,
            CModifierKeyCode::LeftHyper => ModifierKeyCode::LeftHyper,
            CModifierKeyCode::LeftMeta => ModifierKeyCode::LeftMeta,
            CModifierKeyCode::RightShift => ModifierKeyCode::RightShift,
            CModifierKeyCode::RightControl => ModifierKeyCode::RightControl,
            CModifierKeyCode::RightAlt => ModifierKeyCode::RightAlt,
            CModifierKeyCode::RightSuper => ModifierKeyCode::RightSuper,
            CModifierKeyCode::RightHyper => ModifierKeyCode::RightHyper,
            CModifierKeyCode::RightMeta => ModifierKeyCode::RightMeta,
            CModifierKeyCode::IsoLevel3Shift => ModifierKeyCode::IsoLevel3Shift,
            CModifierKeyCode::IsoLevel5Shift => ModifierKeyCode::IsoLevel5Shift,
        }
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
    /// CapsLock key.
    CapsLock,
    /// ScrollLock key.
    ScrollLock,
    /// NumLock key.
    NumLock,
    /// PrintScreen key.
    PrintScreen,
    /// Pause key.
    Pause,
    /// Menu key.
    Menu,
    /// KeypadBegin key.
    KeypadBegin,
    /// A media key.
    Media(MediaKeyCode),
    /// A modifier key.
    Modifier(ModifierKeyCode),
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
            KeyCode::CapsLock => CKeyCode::CapsLock,
            KeyCode::ScrollLock => CKeyCode::ScrollLock,
            KeyCode::NumLock => CKeyCode::NumLock,
            KeyCode::PrintScreen => CKeyCode::PrintScreen,
            KeyCode::Pause => CKeyCode::Pause,
            KeyCode::Menu => CKeyCode::Menu,
            KeyCode::KeypadBegin => CKeyCode::KeypadBegin,
            KeyCode::Media(media_key_code) => CKeyCode::Media(media_key_code.into()),
            KeyCode::Modifier(modifier_key_code) => CKeyCode::Modifier(modifier_key_code.into()),
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
            CKeyCode::CapsLock => KeyCode::CapsLock,
            CKeyCode::ScrollLock => KeyCode::ScrollLock,
            CKeyCode::NumLock => KeyCode::NumLock,
            CKeyCode::PrintScreen => KeyCode::PrintScreen,
            CKeyCode::Pause => KeyCode::Pause,
            CKeyCode::Menu => KeyCode::Menu,
            CKeyCode::KeypadBegin => KeyCode::KeypadBegin,
            CKeyCode::Media(media_key_code) => KeyCode::Media(media_key_code.into()),
            CKeyCode::Modifier(modifier_key_code) => KeyCode::Modifier(modifier_key_code.into()),
        }
    }
}
