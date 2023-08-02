use std::{io::Read, sync::Arc};

use anyhow::bail;
use arc_swap::ArcSwap;

use crate::DiffProvider;

pub struct File;

const MAX_SIZE: usize = 10 * 1024 * 1024;
const READ_SIZE: usize = 8 * 1024;

impl DiffProvider for File {
    fn get_diff_base(&self, file: &std::path::Path) -> anyhow::Result<Vec<u8>> {
        let mut fh = std::fs::File::open(file)?;
        let mut contents = vec![];
        loop {
            let mut chunk = [0; READ_SIZE];
            let bytes = fh.read(&mut chunk)?;
            if bytes == 0 {
                break;
            }
            if contents.len() + bytes > MAX_SIZE {
                bail!("file too long");
            }
            contents.extend_from_slice(&chunk[..bytes]);
        }
        Ok(contents)
    }

    fn get_current_head_name(
        &self,
        _file: &std::path::Path,
    ) -> anyhow::Result<std::sync::Arc<arc_swap::ArcSwap<Box<str>>>> {
        Ok(Arc::new(ArcSwap::from_pointee(
            "(file)".to_string().into_boxed_str(),
        )))
    }
}
