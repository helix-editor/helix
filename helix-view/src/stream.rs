use anyhow::Error;
use helix_core::{encoding, ropey::iter::Chunks, Rope, RopeBuilder};

/// 8kB of buffer space for encoding and decoding `Rope`s.
const BUF_SIZE: usize = 8192;

pub trait TextBuilder {
    type Output;

    fn new() -> Self;
    fn append(&mut self, chunk: &str);
    fn finish(self) -> Self::Output;
}

pub trait ChunksIterator {
    type IntoIter<'a>: Iterator<Item = &'a str>
    where
        Self: 'a;

    fn chunks<'a>(&'a self) -> Self::IntoIter<'a>;
}

impl TextBuilder for RopeBuilder {
    type Output = Rope;

    fn new() -> Self {
        RopeBuilder::new()
    }

    fn append(&mut self, chunk: &str) {
        RopeBuilder::append(self, chunk);
    }

    fn finish(self) -> Self::Output {
        RopeBuilder::finish(self)
    }
}

impl ChunksIterator for Rope {
    type IntoIter<'a> = Chunks<'a>;

    fn chunks<'a>(&'a self) -> Self::IntoIter<'a> {
        Rope::chunks(self)
    }
}

// The documentation and implementation of this function should be up-to-date with
// its sibling function, `to_writer()`.
//
/// Decodes a stream of bytes into UTF-8, returning a `Rope` and the
/// encoding it was decoded as. The optional `encoding` parameter can
/// be used to override encoding auto-detection.
pub fn from_reader<R, T>(
    reader: &mut R,
    encoding: Option<&'static encoding::Encoding>,
) -> Result<(T::Output, &'static encoding::Encoding), Error>
where
    R: std::io::Read + ?Sized,
    T: TextBuilder,
{
    // These two buffers are 8192 bytes in size each and are used as
    // intermediaries during the decoding process. Text read into `buf`
    // from `reader` is decoded into `buf_out` as UTF-8. Once either
    // `buf_out` is full or the end of the reader was reached, the
    // contents are appended to `builder`.
    let mut buf = [0u8; BUF_SIZE];
    let mut buf_out = [0u8; BUF_SIZE];
    let mut builder = T::new();

    // By default, the encoding of the text is auto-detected via the
    // `chardetng` crate which requires sample data from the reader.
    // As a manual override to this auto-detection is possible, the
    // same data is read into `buf` to ensure symmetry in the upcoming
    // loop.
    let (encoding, mut decoder, mut slice, mut is_empty) = {
        let read = reader.read(&mut buf)?;
        let is_empty = read == 0;
        let encoding = encoding.unwrap_or_else(|| {
            let mut encoding_detector = chardetng::EncodingDetector::new();
            encoding_detector.feed(&buf, is_empty);
            encoding_detector.guess(None, true)
        });
        let decoder = encoding.new_decoder();

        // If the amount of bytes read from the reader is less than
        // `buf.len()`, it is undesirable to read the bytes afterwards.
        let slice = &buf[..read];
        (encoding, decoder, slice, is_empty)
    };

    // `RopeBuilder::append()` expects a `&str`, so this is the "real"
    // output buffer. When decoding, the number of bytes in the output
    // buffer will often exceed the number of bytes in the input buffer.
    // The `result` returned by `decode_to_str()` will state whether or
    // not that happened. The contents of `buf_str` is appended to
    // `builder` and it is reused for the next iteration of the decoding
    // loop.
    //
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

            // These variables act as the read and write cursors of `buf` and `buf_str` respectively.
            // They are necessary in case the output buffer fills before decoding of the entire input
            // loop is complete. Otherwise, the loop would endlessly iterate over the same `buf` and
            // the data inside the output buffer would be overwritten.
            total_read += read;
            total_written += written;
            match result {
                encoding::CoderResult::InputEmpty => {
                    debug_assert_eq!(slice.len(), total_read);
                    break;
                }
                encoding::CoderResult::OutputFull => {
                    debug_assert!(slice.len() > total_read);
                    builder.append(&buf_str[..total_written]);
                    total_written = 0;
                }
            }
        }
        // Once the end of the stream is reached, the output buffer is
        // flushed and the loop terminates.
        if is_empty {
            debug_assert_eq!(reader.read(&mut buf)?, 0);
            builder.append(&buf_str[..total_written]);
            break;
        }

        // Once the previous input has been processed and decoded, the next set of
        // data is fetched from the reader. The end of the reader is determined to
        // be when exactly `0` bytes were read from the reader, as per the invariants
        // of the `Read` trait.
        let read = reader.read(&mut buf)?;
        slice = &buf[..read];
        is_empty = read == 0;
    }
    let rope = builder.finish();
    Ok((rope, encoding))
}

// The documentation and implementation of this function should be up-to-date with
// its sibling function, `from_reader()`.
//
/// Encodes the text inside `rope` into the given `encoding` and writes the
/// encoded output into `writer.` As a `Rope` can only contain valid UTF-8,
/// replacement characters may appear in the encoded text.
pub async fn to_writer<'a, W, T>(
    writer: &'a mut W,
    encoding: &'static encoding::Encoding,
    text: &T,
) -> Result<(), Error>
where
    W: tokio::io::AsyncWriteExt + Unpin + ?Sized,
    T: ChunksIterator,
{
    // Text inside a `Rope` is stored as non-contiguous blocks of data called
    // chunks. The absolute size of each chunk is unknown, thus it is impossible
    // to predict the end of the chunk iterator ahead of time. Instead, it is
    // determined by filtering the iterator to remove all empty chunks and then
    // appending an empty chunk to it. This is valuable for detecting when all
    // chunks in the `Rope` have been iterated over in the subsequent loop.
    let iter = text
        .chunks()
        .filter(|c| !c.is_empty())
        .chain(std::iter::once(""));
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

        // Once the end of the iterator is reached, the output buffer is
        // flushed and the outer loop terminates.
        if is_empty {
            writer.write_all(&buf[..total_written]).await?;
            writer.flush().await?;
            break;
        }
    }
    Ok(())
}
