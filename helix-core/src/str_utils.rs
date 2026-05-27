//! Small polyfills for `&str` <-> char-index conversions.
//!
//! ropey 1.x exposed these from `ropey::str_utils`, but ropey 2.0 moved that
//! module to `pub(crate)`. The few call sites that still need them work with
//! raw `&str` chunks (e.g. handling LSP positions that are reported in chars),
//! so they live here.

pub fn byte_to_char_idx(s: &str, byte_idx: usize) -> usize {
    s[..byte_idx].chars().count()
}

pub fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map_or(s.len(), |(byte_idx, _)| byte_idx)
}
