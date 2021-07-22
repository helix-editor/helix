use crate::input::KeyEvent;
use helix_core::unicode::width::UnicodeWidthStr;
use std::{collections::BTreeMap, fmt::Write};

#[derive(Debug)]
/// Info box used in editor. Rendering logic will be in other crate.
pub struct Info {
    /// Title kept as static str for now.
    pub title: String,
    /// Text body, should contains newline.
    pub text: String,
    /// Body width.
    pub width: u16,
    /// Body height.
    pub height: u16,
}

impl Info {
    // body is a BTreeMap instead of a HashMap because keymaps are represented
    // with nested hashmaps with no ordering, and each invocation of infobox would
    // show different orders of items
    pub fn key(title: &str, body: BTreeMap<&str, Vec<KeyEvent>>) -> Info {
        let (lpad, mpad, rpad) = (1, 2, 1);
        let keymaps_width: u16 = body
            .values()
            .map(|r| r.iter().map(|e| e.width() as u16 + 2).sum::<u16>() - 2)
            .max()
            .unwrap();
        let mut text = String::new();
        let mut width = 0;
        let height = body.len() as u16;
        for (desc, keyevents) in body {
            let keyevent = keyevents[0];
            let mut left = keymaps_width - keyevent.width() as u16;
            for _ in 0..lpad {
                text.push(' ');
            }
            write!(text, "{}", keyevent).ok();
            for keyevent in &keyevents[1..] {
                write!(text, ", {}", keyevent).ok();
                left -= 2 + keyevent.width() as u16;
            }
            for _ in 0..left + mpad {
                text.push(' ');
            }
            let desc = desc.trim();
            let w = lpad + keymaps_width + mpad + (desc.width() as u16) + rpad;
            if w > width {
                width = w;
            }
            writeln!(text, "{}", desc).ok();
        }
        Info {
            title: title.to_string(),
            text,
            width,
            height,
        }
    }
}
