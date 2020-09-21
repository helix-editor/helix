use anyhow::Error;

use std::path::PathBuf;

use crate::theme::Theme;
use helix_core::State;

pub struct View {
    pub state: State,
    pub first_line: u16,
    pub size: (u16, u16),
    pub theme: Theme, // TODO: share one instance
}

impl View {
    pub fn open(path: PathBuf, size: (u16, u16)) -> Result<View, Error> {
        let mut state = State::load(path)?;
        let theme = Theme::default();
        state.syntax.as_mut().unwrap().configure(theme.scopes());

        let view = View {
            state,
            first_line: 0,
            size, // TODO: pass in from term
            theme,
        };

        Ok(view)
    }

    pub fn ensure_cursor_in_view(&mut self) {
        let cursor = self.state.selection().cursor();
        let line = self.state.doc().char_to_line(cursor) as u16;
        let document_end = self.first_line + self.size.1.saturating_sub(1) - 1;

        let padding = 5u16;

        // TODO: side scroll

        if line > document_end.saturating_sub(padding) {
            // scroll down
            self.first_line += line - (document_end.saturating_sub(padding));
        } else if line < self.first_line + padding {
            // scroll up
            self.first_line = line.saturating_sub(padding);
        }
    }
}
