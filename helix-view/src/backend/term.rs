use crate::input::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

impl From<crossterm::event::Event> for Event {
    fn from(event: crossterm::event::Event) -> Self {
        match event {
            crossterm::event::Event::Key(key) => Self::Key(key.into()),
            crossterm::event::Event::Mouse(mouse) => Self::Mouse(mouse.into()),
            crossterm::event::Event::Resize(w, h) => Self::Resize(w, h),
        }
    }
}

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
