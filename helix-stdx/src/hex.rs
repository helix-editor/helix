//! Functions for working with small hexadecimal strings.
//! Use `faster-hex` for big strs.

#[must_use]
const fn from_nibble(h: u8) -> u8 {
    match h {
        b'A'..=b'F' => h - b'A' + 10,
        b'a'..=b'f' => h - b'a' + 10,
        b'0'..=b'9' => h - b'0',
        _ => 0xff, // Err
    }
}

/// Decodes a nibble into an octet, repeating its value on each half.
/// That is, the value is its own padding.
///
/// # Errors
/// If the byte `h` is not an ASCII nibble
#[must_use]
pub const fn dupe_from_nibble(mut h: u8) -> Option<u8> {
    h = from_nibble(h);
    if h > 0xf {
        return None;
    }
    Some((h << 4) | h)
}

/// Decodes a big-endian nibble-pair into an octet.
///
/// # Errors
/// If any of the two bytes is not an ASCII nibble
pub const fn byte_from_pair(mut h: [u8; 2]) -> Option<u8> {
    // reuse memory
    h[0] = from_nibble(h[0]);
    h[1] = from_nibble(h[1]);
    // we could split this in 2 `if`s,
    // to avoid calling `from_nibble`,
    // but that might be slower
    if h[0] > 0xf || h[1] > 0xf {
        return None;
    }
    Some((h[0] << 4) | h[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity_nibble_lowercase() {
        for i in 0..0x10_u8 {
            let c = format!("{:x}", i);
            assert_eq!(c.len(), 1);
            assert_eq!(
                u8::from_str_radix(&c, 0x10).unwrap(),
                from_nibble(c.as_bytes()[0])
            );
        }
    }
    #[test]
    fn sanity_nibble_uppercase() {
        for i in 0..0x10_u8 {
            let c = format!("{:X}", i);
            assert_eq!(c.len(), 1);
            assert_eq!(
                u8::from_str_radix(&c, 0x10).unwrap(),
                from_nibble(c.as_bytes()[0])
            );
        }
    }

    #[test]
    fn sanity_nibble2() {
        assert_eq!(dupe_from_nibble(b'0'), Some(0));
        assert_eq!(dupe_from_nibble(b'1'), Some(0x11));
        assert_eq!(dupe_from_nibble(b'7'), Some(0x77));
        assert_eq!(dupe_from_nibble(b'a'), Some(0xaa));
        assert_eq!(dupe_from_nibble(b'f'), Some(0xff));
    }

    #[test]
    fn invalid_nibble() {
        for c in *b"gGzZ+-" {
            assert_eq!(from_nibble(c), 0xff);
        }
    }

    #[test]
    fn pair_endian() {
        assert_eq!(byte_from_pair(*b"00"), Some(0));
        assert_eq!(byte_from_pair(*b"fF"), Some(0xff));
        assert_eq!(byte_from_pair(*b"c3"), Some(0xc3));
    }
    #[test]
    fn invalid_pair() {
        assert!(byte_from_pair(*b"+1").is_none());
        assert!(byte_from_pair(*b"-1").is_none());
        assert!(byte_from_pair(*b"Gg").is_none());
        assert!(byte_from_pair(*b"0x").is_none());
    }
}
