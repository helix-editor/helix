use std::{
    borrow::Cow,
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
};

use alacritty_terminal::{
    event::{Event, EventListener},
    grid::Scroll,
    term::{test::TermSize, Config},
    vte::ansi,
    Term,
};
use helix_graphics::{graphics::CursorKind, theme::Color};
use helix_input::input::{KeyCode, MouseEvent, MouseEventKind};

use crate::pty::{PtyEvent, TerminalId, TerminalRegistry};
use tokio::{select, sync::mpsc};

pub fn cursor_kind_from_ansi(shape: ansi::CursorShape) -> CursorKind {
    match shape {
        ansi::CursorShape::Block => CursorKind::Block,
        ansi::CursorShape::Underline => CursorKind::Underline,
        ansi::CursorShape::Beam => CursorKind::Bar,
        ansi::CursorShape::HollowBlock => CursorKind::Block,
        ansi::CursorShape::Hidden => CursorKind::Hidden,
    }
}

pub fn color_from_ansi(col: ansi::Color) -> Color {
    match col {
        ansi::Color::Named(named) => match named {
            ansi::NamedColor::Black => Color::Black,
            ansi::NamedColor::Red => Color::Red,
            ansi::NamedColor::Green => Color::Green,
            ansi::NamedColor::Yellow => Color::Yellow,
            ansi::NamedColor::Blue => Color::Blue,
            ansi::NamedColor::Magenta => Color::Magenta,
            ansi::NamedColor::Cyan => Color::Cyan,
            ansi::NamedColor::White => Color::White,
            ansi::NamedColor::BrightBlack => Color::Gray,
            ansi::NamedColor::BrightRed => Color::LightRed,
            ansi::NamedColor::BrightGreen => Color::LightGreen,
            ansi::NamedColor::BrightYellow => Color::LightYellow,
            ansi::NamedColor::BrightBlue => Color::LightBlue,
            ansi::NamedColor::BrightMagenta => Color::LightMagenta,
            ansi::NamedColor::BrightCyan => Color::LightCyan,
            _ => Color::Reset,
        },
        ansi::Color::Spec(c) => Color::Rgb(c.r, c.g, c.b),
        ansi::Color::Indexed(idx) => Color::Indexed(idx),
    }
}

pub struct Listener {
    term_id: TerminalId,
    sender: mpsc::UnboundedSender<(TerminalId, Event)>,
}

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        let _ = self.sender.send((self.term_id, event));
    }
}

#[derive(Debug, Clone)]
pub enum TerminalEvent {
    TitleChange(TerminalId, String),
    Update(TerminalId),
}

pub enum TerminalState {
    Initializing,
    Normal,
    Failed(String),
    Terminated(i32),
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum ChordState {
    Insert,
    Normal,
}

impl std::fmt::Display for ChordState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            ChordState::Insert => "INS",
            ChordState::Normal => "NOR",
        };
        write!(f, "{}", text)
    }
}

struct TerminalModel {
    state: TerminalState,
    parser: ansi::Processor,
    term: Term<Listener>,
}

impl TerminalModel {
    #[inline]
    fn advance<D: IntoIterator<Item = u8>>(&mut self, data: D) {
        for b in data {
            self.parser.advance(&mut self.term, &[b]);
        }
    }

    #[inline]
    fn resize(&mut self, size: (u16, u16)) {
        self.term.resize(TermSize::new(size.1 as _, size.0 as _));
    }
}

