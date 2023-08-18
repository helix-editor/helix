use crate::register::Registers;
use helix_core::unicode::width::UnicodeWidthStr;

use crate::graphics::Rect;

#[derive(Debug)]
pub enum Location {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
    Center,
}

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
    /// Info box location
    pub location: Location,
}

impl Info {
    pub fn new<T>(title: &str, body: T, location: Location) -> Self
    where
        T: AsRef<str>,
    {
        Self::from_lines(
            title,
            &body.as_ref().split('\n').collect::<Vec<_>>(),
            location,
        )
    }

    pub fn from_lines<T>(title: &str, lines: &[T], location: Location) -> Self
    where
        T: AsRef<str>,
    {
        if lines.is_empty() {
            return Self {
                title: title.to_string(),
                text: "".to_string(),
                width: title.len() as u16,
                height: 1,
                location,
            };
        }

        let lines = lines.iter().map(|l| l.as_ref()).collect::<Vec<_>>();
        let width = lines.iter().map(|l| l.len()).max().unwrap_or(title.len());

        Info {
            title: title.to_string(),
            text: lines.join("\n"),
            width: width as u16,
            height: lines.len() as u16,
            location,
        }
    }

    pub fn from_kv_pairs<T, U>(title: &str, kv_pairs: &[(T, U)], location: Location) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        let item_width = kv_pairs
            .iter()
            .map(|(item, _)| item.as_ref().width())
            .max()
            .unwrap();

        let x = kv_pairs
            .iter()
            .map(|(item, desc)| {
                format!(
                    "{:width$}  {}",
                    item.as_ref(),
                    desc.as_ref(),
                    width = item_width
                )
            })
            .collect::<Vec<_>>();

        Self::from_lines(title, &x, location)
    }

    pub fn from_registers(registers: &Registers) -> Self {
        let body: Vec<_> = registers
            .iter_preview()
            .map(|(ch, preview)| (ch.to_string(), preview))
            .collect();

        let mut infobox = Self::from_kv_pairs("Registers", &body, Location::BottomRight);
        infobox.width = 30; // copied content could be very long
        infobox
    }

    pub fn get_intersecting_area(&self, viewport: Rect) -> Rect {
        let width = self.width + 2 + 2; // +2 for border, +2 for margin
        let height = self.height + 2; // +2 for border

        let (x, y) = match self.location {
            // Top variants don't take into consideration bufferline height
            Location::TopRight => (viewport.width.saturating_sub(width), 0),
            Location::TopLeft => (0, 0),
            Location::BottomRight => (
                viewport.width.saturating_sub(width),
                viewport.height.saturating_sub(height + 2), // +2 for statusline
            ),
            Location::BottomLeft => (
                0,
                viewport.height.saturating_sub(height + 2), // +2 for statusline
            ),
            Location::Center => (
                viewport
                    .width
                    .saturating_div(2)
                    .saturating_sub(width.saturating_div(2)),
                viewport
                    .height
                    .saturating_div(2)
                    .saturating_sub(height.saturating_div(2)),
            ),
        };

        viewport.intersection(Rect::new(x, y, width, height))
    }
}
