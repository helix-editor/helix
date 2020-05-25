use anyhow::Error;
use ropey::Rope;
use std::{env, fs::File, io::BufReader, path::PathBuf};

pub struct Buffer {
    pub contents: Rope,
}

impl Buffer {
    pub fn load(path: PathBuf) -> Result<Self, Error> {
        let current_dir = env::current_dir()?;

        let contents = Rope::from_reader(BufReader::new(File::open(path)?))?;

        // TODO: create if not found
        Ok(Buffer { contents })
    }
}
