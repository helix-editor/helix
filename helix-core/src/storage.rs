use std::{
    io::{ErrorKind, Read, Result, Write},
    path::{Path, PathBuf},
};

use log::debug;
use web_sys::Storage;

pub struct WebStorage {
    id: String,
    content: Vec<u8>,
    read_pos: usize,
}

impl Read for WebStorage {
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        let remaining = self.content.len() - self.read_pos;
        let capacity = buf.len();
        if capacity >= remaining {
            let r = buf.write(&self.content[self.read_pos..]);
            self.read_pos += remaining;
            r
        } else {
            let r = buf.write(&self.content[..capacity]);
            self.read_pos += capacity;
            r
        }
    }
}

impl Write for WebStorage {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let storage = storage()?;
        if let Ok(content) = String::from_utf8(buf.to_vec()) {
            match storage.set_item(&self.id, &content) {
                Ok(_) => Ok(buf.len()),
                Err(e) => {
                    debug!("error writing to storage {:?}", e);
                    Err(ErrorKind::Other.into())
                }
            }
        } else {
            debug!("storage only supports writing UTF-8 content");
            Err(ErrorKind::InvalidData.into())
        }
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

pub fn exists<P: AsRef<Path>>(path: P) -> bool {
    fn inner(path: &Path) -> bool {
        if let Ok(storage) = storage() {
            let path = path.to_string_lossy();
            match storage.get_item(&path) {
                Ok(Some(_)) => true,
                _ => false,
            }
        } else {
            false
        }
    }
    inner(path.as_ref())
}

pub fn open<P: AsRef<Path>>(path: P) -> Result<WebStorage> {
    fn inner(path: &Path) -> Result<WebStorage> {
        let storage = storage()?;
        let path = path.to_string_lossy();
        match storage.get_item(&path) {
            Ok(Some(content)) => Ok(WebStorage {
                id: path.into_owned(),
                content: content.into_bytes().to_vec(),
                read_pos: 0,
            }),
            Ok(None) => {
                debug!("content not found in storage");
                Ok(WebStorage {
                    id: path.into_owned(),
                    content: vec![],
                    read_pos: 0,
                })
            }
            Err(e) => {
                debug!("error accessing storage {:?}", e);
                Err(ErrorKind::Other.into())
            }
        }
    }
    inner(path.as_ref())
}

pub fn read_to_string(path: PathBuf) -> Result<String> {
    let path = path.to_string_lossy();
    let storage = storage()?;
    match storage.get_item(&path) {
        Ok(Some(content)) => Ok(content),
        Ok(None) => {
            debug!("nothing found in storage for path {}", &path);
            Err(ErrorKind::NotFound.into())
        }
        Err(e) => {
            debug!("error reading content from storage: {:?}", e);
            Err(ErrorKind::Other.into())
        }
    }
}

fn storage() -> Result<Storage> {
    match web_sys::window().unwrap().local_storage() {
        Ok(Some(storage)) => Ok(storage),
        Ok(None) => {
            debug!("no storage available");
            Err(ErrorKind::Unsupported.into())
        }
        Err(e) => {
            debug!("error accessing storage: {:?}", e);
            Err(ErrorKind::Other.into())
        }
    }
}
