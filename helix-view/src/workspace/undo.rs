use anyhow::Result;
use std::{
    io::{Error, ErrorKind, Read, Write},
    path::PathBuf,
};

use helix_core::{
    parse::*,
    path::{os_str_as_bytes, path_from_bytes},
};

#[derive(Default, Debug)]
pub struct UndoIndex(pub Vec<(usize, PathBuf)>);

impl UndoIndex {
    pub fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        write_vec(writer, &self.0, |writer, (id, path)| {
            write_usize(writer, *id)?;
            write_vec(writer, &os_str_as_bytes(path), |writer, byte| {
                write_byte(writer, *byte)
            })?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> Result<Self> {
        let res = read_vec(reader, |reader| {
            let id = read_usize(reader)?;
            let path = path_from_bytes(&read_vec(reader, read_byte)?)
                .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
            Ok((id, path))
        })?;
        Ok(Self(res))
    }

    pub fn find_id(&self, path: &PathBuf) -> Option<usize> {
        self.0
            .iter()
            .find_map(|(id, index_path)| (index_path == path).then(|| *id))
    }
}
