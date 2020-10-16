use crate::View;

use std::path::PathBuf;

use anyhow::Error;

pub struct Editor {
    pub view: Option<View>,
    pub should_close: bool,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            view: None,
            should_close: false,
        }
    }

    pub fn open(&mut self, path: PathBuf, size: (u16, u16)) -> Result<(), Error> {
        self.view = Some(View::open(path, size)?);
        Ok(())
    }
}
