use helix_view::editor::{WhitespacePalette, WhitespaceRender, WhitespaceRenderValue};

use helix_core::str_utils::char_to_byte_idx;

#[derive(Debug, Eq, PartialEq)]
pub enum WhitespaceKind {
    None,
    Space,
    NonBreakingSpace,
    Tab(usize),
    Newline,
}

#[derive(Debug)]
pub struct TrailingWhitespaceTracker {
    enabled: bool,
    palette: WhitespacePalette,
    tracking: bool,
    tracking_from: usize,
    tracking_content: Vec<WhitespaceKind>,
}

impl TrailingWhitespaceTracker {
    pub fn new(render: &WhitespaceRender, palette: WhitespacePalette) -> Self {
        Self {
            palette,
            enabled: render.any(WhitespaceRenderValue::Trailing),
            tracking: false,
            tracking_from: 0,
            tracking_content: vec![],
        }
    }

    pub fn track(&mut self, from: usize, kind: WhitespaceKind) {
        if kind == WhitespaceKind::None {
            self.tracking = false;
            return;
        }
        if !self.tracking {
            self.tracking = true;
            self.tracking_from = from;
            self.tracking_content.clear();
        }
        self.tracking_content.push(kind);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[must_use]
    pub fn get(&mut self) -> Option<(usize, String)> {
        if !self.enabled || !self.tracking {
            return None;
        }

        self.tracking = false;
        let trailing_whitespace = self
            .tracking_content
            .iter()
            .map(|kind| match kind {
                WhitespaceKind::Space => &self.palette.space,
                WhitespaceKind::NonBreakingSpace => &self.palette.nbsp,
                WhitespaceKind::Tab(width) => {
                    let grapheme_tab_width = char_to_byte_idx(&self.palette.tab, *width);
                    &self.palette.tab[..grapheme_tab_width]
                }
                WhitespaceKind::Newline => &self.palette.newline,
                WhitespaceKind::None => "",
            })
            .collect::<Vec<&str>>()
            .join("");

        Some((self.tracking_from, trailing_whitespace))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use helix_view::editor::WhitespaceRender;

    fn palette() -> WhitespacePalette {
        WhitespacePalette {
            space: "S".into(),
            nbsp: "N".into(),
            tab: "T".into(),
            virtual_tab: "V".into(),
            newline: "L".into(),
        }
    }

    #[test]
    fn test_trailing_whitespace_tracker_correctly_tracks_sequences() {
        let ws_render = WhitespaceRender::Basic(WhitespaceRenderValue::Trailing);

        let mut sut = TrailingWhitespaceTracker::new(&ws_render, palette());

        sut.track(5, WhitespaceKind::Space);
        sut.track(6, WhitespaceKind::NonBreakingSpace);
        sut.track(7, WhitespaceKind::Tab(1));
        sut.track(8, WhitespaceKind::Newline);

        let trailing = sut.get();
        assert!(trailing.is_some());
        let (from, display) = trailing.unwrap();
        assert_eq!(5, from);
        assert_eq!("SNTL", display);

        // Now we break the sequence
        sut.track(6, WhitespaceKind::None);
        let trailing = sut.get();
        assert!(trailing.is_none());

        // Now we track again
        sut.track(10, WhitespaceKind::Tab(1));
        sut.track(11, WhitespaceKind::NonBreakingSpace);
        sut.track(12, WhitespaceKind::Space);
        sut.track(13, WhitespaceKind::Newline);

        let trailing = sut.get();
        assert!(trailing.is_some());
        let (from, display) = trailing.unwrap();
        assert_eq!(10, from);
        assert_eq!("TNSL", display);
    }
}