fn resolve_key_event(mut key: helix_input::input::KeyEvent) -> Option<&'static str> {
    use helix_input::input::KeyModifiers;

    key.modifiers =
        (KeyModifiers::ALT | KeyModifiers::CONTROL | KeyModifiers::SHIFT) & key.modifiers;

    // Generates a `Modifiers` value to check against.
    macro_rules! modifiers {
            (ctrl) => {
                KeyModifiers::CONTROL
            };

            (alt) => {
                KeyModifiers::ALT
            };

            (shift) => {
                KeyModifiers::SHIFT
            };

            ($mod:ident $(| $($mods:ident)|+)?) => {
                modifiers!($mod) $(| modifiers!($($mods)|+) )?
            };
        }

    // Generates modifier values for ANSI sequences.
    macro_rules! modval {
        (shift) => {
            // 1
            "2"
        };
        (alt) => {
            // 2
            "3"
        };
        (alt | shift) => {
            // 1 + 2
            "4"
        };
        (ctrl) => {
            // 4
            "5"
        };
        (ctrl | shift) => {
            // 1 + 4
            "6"
        };
        (alt | ctrl) => {
            // 2 + 4
            "7"
        };
        (alt | ctrl | shift) => {
            // 1 + 2 + 4
            "8"
        };
    }

    // Generates ANSI sequences to move the cursor by one position.
    macro_rules! term_sequence {
            // Generate every modifier combination (except meta)
            ([all], $evt:ident, $no_mod:literal, $pre:literal, $post:literal) => {
                {
                    term_sequence!([], $evt, $no_mod);
                    term_sequence!([shift, alt, ctrl], $evt, $pre, $post);
                    term_sequence!([alt | shift, ctrl | shift, alt | ctrl], $evt, $pre, $post);
                    term_sequence!([alt | ctrl | shift], $evt, $pre, $post);
                    return None;
                }
            };
            // No modifiers
            ([], $evt:ident, $no_mod:literal) => {
                if $evt.modifiers.is_empty() {
                    return Some($no_mod);
                }
            };
            // A single modifier combination
            ([$($mod:ident)|+], $evt:ident, $pre:literal, $post:literal) => {
                if $evt.modifiers == modifiers!($($mod)|+) {
                    return Some(concat!($pre, modval!($($mod)|+), $post));
                }
            };
            // Break down multiple modifiers into a series of single combination branches
            ([$($($mod:ident)|+),+], $evt:ident, $pre:literal, $post:literal) => {
                $(
                    term_sequence!([$($mod)|+], $evt, $pre, $post);
                )+
            };
        }

    match key.code {
        helix_input::input::KeyCode::Char(c) => {
            if key.modifiers == KeyModifiers::CONTROL {
                // Convert the character into its index (into a control character).
                // In essence, this turns `ctrl+h` into `^h`
                let str = match c {
                    '@' => "\x00",
                    'a' => "\x01",
                    'b' => "\x02",
                    'c' => "\x03",
                    'd' => "\x04",
                    'e' => "\x05",
                    'f' => "\x06",
                    'g' => "\x07",
                    'h' => "\x08",
                    'i' => "\x09",
                    'j' => "\x0a",
                    'k' => "\x0b",
                    'l' => "\x0c",
                    'm' => "\x0d",
                    'n' => "\x0e",
                    'o' => "\x0f",
                    'p' => "\x10",
                    'q' => "\x11",
                    'r' => "\x12",
                    's' => "\x13",
                    't' => "\x14",
                    'u' => "\x15",
                    'v' => "\x16",
                    'w' => "\x17",
                    'x' => "\x18",
                    'y' => "\x19",
                    'z' => "\x1a",
                    '[' => "\x1b",
                    '\\' => "\x1c",
                    ']' => "\x1d",
                    '^' => "\x1e",
                    '_' => "\x1f",
                    _ => return None,
                };

                Some(str)
            } else {
                None
            }
        }

        helix_input::input::KeyCode::Backspace => {
            Some(if key.modifiers.contains(KeyModifiers::CONTROL) {
                "\x08" // backspace
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                "\x1b\x7f"
            } else {
                "\x7f"
            })
        }

        helix_input::input::KeyCode::Tab => Some("\x09"),
        helix_input::input::KeyCode::Enter => Some("\r"),
        helix_input::input::KeyCode::Esc => Some("\x1b"),

        // The following either expands to `\x1b[X` or `\x1b[1;NX` where N is a modifier value
        helix_input::input::KeyCode::Up => term_sequence!([all], key, "\x1b[A", "\x1b[1;", "A"),
        helix_input::input::KeyCode::Down => term_sequence!([all], key, "\x1b[B", "\x1b[1;", "B"),
        helix_input::input::KeyCode::Right => term_sequence!([all], key, "\x1b[C", "\x1b[1;", "C"),
        helix_input::input::KeyCode::Left => term_sequence!([all], key, "\x1b[D", "\x1b[1;", "D"),
        helix_input::input::KeyCode::Home => term_sequence!([all], key, "\x1bOH", "\x1b[1;", "H"),
        helix_input::input::KeyCode::End => term_sequence!([all], key, "\x1bOF", "\x1b[1;", "F"),
        helix_input::input::KeyCode::Insert => {
            term_sequence!([all], key, "\x1b[2~", "\x1b[2;", "~")
        }
        helix_input::input::KeyCode::Delete => {
            term_sequence!([all], key, "\x1b[3~", "\x1b[3;", "~")
        }
        _ => None,
    }
}

