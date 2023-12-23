use std::io::Read;

use crate::{history::History, Transaction};

fn get_hash<R: Read>(reader: &mut R) -> std::io::Result<[u8; 20]> {
    const BUF_SIZE: usize = 8192;

    let mut buf = [0u8; BUF_SIZE];
    let mut hash = sha1_smol::Sha1::new();
    loop {
        let total_read = reader.read(&mut buf)?;
        if total_read == 0 {
            break;
        }

        hash.update(&buf[0..total_read]);
    }
    Ok(hash.digest().bytes())
}

pub enum Error {}

// impl History {
//     pub fn serialize<W: Write + Seek>(&self)
// }
