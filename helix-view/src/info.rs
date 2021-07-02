use crate::input::KeyEvent;
use helix_core::unicode::width::UnicodeWidthStr;
use std::fmt::Write;

#[derive(Debug)]
/// Info box used in editor. Rendering logic will be in other crate.
pub struct Info {
    /// Title kept as static str for now.
    pub title: &'static str,
    /// Text body, should contains newline.
    pub text: String,
    /// Body width.
    pub width: u16,
    /// Body height.
    pub height: u16,
}

impl Info {
    pub fn key(title: &'static str, body: Vec<(&[KeyEvent], &'static str)>) -> Info {
        let keymaps_width: u16 = body
            .iter()
            .map(|r| r.0.iter().map(|e| e.width() as u16 + 2).sum::<u16>() - 2)
            .max()
            .unwrap();
        let mut text = String::new();
        let mut width = 0;
        let height = body.len() as u16;
        for (keyevents, desc) in body {
            let keyevent = keyevents[0];
            let mut left = keymaps_width - keyevent.width() as u16;
            write!(text, "{}", keyevent).ok();
            for keyevent in &keyevents[1..] {
                write!(text, ", {}", keyevent).ok();
                left -= 2 + keyevent.width() as u16;
            }
            for _ in 0..left {
                text.push(' ');
            }
            if keymaps_width + 2 + (desc.width() as u16) > width {
                width = keymaps_width + 2 + desc.width() as u16;
            }
            writeln!(text, "  {}", &desc).ok();
        }
        Info {
            title,
            text,
            width,
            height,
        }
    }
}
