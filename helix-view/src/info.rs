use crate::register::Registers;
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
    pub fn new<T, U>(title: &str, body: &[(T, U)]) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        if body.is_empty() {
            return Self {
                title: title.to_string(),
                height: 1,
                width: title.len() as u16,
                text: "".to_string(),
            };
        }

        let item_width = body
            .iter()
            .map(|(item, _)| item.as_ref().width())
            .max()
            .unwrap();
        let mut text = String::new();

        let mut height = 0;

        for (item, desc) in body {
            let mut line_iter = desc.as_ref().lines();

            if let Some(first_line) = line_iter.next() {
                let _ = writeln!(
                    text,
                    "{:width$}  {}",
                    item.as_ref(),
                    first_line,
                    width = item_width
                );
                height += 1;
            }

            for line in line_iter {
                let _ = writeln!(text, "{:width$}  {}", "", line, width = item_width);
                height += 1;
            }
        }

        Self {
            title: title.to_string(),
            width: text.lines().map(|l| l.width()).max().unwrap_or(body.len()) as u16,
            height,
            text,
        }
    }

    pub fn from_registers(registers: &Registers) -> Self {
        let body: Vec<_> = registers
            .iter_preview()
            .map(|(ch, preview)| (ch.to_string(), preview))
            .collect();

        let mut infobox = Self::new("Registers", &body);
        infobox.width = 30; // copied content could be very long
        infobox
    }
}
