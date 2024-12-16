use std::io;

use ropey::iter::Chunks;
use ropey::RopeSlice;

pub struct RopeReader<'a> {
    current_chunk: &'a [u8],
    chunks: Chunks<'a>,
}

impl<'a> RopeReader<'a> {
    pub fn new(rope: RopeSlice<'a>) -> RopeReader<'a> {
        RopeReader {
            current_chunk: &[],
            chunks: rope.chunks(),
        }
    }
}

impl io::Read for RopeReader<'_> {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let buf_len = buf.len();
        loop {
            let read_bytes = self.current_chunk.read(buf)?;
            buf = &mut buf[read_bytes..];
            if buf.is_empty() {
                return Ok(buf_len);
            }

            if let Some(next_chunk) = self.chunks.next() {
                self.current_chunk = next_chunk.as_bytes();
            } else {
                return Ok(buf_len - buf.len());
            }
        }
    }
}
