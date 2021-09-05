use crate::input::KeyEvent;
use helix_core::unicode::width::UnicodeWidthStr;
use std::fmt::Write;

#[derive(Debug)]
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
    pub fn new(title: &str, body: Vec<(&str, Vec<KeyEvent>)>) -> Info {
        let body = body
            .into_iter()
            .map(|(desc, events)| {
                let events = events.iter().map(ToString::to_string).collect::<Vec<_>>();
                (desc, events.join(", "))
            })
            .collect::<Vec<_>>();

        let keymaps_width = body.iter().map(|r| r.1.len()).max().unwrap();
        let mut text = String::new();

        for (desc, keyevents) in &body {
            let _ = writeln!(
                text,
                "{:width$}  {}",
                keyevents,
                desc,
                width = keymaps_width
            );
        }

        Info {
            title: title.to_string(),
            width: text.lines().map(|l| l.width()).max().unwrap() as u16,
            height: body.len() as u16,
            text,
        }
    }
}
