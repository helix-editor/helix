use crate::theme::Theme;
use crate::View;
use helix_core::State;

use std::path::PathBuf;

use anyhow::Error;

pub struct Editor {
    pub views: Vec<View>,
    pub focus: usize,
    pub should_close: bool,
    pub theme: Theme, // TODO: share one instance
}

impl Editor {
    pub fn new() -> Self {
        let theme = Theme::default();

        Self {
            views: Vec::new(),
            focus: 0,
            should_close: false,
            theme,
        }
    }

    pub fn open(&mut self, path: PathBuf, size: (u16, u16)) -> Result<(), Error> {
        let pos = self.views.len();
        let state = State::load(path, self.theme.scopes())?;
        self.views.push(View::new(state, size)?);
        self.focus = pos;
        Ok(())
    }

    pub fn view(&self) -> Option<&View> {
        self.views.get(self.focus)
    }

    pub fn view_mut(&mut self) -> Option<&mut View> {
        self.views.get_mut(self.focus)
    }
}
