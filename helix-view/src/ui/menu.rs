use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
use fuzzy_matcher::FuzzyMatcher;

use crate::ui::prompt::PromptEvent as MenuEvent;
use crate::Editor;

#[derive(Debug, Clone)]
pub struct Cell {
    pub content: String,
}

impl From<String> for Cell {
    fn from(content: String) -> Self {
        Self { content }
    }
}

impl From<&'static str> for Cell {
    fn from(s: &'static str) -> Self {
        Self {
            content: s.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Row {
    pub cells: Vec<Cell>,
}

impl Row {
    pub fn new(cells: Vec<Cell>) -> Self {
        Self { cells }
    }
}

pub trait Item {
    fn sort_text(&self) -> &str;
    fn filter_text(&self) -> &str;

    fn label(&self) -> &str;
    fn row(&self) -> Row;
}

/// A menu component where each row has a menu item with multiple columns.
/// An example would be a completion menu, where each completion is a row and
/// each row has two columns, the completion string itself and the type of
/// the completion (class, variable, etc).
pub struct Menu<T: Item> {
    pub options: Vec<T>,

    pub cursor: Option<usize>,

    pub matcher: Box<Matcher>,
    /// (index, score)
    pub matches: Vec<(usize, i64)>,

    /// Max width of each column across all rows
    pub widths: Vec<u16>,

    pub callback_fn: Box<dyn Fn(&mut Editor, Option<&T>, MenuEvent)>,

    pub scroll: usize,
    pub size: (u16, u16),
}

impl<T: Item> Menu<T> {
    // TODO: it's like a slimmed down picker, share code? (picker = menu + prompt with different
    // rendering)
    pub fn new(
        options: Vec<T>,
        callback_fn: impl Fn(&mut Editor, Option<&T>, MenuEvent) + 'static,
    ) -> Self {
        let mut menu = Self {
            options,
            matcher: Box::new(Matcher::default()),
            matches: Vec::new(),
            cursor: None,
            widths: Vec::new(),
            callback_fn: Box::new(callback_fn),
            scroll: 0,
            size: (0, 0),
        };

        // TODO: scoring on empty input should just use a fastpath
        menu.score("");

        menu
    }

    pub fn score(&mut self, pattern: &str) {
        // need to borrow via pattern match otherwise it complains about simultaneous borrow
        let Self {
            ref mut matcher,
            ref mut matches,
            ref options,
            ..
        } = *self;

        // reuse the matches allocation
        matches.clear();
        matches.extend(options.iter().enumerate().filter_map(|(index, option)| {
            let text = option.filter_text();
            // TODO: using fuzzy_indices could give us the char idx for match highlighting
            matcher
                .fuzzy_match(text, pattern)
                .map(|score| (index, score))
        }));
        // matches.sort_unstable_by_key(|(_, score)| -score);
        matches.sort_unstable_by_key(|(index, _score)| options[*index].sort_text());

        // reset cursor position
        self.cursor = None;
        self.scroll = 0;
    }

    pub fn move_up(&mut self) {
        let len = self.matches.len();
        let pos = self.cursor.map_or(0, |i| (i + len.saturating_sub(1)) % len) % len;
        self.cursor = Some(pos);
        self.adjust_scroll();
    }

    pub fn move_down(&mut self) {
        let len = self.matches.len();
        let pos = self.cursor.map_or(0, |i| i + 1) % len;
        self.cursor = Some(pos);
        self.adjust_scroll();
    }

    pub fn adjust_scroll(&mut self) {
        let win_height = self.size.1 as usize;
        if let Some(cursor) = self.cursor {
            let mut scroll = self.scroll;
            if cursor > (win_height + scroll).saturating_sub(1) {
                // scroll down
                scroll += cursor - (win_height + scroll).saturating_sub(1)
            } else if cursor < scroll {
                // scroll up
                scroll = cursor
            }
            self.scroll = scroll;
        }
    }

    pub fn selection(&self) -> Option<&T> {
        self.cursor.and_then(|cursor| {
            self.matches
                .get(cursor)
                .map(|(index, _score)| &self.options[*index])
        })
    }

    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    pub fn len(&self) -> usize {
        self.matches.len()
    }
}
