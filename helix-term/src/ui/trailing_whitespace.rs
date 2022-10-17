use helix_view::editor::{WhitespaceRender, WhitespaceRenderValue};

#[derive(PartialEq)]
pub enum WhitespaceKind {
    None,
    Space,
    NonBreakingSpace,
    Tab(usize, usize),
}

pub struct TrailingWhitespaceTracker<'a> {
    enabled: bool,

    tracking: bool,
    tracking_from: usize,
    tracking_content: Vec<WhitespaceKind>,

    space: &'a str,
    nbsp: &'a str,
    tab: &'a str,
    tab_enabled: bool,
}

impl<'a> TrailingWhitespaceTracker<'a> {
    pub fn new(
        cfg: &WhitespaceRender,
        space: &'a str,
        nbsp: &'a str,
        tab: &'a str,
        tab_empty: &'a str,
    ) -> Self {
        let space_enabled = matches!(
            cfg.space(),
            WhitespaceRenderValue::Trailing | WhitespaceRenderValue::All
        );
        let nbsp_enabled = matches!(
            cfg.nbsp(),
            WhitespaceRenderValue::Trailing | WhitespaceRenderValue::All
        );
        let tab_enabled = matches!(
            cfg.tab(),
            WhitespaceRenderValue::Trailing | WhitespaceRenderValue::All
        );

        let enabled = cfg.space() == WhitespaceRenderValue::Trailing
            || cfg.nbsp() == WhitespaceRenderValue::Trailing
            || cfg.tab() == WhitespaceRenderValue::Trailing;

        Self {
            enabled,

            tracking: false,
            tracking_from: 0,
            tracking_content: vec![],

            space: if space_enabled { space } else { " " },
            nbsp: if nbsp_enabled { nbsp } else { " " },
            tab: if tab_enabled { tab } else { tab_empty },
            tab_enabled,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn track(&mut self, from: usize, kind: WhitespaceKind) {
        if kind == WhitespaceKind::None {
            self.reset();
            return;
        }
        if !self.tracking {
            self.tracking = true;
            self.tracking_from = from;
            self.tracking_content.clear();
        }
        self.tracking_content.push(kind);
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
                WhitespaceKind::Space => self.space,
                WhitespaceKind::NonBreakingSpace => self.nbsp,
                WhitespaceKind::Tab(original_width, trailing_width) => {
                    &self.tab[..*(if self.tab_enabled {
                        trailing_width
                    } else {
                        original_width
                    })]
                }
                WhitespaceKind::None => "",
            })
            .collect::<Vec<&str>>()
            .join("");

        Some((self.tracking_from, trailing_whitespace))
    }

    fn reset(&mut self) {
        self.tracking = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_view::editor::{WhitespaceConfig, WhitespaceRender};

    #[test]
    fn test_default_trailing_whitespace_tracker() {
        let cfg = &WhitespaceConfig::default();
        let sut = TrailingWhitespaceTracker::new(&cfg.render, " ", " ", " ", " ");

        assert!(
            !sut.enabled(),
            "trailing whitespace is not enabled by default"
        );
    }

    #[test]
    fn test_trailing_whitespace_tracker_basic_config_enables() {
        let mut cfg = &mut WhitespaceConfig::default();
        cfg.render = WhitespaceRender::Basic(WhitespaceRenderValue::Trailing);
        let sut = TrailingWhitespaceTracker::new(&cfg.render, " ", " ", " ", " ");

        assert!(
            sut.enabled(),
            "basic config set to trailing should enable the tracker"
        );
    }

    #[test]
    fn test_trailing_whitespace_tracker_specific_configs_enables() {
        let cfg = &WhitespaceRender::Specific {
            default: Some(WhitespaceRenderValue::None),
            space: Some(WhitespaceRenderValue::Trailing),
            nbsp: None,
            tab: None,
            newline: None,
        };
        let sut = TrailingWhitespaceTracker::new(cfg, " ", " ", " ", " ");
        assert!(
            sut.enabled(),
            "should be enabled when space trailing is enabled"
        );

        let cfg = &WhitespaceRender::Specific {
            default: Some(WhitespaceRenderValue::None),
            space: None,
            nbsp: Some(WhitespaceRenderValue::Trailing),
            tab: None,
            newline: None,
        };
        let sut = TrailingWhitespaceTracker::new(cfg, " ", " ", " ", " ");
        assert!(
            sut.enabled(),
            "should be enabled when nbsp trailing is enabled"
        );

        let cfg = &WhitespaceRender::Specific {
            default: Some(WhitespaceRenderValue::None),
            space: None,
            nbsp: None,
            tab: Some(WhitespaceRenderValue::Trailing),
            newline: None,
        };
        let sut = TrailingWhitespaceTracker::new(cfg, " ", " ", " ", " ");
        assert!(
            sut.enabled(),
            "hould be enabled when tab trailing is enabled"
        );
    }

    #[test]
    fn test_trailing_whitespace_tracker_correctly_tracks_sequences() {
        let cfg = &WhitespaceRender::Basic(WhitespaceRenderValue::Trailing);

        let mut sut = TrailingWhitespaceTracker::new(cfg, "S", "N", "T", "E");

        sut.track(5, WhitespaceKind::Space);
        sut.track(6, WhitespaceKind::NonBreakingSpace);
        sut.track(7, WhitespaceKind::Tab(1, 1));

        let trailing = sut.get();
        assert!(trailing.is_some());
        let (from, display) = trailing.unwrap();
        assert_eq!(5, from);
        assert_eq!("SNT", display);

        // Now we break the sequence
        sut.track(6, WhitespaceKind::None);
        let trailing = sut.get();
        assert!(trailing.is_none());

        // Now we track again
        sut.track(10, WhitespaceKind::Tab(1, 1));
        sut.track(11, WhitespaceKind::NonBreakingSpace);
        sut.track(12, WhitespaceKind::Space);

        let trailing = sut.get();
        assert!(trailing.is_some());
        let (from, display) = trailing.unwrap();
        assert_eq!(10, from);
        assert_eq!("TNS", display);
    }

    #[test]
    fn test_trailing_whitespace_tracker_correctly_fallsback_empty_characters() {
        let cfg = &WhitespaceRender::Specific {
            default: Some(WhitespaceRenderValue::None),
            space: None,
            nbsp: None,
            tab: None,
            newline: None,
        };

        let mut sut = TrailingWhitespaceTracker::new(cfg, "S", "N", "T", "E");
        sut.enabled = true; // forcefully enable the tracker

        sut.track(5, WhitespaceKind::Space);
        sut.track(6, WhitespaceKind::NonBreakingSpace);
        sut.track(7, WhitespaceKind::Tab(1, 1));

        let trailing = sut.get();
        assert!(trailing.is_some());
        let (from, display) = trailing.unwrap();
        assert_eq!(5, from);
        assert_eq!("  E", display);
    }
}
