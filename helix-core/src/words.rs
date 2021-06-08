use crate::movement::{categorize, is_punctuation, is_word, skip_over_prev};
use ropey::RopeSlice;

#[must_use]
pub fn nth_prev_word_boundary(slice: RopeSlice, mut char_idx: usize, count: usize) -> usize {
    let mut with_end = false;

    for _ in 0..count {
        if char_idx == 0 {
            break;
        }

        // return if not skip while?
        skip_over_prev(slice, &mut char_idx, |ch| ch == '\n');

        with_end = skip_over_prev(slice, &mut char_idx, char::is_whitespace);

        // refetch
        let ch = slice.char(char_idx);

        if is_word(ch) {
            with_end = skip_over_prev(slice, &mut char_idx, is_word);
        } else if is_punctuation(ch) {
            with_end = skip_over_prev(slice, &mut char_idx, is_punctuation);
        }
    }

    if with_end {
        char_idx
    } else {
        char_idx + 1
    }
}

#[test]
fn different_prev_word_boundary() {
    use ropey::Rope;
    let t = |x, y| {
        let text = Rope::from(x);
        let out = nth_prev_word_boundary(text.slice(..), text.len_chars() - 1, 1);
        assert_eq!(text.slice(..out), y, r#"from "{}""#, x);
    };
    t("abcd\nefg\nwrs", "abcd\nefg\n");
    t("abcd\nefg\n", "abcd\n");
    t("abcd\n", "");
    t("hello, world!", "hello, world");
    t("hello, world", "hello, ");
    t("hello, ", "hello");
    t("hello", "");
    t("こんにちは、世界！", "こんにちは、世界");
    t("こんにちは、世界", "こんにちは、");
    t("こんにちは、", "こんにちは");
    t("こんにちは", "");
    t("この世界。", "この世界");
    t("この世界", "");
    t("お前はもう死んでいる", "");
    t("その300円です", ""); // TODO: should stop at 300
    t("唱k", ""); // TODO: should stop at 唱
    t("1 + 1 = 2", "1 + 1 = ");
    t("1 + 1 =", "1 + 1 ");
    t("1 + 1", "1 + ");
    t("1 + ", "1 ");
    t("1 ", "");
    t("1+1=2", "1+1=");
}
