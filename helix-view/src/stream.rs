use std::io::Write;

use anyhow::Error;
use helix_core::{encoding, Rope, RopeBuilder};

/// 8kB of buffer space for encoding and decoding `Rope`s.
const BUF_SIZE: usize = 8192;

pub struct RopeWrite(RopeBuilder);
impl RopeWrite {
    pub fn finish(self) -> Rope {
        self.0.finish()
    }
}

impl Default for RopeWrite {
    fn default() -> Self {
        Self(RopeBuilder::new())
    }
}

impl std::fmt::Write for RopeWrite {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.append(s);
        Ok(())
    }
}

/// Decodes a stream of bytes into UTF-8, also returning the encoding
/// it was decoded as. The optional `encoding` parameter can be used
/// to override encoding auto-detection.
pub fn from_reader<R, B>(
    reader: &mut R,
    mut builder: B,
    encoding: Option<&'static encoding::Encoding>,
) -> Result<(B, &'static encoding::Encoding), Error>
where
    R: std::io::Read + ?Sized,
    B: std::fmt::Write,
{
    let mut buf = [0u8; BUF_SIZE];
    let mut buf_out = [0u8; BUF_SIZE];

    // Use the given `encoding` or auto-detect it.
    let (encoding, mut decoder, mut slice, mut is_empty) = {
        let read = reader.read(&mut buf)?;
        let is_empty = read == 0;
        let encoding = encoding.unwrap_or_else(|| {
            let mut encoding_detector = chardetng::EncodingDetector::new();
            encoding_detector.feed(&buf, is_empty);
            encoding_detector.guess(None, true)
        });
        let decoder = encoding.new_decoder();

        // Only slice up to how much was read.
        let slice = &buf[..read];
        (encoding, decoder, slice, is_empty)
    };

    // As it is possible to read less than the buffer's maximum from `read()`
    // even when the end of the reader has yet to be reached, the end of
    // the reader is determined only when a `read()` call returns `0`.
    //
    // SAFETY: `buf_out` is a zero-initialized array, thus it will always
    // contain valid UTF-8.
    let buf_str = unsafe { std::str::from_utf8_unchecked_mut(&mut buf_out[..]) };
    let mut total_written = 0usize;
    loop {
        let mut total_read = 0usize;

        // An inner loop is necessary as it is possible that the input buffer
        // may not be completely decoded on the first `decode_to_str()` call
        // which would happen in cases where the output buffer is filled to
        // capacity.
        loop {
            let (result, read, written, ..) = decoder.decode_to_str(
                &slice[total_read..],
                &mut buf_str[total_written..],
                is_empty,
            );

            total_read += read;
            total_written += written;
            match result {
                encoding::CoderResult::InputEmpty => {
                    debug_assert_eq!(slice.len(), total_read);
                    break;
                }
                encoding::CoderResult::OutputFull => {
                    debug_assert!(slice.len() > total_read);
                    builder.write_str(&buf_str[..total_written])?;
                    total_written = 0;
                }
            }
        }
        // Once the end of the stream is reached, the output buffer is
        // flushed and the loop terminates.
        if is_empty {
            debug_assert_eq!(reader.read(&mut buf)?, 0);
            builder.write_str(&buf_str[..total_written])?;
            break;
        }

        let read = reader.read(&mut buf)?;
        slice = &buf[..read];
        is_empty = read == 0;
    }
    Ok((builder, encoding))
}

/// Writes text into `writer` according to the given `encoding`.
pub async fn to_writer<'a, W, T>(
    writer: &'a mut W,
    encoding: &'static encoding::Encoding,
    text: T,
) -> Result<(), Error>
where
    W: tokio::io::AsyncWriteExt + Unpin + ?Sized,
    T: IntoIterator<Item = &'a str>,
{
    let iter = text.into_iter().filter(|c| !c.is_empty());
    let mut buf = [0u8; BUF_SIZE];
    let mut encoder = encoding.new_encoder();
    let mut total_written = 0usize;
    for chunk in iter {
        let is_empty = chunk.is_empty();
        let mut total_read = 0usize;

        // An inner loop is necessary as it is possible that the input buffer
        // may not be completely encoded on the first `encode_from_utf8()` call
        // which would happen in cases where the output buffer is filled to
        // capacity.
        loop {
            let (result, read, written, ..) =
                encoder.encode_from_utf8(&chunk[total_read..], &mut buf[total_written..], is_empty);

            // These variables act as the read and write cursors of `chunk` and `buf` respectively.
            // They are necessary in case the output buffer fills before encoding of the entire input
            // loop is complete. Otherwise, the loop would endlessly iterate over the same `chunk` and
            // the data inside the output buffer would be overwritten.
            total_read += read;
            total_written += written;
            match result {
                encoding::CoderResult::InputEmpty => {
                    debug_assert_eq!(chunk.len(), total_read);
                    debug_assert!(buf.len() >= total_written);
                    break;
                }
                encoding::CoderResult::OutputFull => {
                    debug_assert!(chunk.len() > total_read);
                    writer.write_all(&buf[..total_written]).await?;
                    total_written = 0;
                }
            }
        }
    }
    // Once the end of the iterator is reached, the output buffer is
    // flushed and the outer loop terminates.
    writer.write_all(&buf[..total_written]).await?;
    writer.flush().await?;
    Ok(())
}
