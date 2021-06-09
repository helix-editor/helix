use crate::movement::{enumerated_chars, EnumeratedCharHelpers};
use ropey::RopeSlice;

#[must_use]
pub fn nth_prev_word_boundary(slice: RopeSlice, index: usize, count: usize) -> usize {
    (0..count)
        .try_fold(index, |index, _| {
            let (_, backwards) = enumerated_chars(&slice, index);
            backwards.skip_newlines().end_of_word()
        })
        .unwrap_or(0)
}

#[test]
fn different_prev_word_boundary() {
    use ropey::Rope;
    let t = |x, y| {
        let text = Rope::from(x);
        let out = nth_prev_word_boundary(text.slice(..), text.len_chars().saturating_sub(1), 1);
        assert_eq!(text.slice(..out), y, r#"from "{}""#, x);
    };
    t("abcd\nefg\nwrs", "abcd\nefg\n");
    t("abcd\nefg\n", "abcd\n");
    t("abcd\n", "");
    t("hello, world!", "hello, world");
    t("hello, world", "hello, ");
    t("hello, ", "hello");
    t("hello", "");
    t(",", "");
    t("こんにちは、世界！", "こんにちは、世界");
    t("こんにちは、世界", "こんにちは、");
    t("こんにちは、", "こんにちは");
    t("こんにちは", "");
    t("この世界。", "この世界");
    t("この世界", "");
    t("お前はもう死んでいる", "");
    t("その300円です", ""); // TODO: should stop at 300
    t("唱k", ""); // TODO: should stop at 唱
    t("，", "");
    t("1 + 1 = 2", "1 + 1 = ");
    t("1 + 1 =", "1 + 1 ");
    t("1 + 1", "1 + ");
    t("1 + ", "1 ");
    t("1 ", "");
    t("1+1=2", "1+1=");
    t("", "");
}
