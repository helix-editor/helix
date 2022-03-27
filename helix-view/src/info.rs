use crate::input::KeyEvent;
use helix_core::{register::Registers, unicode::width::UnicodeWidthStr};
use std::{collections::BTreeSet, fmt::Write};

#[derive(Debug, Clone)]
/// Info box used in editor. Rendering logic will be in other crate.
pub struct Info {
    /// Title shown at top.
    pub title: String,
    /// Text body, should contain newlines.
    pub text: String,
    /// Body width.
    pub width: u16,
    /// Body height.
    pub height: u16,
}

impl Info {
    pub fn new(title: &str, body: Vec<(String, String)>) -> Self {
        if body.is_empty() {
            return Self {
                title: title.to_string(),
                height: 1,
                width: title.len() as u16,
                text: "".to_string(),
            };
        }

        let item_width = body.iter().map(|(item, _)| item.width()).max().unwrap();
        let mut text = String::new();

        for (item, desc) in &body {
            let _ = writeln!(text, "{:width$}  {}", item, desc, width = item_width);
        }

        Self {
            title: title.to_string(),
            width: text.lines().map(|l| l.width()).max().unwrap() as u16,
            height: body.len() as u16,
            text,
        }
    }

    pub fn from_keymap(title: &str, body: Vec<(&str, BTreeSet<KeyEvent>)>) -> Self {
        let body = body
            .into_iter()
            .map(|(desc, events)| {
                let events = events.iter().map(ToString::to_string).collect::<Vec<_>>();
                (events.join(", "), desc.to_string())
            })
            .collect();

        Self::new(title, body)
    }

    pub fn from_registers(registers: &Registers) -> Self {
        let body = registers
            .inner()
            .iter()
            .map(|(ch, reg)| {
                let content = reg
                    .read()
                    .get(0)
                    .and_then(|s| s.lines().next())
                    .map(String::from)
                    .unwrap_or_default();
                (ch.to_string(), content)
            })
            .collect();

        let mut infobox = Self::new("Registers", body);
        infobox.width = 30; // copied content could be very long
        infobox
    }
}
