use crate::View;

use std::path::PathBuf;

use anyhow::Error;

pub struct Editor {
    pub views: Vec<View>,
    pub focus: usize,
    pub should_close: bool,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            views: Vec::new(),
            focus: 0,
            should_close: false,
        }
    }

    pub fn open(&mut self, path: PathBuf, size: (u16, u16)) -> Result<(), Error> {
        let pos = self.views.len();
        self.views.push(View::open(path, size)?);
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
