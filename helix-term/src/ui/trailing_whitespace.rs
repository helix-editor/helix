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