pub struct TerminalView {
    config: Config,
    chord_state: ChordState,
    pub visible: bool,
    pub viewport: (u16, u16),
    active_term: Option<TerminalId>,
    events: mpsc::UnboundedReceiver<(TerminalId, Event)>,
    sender: mpsc::UnboundedSender<(TerminalId, Event)>,
    pub(crate) registry: TerminalRegistry,
    models: HashMap<TerminalId, RefCell<TerminalModel>>,
}

impl TerminalView {
    pub fn new() -> TerminalView {
        let (sender, events) = mpsc::unbounded_channel();

        Self {
            config: Config::default(),
            chord_state: ChordState::Insert,
            active_term: None,
            visible: false,
            viewport: (24, 80),
            events,
            sender,
            registry: TerminalRegistry::new(),
            models: Default::default(),
        }
    }

    pub fn terminals(&self) -> Vec<&TerminalId> {
        let mut terminals = self.models.keys().collect::<Vec<_>>();
        terminals.sort();
        terminals
    }

    pub fn chord_state(&self) -> ChordState {
        self.chord_state.clone()
    }

    pub fn spawn_shell(&mut self, size: (u16, u16)) {
        if let Ok(term_id) = self.registry.new_terminal(Default::default()) {
            let sender = self.sender.clone();
            let listener = Listener { term_id, sender };

            let size = TermSize::new(size.1 as _, size.0 as _);
            self.active_term = Some(term_id);
            self.models.insert(
                term_id,
                RefCell::new(TerminalModel {
                    state: TerminalState::Initializing,
                    parser: ansi::Processor::new(),
                    term: Term::new(self.config.clone(), &size, listener),
                }),
            );
        }
    }

    pub fn toggle_terminal(&mut self) {
        if self.active_term.is_none() {
            self.spawn_shell(self.viewport);
        }

        if let Some(term_id) = self.active_term {
            self.visible = !self.visible;
            let _ = self.sender.send((term_id, Event::Wakeup));
        }
    }

    #[inline]
    pub fn close_active_terminal(&mut self) {
        if let Some(term_id) = self.active_term {
            self.close_term(term_id);
            self.set_next_active();
            if self.active_term.is_none() {
                self.toggle_terminal();
            }
        }
    }

    #[inline]
    pub fn get_active(&'_ self) -> Option<(TerminalId, Ref<'_, Term<Listener>>)> {
        let id = self.active_term?;

        Some((id, self.get_term(id)?))
    }

    pub fn get_active_mut(&'_ mut self) -> Option<(TerminalId, RefMut<'_, Term<Listener>>)> {
        let id = self.active_term?;

        Some((id, self.get_term_mut(id)?))
    }

    pub fn set_next_active(&'_ mut self) {
        if let Some(key) = self.active_term.as_ref().and_then(|key| {
            self.terminals()
                .iter()
                .filter(|&&k| k > key)
                .min() // smallest key greater than the given one
                .copied()
        }) {
            self.active_term = Some(*key);
        } else {
            self.active_term = self.terminals().last().copied().copied();
        }
    }

    pub fn set_prev_active(&'_ mut self) {
        if let Some(key) = self.active_term.as_ref().and_then(|key| {
            self.terminals()
                .iter()
                .filter(|&&k| k < key)
                .max() // smallest key greater than the given one
                .copied()
        }) {
            self.active_term = Some(*key);
        } else {
            self.active_term = self.terminals().first().copied().copied();
        }
    }

