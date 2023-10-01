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
impl Transaction {
    pub fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write_option(writer, self.selection.as_ref(), |writer, selection| {
            write_usize(writer, selection.primary_index)?;
            write_vec(writer, selection.ranges(), |writer, range| {
                write_usize(writer, range.anchor)?;
                write_usize(writer, range.head)?;
                write_option(writer, range.old_visual_position.as_ref(), |writer, pos| {
                    write_u32(writer, pos.0)?;
                    write_u32(writer, pos.1)?;
                    Ok(())
                })?;
                Ok(())
            })?;

            Ok(())
        })?;

        write_usize(writer, self.changes.len)?;
        write_usize(writer, self.changes.len_after)?;
        write_vec(writer, self.changes.changes(), |writer, operation| {
            let variant = match operation {
                Operation::Retain(_) => 0,
                Operation::Delete(_) => 1,
                Operation::Insert(_) => 2,
            };
            write_byte(writer, variant)?;
            match operation {
                Operation::Retain(n) | Operation::Delete(n) => {
                    write_usize(writer, *n)?;
                }

                Operation::Insert(tendril) => {
                    write_string(writer, tendril.as_str())?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let selection = read_option(reader, |reader| {
            let primary_index = read_usize(reader)?;
            let ranges = read_vec(reader, |reader| {
                let anchor = read_usize(reader)?;
                let head = read_usize(reader)?;
                let old_visual_position = read_option(reader, |reader| {
                    let res = (read_u32(reader)?, read_u32(reader)?);
                    Ok(res)
                })?;
                Ok(Range {
                    anchor,
                    head,
                    old_visual_position,
                })
            })?;
            Ok(Selection {
                ranges: ranges.into(),
                primary_index,
            })
        })?;

        let len = read_usize(reader)?;
        let len_after = read_usize(reader)?;
        let changes = read_vec(reader, |reader| {
            let res = match read_byte(reader)? {
                0 => Operation::Retain(read_usize(reader)?),
                1 => Operation::Delete(read_usize(reader)?),
                2 => Operation::Insert(read_string(reader)?.into()),
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "invalid variant",
                    ))
                }
            };
            Ok(res)
        })?;
        let changes = ChangeSet {
            changes,
            len,
            len_after,
        };

        Ok(Transaction { changes, selection })
    }
}
