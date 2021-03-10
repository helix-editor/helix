use crate::RopeSlice;

pub fn find_nth_next(text: RopeSlice, ch: char, pos: usize, n: usize) -> Option<usize> {
    // start searching right after pos
    let mut byte_idx = text.char_to_byte(pos + 1);

    let (mut chunks, mut chunk_byte_idx, _chunk_char_idx, _chunk_line_idx) =
        text.chunks_at_byte(byte_idx);

    let mut chunk = chunks.next().unwrap_or("");

    chunk = &chunk[(byte_idx - chunk_byte_idx)..];

    for _ in 0..n {
        loop {
            match chunk.find(ch) {
                Some(pos) => {
                    byte_idx += pos;
                    chunk = &chunk[pos + 1..];
                    break;
                }
                None => match chunks.next() {
                    Some(next_chunk) => {
                        byte_idx += chunk.len();
                        chunk = next_chunk;
                    }
                    None => {
                        log::info!("no more chunks");
                        return None;
                    }
                },
            }
        }
    }
    Some(text.byte_to_char(byte_idx))
}

pub fn find_nth_prev(text: RopeSlice, ch: char, pos: usize, n: usize) -> Option<usize> {
    // start searching right before pos
    let mut byte_idx = text.char_to_byte(pos.saturating_sub(1));

    let (mut chunks, mut chunk_byte_idx, _chunk_char_idx, _chunk_line_idx) =
        text.chunks_at_byte(byte_idx);

    let mut chunk = chunks.prev().unwrap_or("");

    // start searching from pos
    chunk = &chunk[..=byte_idx - chunk_byte_idx];

    for _ in 0..n {
        loop {
            match chunk.rfind(ch) {
                Some(pos) => {
                    byte_idx = chunk_byte_idx + pos;
                    chunk = &chunk[..pos];
                    break;
                }
                None => match chunks.prev() {
                    Some(prev_chunk) => {
                        chunk_byte_idx -= chunk.len();
                        chunk = prev_chunk;
                    }
                    None => return None,
                },
            }
        }
    }
    Some(text.byte_to_char(byte_idx))
}
