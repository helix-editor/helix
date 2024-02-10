// A minimal base64 implementation to keep from pulling in a crate for just that. It's based on
// https://github.com/marshallpierce/rust-base64 but without all the customization options.
// The biggest portion comes from
// https://github.com/marshallpierce/rust-base64/blob/a675443d327e175f735a37f574de803d6a332591/src/engine/naive.rs#L42
// Thanks, rust-base64!

// The MIT License (MIT)

// Copyright (c) 2015 Alice Maz

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use std::ops::{BitAnd, BitOr, Shl, Shr};

const PAD_BYTE: u8 = b'=';
const ENCODE_TABLE: &[u8] =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".as_bytes();
const LOW_SIX_BITS: u32 = 0x3F;

pub fn encode(input: &[u8]) -> String {
    let rem = input.len() % 3;
    let complete_chunks = input.len() / 3;
    let remainder_chunk = usize::from(rem != 0);
    let encoded_size = (complete_chunks + remainder_chunk) * 4;

    let mut output = vec![0; encoded_size];

    // complete chunks first
    let complete_chunk_len = input.len() - rem;

    let mut input_index = 0_usize;
    let mut output_index = 0_usize;
    while input_index < complete_chunk_len {
        let chunk = &input[input_index..input_index + 3];

        // populate low 24 bits from 3 bytes
        let chunk_int: u32 =
            (chunk[0] as u32).shl(16) | (chunk[1] as u32).shl(8) | (chunk[2] as u32);
        // encode 4x 6-bit output bytes
        output[output_index] = ENCODE_TABLE[chunk_int.shr(18) as usize];
        output[output_index + 1] = ENCODE_TABLE[chunk_int.shr(12_u8).bitand(LOW_SIX_BITS) as usize];
        output[output_index + 2] = ENCODE_TABLE[chunk_int.shr(6_u8).bitand(LOW_SIX_BITS) as usize];
        output[output_index + 3] = ENCODE_TABLE[chunk_int.bitand(LOW_SIX_BITS) as usize];

        input_index += 3;
        output_index += 4;
    }

    // then leftovers
    if rem == 2 {
        let chunk = &input[input_index..input_index + 2];

        // high six bits of chunk[0]
        output[output_index] = ENCODE_TABLE[chunk[0].shr(2) as usize];
        // bottom 2 bits of [0], high 4 bits of [1]
        output[output_index + 1] = ENCODE_TABLE
            [(chunk[0].shl(4_u8).bitor(chunk[1].shr(4_u8)) as u32).bitand(LOW_SIX_BITS) as usize];
        // bottom 4 bits of [1], with the 2 bottom bits as zero
        output[output_index + 2] =
            ENCODE_TABLE[(chunk[1].shl(2_u8) as u32).bitand(LOW_SIX_BITS) as usize];
        output[output_index + 3] = PAD_BYTE;
    } else if rem == 1 {
        let byte = input[input_index];
        output[output_index] = ENCODE_TABLE[byte.shr(2) as usize];
        output[output_index + 1] =
            ENCODE_TABLE[(byte.shl(4_u8) as u32).bitand(LOW_SIX_BITS) as usize];
        output[output_index + 2] = PAD_BYTE;
        output[output_index + 3] = PAD_BYTE;
    }
    String::from_utf8(output).expect("Invalid UTF8")
}

#[cfg(test)]
mod tests {
    fn compare_encode(expected: &str, target: &[u8]) {
        assert_eq!(expected, super::encode(target));
    }

    #[test]
    fn encode_rfc4648_0() {
        compare_encode("", b"");
    }

    #[test]
    fn encode_rfc4648_1() {
        compare_encode("Zg==", b"f");
    }

    #[test]
    fn encode_rfc4648_2() {
        compare_encode("Zm8=", b"fo");
    }

    #[test]
    fn encode_rfc4648_3() {
        compare_encode("Zm9v", b"foo");
    }

    #[test]
    fn encode_rfc4648_4() {
        compare_encode("Zm9vYg==", b"foob");
    }

    #[test]
    fn encode_rfc4648_5() {
        compare_encode("Zm9vYmE=", b"fooba");
    }

    #[test]
    fn encode_rfc4648_6() {
        compare_encode("Zm9vYmFy", b"foobar");
    }

    #[test]
    fn encode_all_ascii() {
        let mut ascii = Vec::<u8>::with_capacity(128);

        for i in 0..128 {
            ascii.push(i);
        }

        compare_encode(
            "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7P\
         D0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn8\
         =",
            &ascii,
        );
    }

    #[test]
    fn encode_all_bytes() {
        let mut bytes = Vec::<u8>::with_capacity(256);

        for i in 0..255 {
            bytes.push(i);
        }
        bytes.push(255); //bug with "overflowing" ranges?

        compare_encode(
            "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7P\
         D0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn\
         +AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6\
         /wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/w==",
            &bytes,
        );
    }
}
