use crate::register::Registers;
use helix_core::graphemes::str_width;
use std::{borrow::Cow, fmt::Write};

#[derive(Debug)]
/// Info box used in editor. Rendering logic will be in other crate.
pub struct Info {
    /// Title shown at top.
    pub title: Cow<'static, str>,
    /// Text body, should contain newlines.
    pub text: String,
    /// Body width.
    pub width: u16,
    /// Body height.
    pub height: u16,
}

impl Info {
    pub fn new<T, K, V>(title: T, body: &[(K, V)]) -> Self
    where
        T: Into<Cow<'static, str>>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let title = title.into();
        if body.is_empty() {
            return Self {
                height: 1,
                width: str_width(title.as_ref()) as u16,
                text: "".to_string(),
                title,
            };
        }

        let item_width = body
            .iter()
            .map(|(item, _)| str_width(item.as_ref()))
            .max()
            .unwrap();
        let mut text = String::new();

        for (item, desc) in body {
            let item = item.as_ref();
            let _ = text.write_str(item);
            let padding = item_width.saturating_sub(str_width(item)) + 2;
            let _ = text.write_str(&" ".repeat(padding));
            let _ = writeln!(text, "{}", desc.as_ref());
        }

        Self {
            title,
            width: text.lines().map(str_width).max().unwrap() as u16,
            height: body.len() as u16,
            text,
        }
    }

    pub fn from_registers(title: impl Into<Cow<'static, str>>, registers: &Registers) -> Self {
        let body: Vec<_> = registers
            .iter_preview()
            .map(|(ch, preview)| (ch.to_string(), preview))
            .collect();

        let mut infobox = Self::new(title, &body);
        infobox.width = 30; // copied content could be very long
        infobox
    }
}
