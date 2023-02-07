use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Result;
use std::io::Write;

pub fn write_byte<W: Write>(writer: &mut W, byte: u8) -> Result<()> {
    writer.write_all(&[byte])?;
    Ok(())
}

pub fn write_bool<W: Write>(writer: &mut W, state: bool) -> Result<()> {
    write_byte(writer, state as u8)
}

pub fn write_u32<W: Write>(writer: &mut W, n: u32) -> Result<()> {
    writer.write_all(&n.to_ne_bytes())?;
    Ok(())
}

pub fn write_u64<W: Write>(writer: &mut W, n: u64) -> Result<()> {
    writer.write_all(&n.to_ne_bytes())?;
    Ok(())
}

pub fn write_usize<W: Write>(writer: &mut W, n: usize) -> Result<()> {
    writer.write_all(&n.to_ne_bytes())?;
    Ok(())
}

pub fn write_string<W: Write>(writer: &mut W, s: &str) -> Result<()> {
    write_usize(writer, s.len())?;
    writer.write_all(s.as_bytes())?;
    Ok(())
}

pub fn write_vec<W: Write, T>(
    writer: &mut W,
    slice: &[T],
    f: impl Fn(&mut W, &T) -> Result<()>,
) -> Result<()> {
    write_usize(writer, slice.len())?;
    for element in slice {
        f(writer, element)?;
    }
    Ok(())
}

pub fn write_option<W: Write, T>(
    writer: &mut W,
    value: Option<T>,
    f: impl Fn(&mut W, T) -> Result<()>,
) -> Result<()> {
    write_bool(writer, value.is_some())?;
    if let Some(value) = value {
        f(writer, value)?;
    }
    Ok(())
}

pub fn read_byte<R: Read>(reader: &mut R) -> Result<u8> {
    match reader.bytes().next() {
        Some(s) => s,
        None => Err(Error::from(ErrorKind::UnexpectedEof)),
    }
}

pub fn read_bool<R: Read>(reader: &mut R) -> Result<bool> {
    let res = match read_byte(reader)? {
        0 => false,
        1 => true,
        _ => {
            return Err(Error::new(
                ErrorKind::Other,
                "invalid byte to bool conversion",
            ))
        }
    };
    Ok(res)
}

pub fn read_u32<R: Read>(reader: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_ne_bytes(buf))
}

pub fn read_u64<R: Read>(reader: &mut R) -> Result<u64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(u64::from_ne_bytes(buf))
}

pub fn read_usize<R: Read>(reader: &mut R) -> Result<usize> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(usize::from_ne_bytes(buf))
}

pub fn read_string<R: Read>(reader: &mut R) -> Result<String> {
    let len = read_usize(reader)?;
    let mut buf = vec![0; len];
    reader.read_exact(&mut buf)?;

    let res = String::from_utf8(buf).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
    Ok(res)
}

pub fn read_vec<R: Read, T>(reader: &mut R, f: impl Fn(&mut R) -> Result<T>) -> Result<Vec<T>> {
    let len = read_usize(reader)?;
    let mut res = Vec::with_capacity(len);
    for _ in 0..len {
        res.push(f(reader)?);
    }
    Ok(res)
}

pub fn read_option<R: Read, T>(
    reader: &mut R,
    f: impl Fn(&mut R) -> Result<T>,
) -> Result<Option<T>> {
    let res = if read_bool(reader)? {
        Some(f(reader)?)
    } else {
        None
    };
    Ok(res)
}