    #[inline]
    pub fn get_term(&'_ self, id: TerminalId) -> Option<Ref<'_, Term<Listener>>> {
        self.models
            .get(&id)
            .map(|t| Ref::map(t.borrow(), |x| &x.term))
    }

    #[inline]
    pub fn get_term_mut(&'_ self, id: TerminalId) -> Option<RefMut<'_, Term<Listener>>> {
        self.models
            .get(&id)
            .map(|t| RefMut::map(t.borrow_mut(), |x| &mut x.term))
    }

    pub fn close_term(&mut self, id: TerminalId) {
        if let Some(mut term) = self.models.remove(&id) {
            if !matches!(
                term.get_mut().state,
                TerminalState::Failed(_) | TerminalState::Terminated(_)
            ) {
                let _ = self.registry.terminate(id);
            }

            drop(term)
        }
    }

    fn handle_key_event(
        &mut self,
        id: TerminalId,
        key: helix_input::input::KeyEvent,
    ) -> Result<(), crate::error::Error> {
        if let Some(ref mut term) = self.get_term_mut(id) {
            let point = term.grid().cursor.point;
            term.scroll_to_point(point);
        }
        // Handle modes
        match (self.chord_state(), key.code) {
            (ChordState::Insert, KeyCode::Esc) => {
                self.chord_state = ChordState::Normal;
                Ok(())
            }
            (ChordState::Normal, KeyCode::Esc) => {
                self.toggle_terminal();
                Ok(())
            }
            (ChordState::Normal, key) => match key {
                KeyCode::Char('d') => {
                    self.close_active_terminal();
                    Ok(())
                }
                KeyCode::Char('i') => {
                    self.chord_state = ChordState::Insert;
                    Ok(())
                }
                KeyCode::Char('n') => {
                    self.spawn_shell(self.viewport);
                    Ok(())
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.set_prev_active();
                    Ok(())
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.set_next_active();
                    Ok(())
                }
                _ => Ok(()),
            },
            (ChordState::Insert, _) => {
                if let Some(s) = resolve_key_event(key) {
                    self.registry.write(id, Cow::Borrowed(s.as_bytes()))?;
                } else if let helix_input::input::KeyCode::Char(ch) = key.code {
                    let mut tmp = [0u8; 4];
                    let s = ch.encode_utf8(&mut tmp);
                    self.registry.write(id, Cow::Owned(s.as_bytes().to_vec()))?;
                } else {
                    log::warn!("unhandled key event `{:?}`", key);
                }
                Ok(())
            }
        }
    }

    fn handle_input_event_sync(
        &mut self,
        id: TerminalId,
        event: &helix_input::input::Event,
    ) -> Result<(), crate::error::Error> {
        match event {
            helix_input::input::Event::FocusGained => (),
            helix_input::input::Event::FocusLost => (),
            helix_input::input::Event::Key(key) => self.handle_key_event(id, *key)?,
            helix_input::input::Event::Mouse(evt) => self.handle_mouse_event(id, *evt)?,
            helix_input::input::Event::Paste(content) => {
                self.registry
                    .write(id, Cow::Owned(content.as_bytes().to_vec()))?;
            }
            helix_input::input::Event::Resize(cols, rows) => {
                if let Some(term) = self.models.get_mut(&id) {
                    let size = (*rows, *cols);
                    self.viewport = size;
                    term.get_mut().resize(size);
                    let _ = self.registry.resize(id, *rows, *cols);
                }
            }
            helix_input::input::Event::IdleTimeout => (),
        }

        Ok(())
    }

    pub fn handle_input_event(&mut self, event: &helix_input::input::Event) -> bool {
        if let Some(id) = self.active_term {
            let _res = self.handle_input_event_sync(id, event);
            return true;
        }

        false
    }

    fn handle_mouse_event(
        &mut self,
        id: TerminalId,
        evt: MouseEvent,
    ) -> Result<(), crate::error::Error> {
        if let Some(mut term) = self.get_term_mut(id) {
            // let point = Point {
            //     line: Line(evt.row as i32),
            //     column: Column(evt.column as usize),
            // };

            // let button = match evt.kind {
            //     MouseEventKind::Down(button) | MouseEventKind::Up(button) | MouseEventKind::Drag(button) => match button {
            //         MouseButton::Left => AlacrittyMouseButton::Left,
            //         MouseButton::Right => AlacrittyMouseButton::Right,
            //         MouseButton::Middle => AlacrittyMouseButton::Middle,
            //     },
            //     _ => AlacrittyMouseButton::Left,
            // };

            // let mut alacritty_modifiers = AlacrittyModifiers::empty();
            // if evt.modifiers.contains(KeyModifiers::SHIFT) {
            //     alacritty_modifiers.insert(AlacrittyModifiers::SHIFT);
            // }
            // if evt.modifiers.contains(KeyModifiers::ALT) {
            //     alacritty_modifiers.insert(AlacrittyModifiers::ALT);
            // }
            // if evt.modifiers.contains(KeyModifiers::CONTROL) {
            //     alacritty_modifiers.insert(AlacrittyModifiers::CONTROL);
            // }
            // if evt.modifiers.contains(KeyModifiers::SUPER) {
            //     alacritty_modifiers.insert(AlacrittyModifiers::SUPER);
            // }

            // let event = AlacrittyMouseEvent {
            //     point,
            //     button,
            //     modifiers: alacritty_modifiers,
            // };

            // term.sc

            match evt.kind {
                // MouseEventKind::Down(_) => term.mouse_input(&event, |d| { let _ = self.registry.write(id, Cow::from(d)); }),
                // MouseEventKind::Up(_) => term.mouse_input(&event, |d| { let _ = self.registry.write(id, Cow::from(d)); }),
                // MouseEventKind::Drag(_) => term.mouse_input(&event, |d| { let _ = self.registry.write(id, Cow::from(d)); }),
                // MouseEventKind::Moved => term.mouse_input(&event, |d| { let _ = self.registry.write(id, Cow::from(d)); }),
                MouseEventKind::ScrollDown => term.scroll_display(Scroll::Delta(-1)),
                MouseEventKind::ScrollUp => term.scroll_display(Scroll::Delta(1)),
                MouseEventKind::ScrollLeft => (),
                MouseEventKind::ScrollRight => (),
                _ => (),
            }
        }

        Ok(())
    }

    pub async fn poll_event(&mut self) -> Option<TerminalEvent> {
        select!(
            event = self.events.recv() => {
                let (id, event) = event?;

                match event {
                    Event::Wakeup => Some(TerminalEvent::Update(id)),
                    Event::Title(title) => Some(TerminalEvent::TitleChange(id, title)),
                    Event::PtyWrite(data) => {
                        let _ = self.registry.write(id, Cow::Owned(data.as_bytes().to_vec()));
                        None
                    }

                    // ResetTitle,
                    // ClipboardStore(ClipboardType, String),
                    // ClipboardLoad(ClipboardType, Arc<dyn Fn(&str) -> String + Sync + Send + 'static>),
                    // MouseCursorDirty => ,
                    // ColorRequest(usize, Arc<dyn Fn(Rgb) -> String + Sync + Send + 'static>),
                    // TextAreaSizeRequest(Arc<dyn Fn(WindowSize) -> String + Sync + Send + 'static>),
                    // CursorBlinkingChange,
                    // Wakeup,
                    // Bell,
                    // Exit,
                    _ => None
                }
            }

            event = self.registry.rx.recv() => {
                let (id, event) = event?;

                match event {
                    PtyEvent::UpdateTerminal(data) => {
                        self.models.get(&id)?.borrow_mut().advance(data);
                        Some(TerminalEvent::Update(id))
                    }
                    // PtyEvent::Error(err) => {
                    //     let term = self.models.get_mut(&id)?;
                    //     term.get_mut().state = TerminalState::Failed(err);
                    //     Some(TerminalEvent::Update(id))
                    // }
                    PtyEvent::TerminalStopped(code) => {
                        let term = self.models.get_mut(&id)?;
                        term.get_mut().state = TerminalState::Terminated(code.unwrap_or(0)); // TODO: Should 0 be the default here?
                        self.active_term = None;
                        self.visible = false;
                        Some(TerminalEvent::Update(id))
                    }
                }

            }
        )
    }
}

impl Drop for TerminalView {
    fn drop(&mut self) {
        let term_ids: Vec<u32> = self.models.keys().copied().collect();
        for id in term_ids {
            let _ = self.registry.terminate(id);
        }
    }
}

impl Default for TerminalView {
    fn default() -> Self {
        Self::new()
    }
}
