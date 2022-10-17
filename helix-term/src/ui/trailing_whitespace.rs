use helix_view::editor::{WhitespaceConfig, WhitespaceRenderValue};

pub enum WhitespaceKind {
    Space,
    NonBreakingSpace,
    Tab(usize, usize),
}

pub struct TrailingWhitespaceTracker {
    enabled: bool,
    is_space_enable: bool,
    nbsp_enabled: bool,
    tab_enabled: bool,

    is_tracking: bool,
    tracking_from: u16,
    tracked: Vec<WhitespaceKind>,
}

impl TrailingWhitespaceTracker {
    pub fn new(cfg: &WhitespaceConfig) -> Self {
        let space_enabled = cfg.render.space() == WhitespaceRenderValue::Trailing;
        let nbsp_enabled = cfg.render.nbsp() == WhitespaceRenderValue::Trailing;
        let tab_enabled = cfg.render.tab() == WhitespaceRenderValue::Trailing;
        let enabled = space_enabled || nbsp_enabled || tab_enabled;
        Self {
            enabled,
            is_space_enable: space_enabled,
            nbsp_enabled,
            tab_enabled,

            is_tracking: false,
            tracking_from: 0,
            tracked: vec![],
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn track_nonwhitespace(&mut self) {
        self.is_tracking = false;
    }

    pub fn track_whitespace(&mut self, from: u16, kind: WhitespaceKind) {
        if !self.is_tracking {
            self.is_tracking = true;
            self.tracking_from = from;
            self.tracked.clear();
        }
        self.tracked.push(kind);
    }

    pub fn get_trailing_whitespace(
        &mut self,
        space_char: &str,
        space: &str,
        nbsp_char: &str,
        nbsp: &str,
        tab_char: &str,
        tab: &str,
    ) -> Option<(u16, String)> {
        if !self.enabled || !self.is_tracking {
            return None;
        }

        self.is_tracking = false;
        let trailing_space = if self.is_space_enable {
            space_char
        } else {
            space
        };
        let trailing_nbsp = if self.nbsp_enabled { nbsp_char } else { nbsp };
        let trailing_whitespace = self
            .tracked
            .iter()
            .map(|kind| match kind {
                WhitespaceKind::Space => trailing_space,
                WhitespaceKind::NonBreakingSpace => trailing_nbsp,
                WhitespaceKind::Tab(original_width, trailing_width) => {
                    if self.tab_enabled {
                        &tab_char[..*trailing_width]
                    } else {
                        &tab[..*original_width]
                    }
                }
            })
            .collect::<Vec<&str>>()
            .join("");

        Some((self.tracking_from, trailing_whitespace))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_view::editor::{WhitespaceConfig, WhitespaceRender};

    #[test]
    fn test_default_trailing_whitespace_tracker() {
        let cfg = &WhitespaceConfig::default();
        let sut = TrailingWhitespaceTracker::new(cfg);

        assert!(
            !sut.is_enabled(),
            "trailing whitespace is not enabled by default"
        );
    }

    #[test]
    fn test_trailing_whitespace_tracker_basic_config_enables() {
        let mut cfg = &mut WhitespaceConfig::default();
        cfg.render = WhitespaceRender::Basic(WhitespaceRenderValue::Trailing);
        let sut = TrailingWhitespaceTracker::new(cfg);

        assert!(
            sut.is_enabled(),
            "basic config set to trailing should enable the tracker"
        );
    }

    #[test]
    fn test_trailing_whitespace_tracker_specific_configs_enables() {
        let mut cfg = &mut WhitespaceConfig::default();
        cfg.render = WhitespaceRender::Specific {
            default: Some(WhitespaceRenderValue::None),
            space: Some(WhitespaceRenderValue::Trailing),
            nbsp: None,
            tab: None,
            newline: None,
        };

        let sut = TrailingWhitespaceTracker::new(cfg);
        assert!(
            sut.is_enabled(),
            "should be enabled when space trailing is enabled"
        );

        cfg.render = WhitespaceRender::Specific {
            default: Some(WhitespaceRenderValue::None),
            space: None,
            nbsp: Some(WhitespaceRenderValue::Trailing),
            tab: None,
            newline: None,
        };
        let sut = TrailingWhitespaceTracker::new(cfg);
        assert!(
            sut.is_enabled(),
            "should be enabled when nbsp trailing is enabled"
        );

        cfg.render = WhitespaceRender::Specific {
            default: Some(WhitespaceRenderValue::None),
            space: None,
            nbsp: None,
            tab: Some(WhitespaceRenderValue::Trailing),
            newline: None,
        };
        let sut = TrailingWhitespaceTracker::new(cfg);
        assert!(
            sut.is_enabled(),
            "hould be enabled when tab trailing is enabled"
        );
    }

    #[test]
    fn test_trailing_whitespace_tracker_correctly_tracks_sequences() {
        let mut cfg = &mut WhitespaceConfig::default();
        cfg.render = WhitespaceRender::Basic(WhitespaceRenderValue::Trailing);
        let mut sut = TrailingWhitespaceTracker::new(cfg);

        sut.track_whitespace(5, WhitespaceKind::Space);
        sut.track_whitespace(6, WhitespaceKind::NonBreakingSpace);
        sut.track_whitespace(7, WhitespaceKind::Tab(1, 1));

        let trailing = sut.get_trailing_whitespace("S", " ", "N", " ", "T", " ");
        assert!(trailing.is_some());
        let (from, display) = trailing.unwrap();
        assert_eq!(5, from);
        assert_eq!("SNT", display);

        // Now we break the sequence
        sut.track_nonwhitespace();
        let trailing = sut.get_trailing_whitespace("S", " ", "N", " ", "T", " ");
        assert!(trailing.is_none());

        // Now we track again
        sut.track_whitespace(10, WhitespaceKind::Tab(1, 1));
        sut.track_whitespace(11, WhitespaceKind::NonBreakingSpace);
        sut.track_whitespace(12, WhitespaceKind::Space);

        let trailing = sut.get_trailing_whitespace("S", " ", "N", " ", "T", " ");
        assert!(trailing.is_some());
        let (from, display) = trailing.unwrap();
        assert_eq!(10, from);
        assert_eq!("TNS", display);
    }
}
