use helix_core::str_utils::char_to_byte_idx;
use helix_view::editor::{WhitespacePalette, WhitespaceRender, WhitespaceRenderValue};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum WhitespaceKind {
    None,
    Space,
    NonBreakingSpace,
    NarrowNonBreakingSpace,
    Tab,
    Newline,
}

impl WhitespaceKind {
    pub fn to_str(self, palette: &WhitespacePalette) -> &str {
        match self {
            WhitespaceKind::Space => &palette.space,
            WhitespaceKind::NonBreakingSpace => &palette.nbsp,
            WhitespaceKind::NarrowNonBreakingSpace => &palette.nnbsp,
            WhitespaceKind::Tab => {
                let grapheme_tab_width = char_to_byte_idx(&palette.tab, palette.tab.len());
                &palette.tab[..grapheme_tab_width]
            }
            WhitespaceKind::Newline | WhitespaceKind::None => "",
        }
    }
}

#[derive(Debug)]
pub struct TrailingWhitespaceTracker {
    enabled: bool,
    palette: WhitespacePalette,
    tracking_from: usize,
    tracking_content: Vec<(WhitespaceKind, usize)>,
}

impl TrailingWhitespaceTracker {
    pub fn new(render: WhitespaceRender, palette: WhitespacePalette) -> Self {
        Self {
            palette,
            enabled: render.any(WhitespaceRenderValue::Trailing),
            tracking_from: 0,
            tracking_content: vec![],
        }
    }

    // Tracks the whitespace and returns wether [`render`] should be called right after
    // to display the trailing whitespace.
    pub fn track(&mut self, from: usize, kind: WhitespaceKind) -> bool {
        if !self.enabled || kind == WhitespaceKind::None {
            self.tracking_content.clear();
            return false;
        }
        if kind == WhitespaceKind::Newline {
            return true;
        }
        if self.tracking_content.is_empty() {
            self.tracking_from = from;
        }
        self.compress(kind);
        false
    }

    pub fn render(&mut self, callback: &mut impl FnMut(&str, usize)) {
        if self.tracking_content.is_empty() {
            return;
        }
        let mut offset = self.tracking_from;
        self.tracking_content.iter().for_each(|(kind, n)| {
            let ws = kind.to_str(&self.palette).repeat(*n);
            callback(&ws, offset);
            offset += n;
        });
        self.tracking_content.clear();
    }

    fn compress(&mut self, kind: WhitespaceKind) {
        if let Some((last_kind, n)) = self.tracking_content.last_mut() {
            if *last_kind == kind {
                *n += 1;
                return;
            }
        }
        self.tracking_content.push((kind, 1));
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
            nnbsp: "M".into(),
            tab: "<TAB>".into(),
            virtual_tab: "V".into(),
            newline: "L".into(),
        }
    }

    fn capture(sut: &mut TrailingWhitespaceTracker) -> (String, usize, usize) {
        let mut captured_content = String::new();
        let mut from: usize = 0;
        let mut to: usize = 0;

        sut.render(&mut |content: &str, pos: usize| {
            captured_content.push_str(content);
            if from == 0 {
                from = pos;
            }
            to = pos;
        });

        (captured_content, from, to)
    }

    #[test]
    fn test_trailing_whitespace_tracker_correctly_tracks_sequences() {
        let ws_render = WhitespaceRender::Basic(WhitespaceRenderValue::Trailing);

        let mut sut = TrailingWhitespaceTracker::new(ws_render, palette());

        sut.track(5, WhitespaceKind::Space);
        sut.track(6, WhitespaceKind::NonBreakingSpace);
        sut.track(7, WhitespaceKind::NarrowNonBreakingSpace);
        sut.track(8, WhitespaceKind::Tab);

        let (content, from, to) = capture(&mut sut);

        assert_eq!(5, from);
        assert_eq!(8, to);
        assert_eq!("SNM<TAB>", content);

        // Now we break the sequence
        sut.track(6, WhitespaceKind::None);

        let (content, from, to) = capture(&mut sut);
        assert_eq!(0, from);
        assert_eq!(0, to);
        assert_eq!("", content);

        sut.track(10, WhitespaceKind::Tab);
        sut.track(11, WhitespaceKind::NonBreakingSpace);
        sut.track(12, WhitespaceKind::NarrowNonBreakingSpace);
        sut.track(13, WhitespaceKind::Space);

        let (content, from, to) = capture(&mut sut);
        assert_eq!(10, from);
        assert_eq!(13, to);
        assert_eq!("<TAB>NMS", content);

        // Verify compression works
        sut.track(20, WhitespaceKind::Space);
        sut.track(21, WhitespaceKind::Space);
        sut.track(22, WhitespaceKind::NonBreakingSpace);
        sut.track(23, WhitespaceKind::NonBreakingSpace);
        sut.track(24, WhitespaceKind::NarrowNonBreakingSpace);
        sut.track(25, WhitespaceKind::NarrowNonBreakingSpace);
        sut.track(26, WhitespaceKind::Tab);
        sut.track(27, WhitespaceKind::Tab);
        sut.track(28, WhitespaceKind::Tab);

        let (content, from, to) = capture(&mut sut);
        assert_eq!(20, from);
        assert_eq!(26, to); // Compression means last tracked token is on 26 instead of 28
        assert_eq!("SSNNMM<TAB><TAB><TAB>", content);
    }
}
