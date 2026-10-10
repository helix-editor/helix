use crate::doc_formatter::{DocumentFormatter, TextFormat};
use crate::text_annotations::{InlineAnnotation, Overlay, TextAnnotations};

impl TextFormat {
    fn new_test(softwrap: bool) -> Self {
        TextFormat {
            soft_wrap: softwrap,
            tab_width: 2,
            max_wrap: 3,
            max_indent_retain: 4,
            wrap_indicator: ".".into(),
            wrap_indicator_highlight: None,
            // use a prime number to allow lining up too often with repeat
            viewport_width: 17,
            soft_wrap_at_text_width: false,
        }
    }
}

impl<'t> DocumentFormatter<'t> {
    fn collect_to_str(&mut self) -> String {
        use std::fmt::Write;
        let mut res = String::new();
        let viewport_width = self.text_fmt.viewport_width;
        let soft_wrap_at_text_width = self.text_fmt.soft_wrap_at_text_width;
        let mut line = 0;

        for grapheme in self {
            if grapheme.visual_pos.row != line {
                line += 1;
                assert_eq!(grapheme.visual_pos.row, line);
                write!(res, "\n{}", ".".repeat(grapheme.visual_pos.col)).unwrap();
            }
            if !soft_wrap_at_text_width {
                assert!(
                    grapheme.visual_pos.col <= viewport_width as usize,
                    "softwrapped failed {}<={viewport_width}",
                    grapheme.visual_pos.col
                );
            }
            write!(res, "{}", grapheme.raw).unwrap();
        }

        res
    }
}

fn softwrap_text(text: &str) -> String {
    DocumentFormatter::new_at_prev_checkpoint(
        text.into(),
        &TextFormat::new_test(true),
        &TextAnnotations::default(),
        0,
    )
    .collect_to_str()
}

#[test]
fn basic_softwrap() {
    assert_eq!(
        softwrap_text(&"foo ".repeat(10)),
        "foo foo foo foo \n.foo foo foo foo \n.foo foo  "
    );
    assert_eq!(
        softwrap_text(&"fooo ".repeat(10)),
        "fooo fooo fooo \n.fooo fooo fooo \n.fooo fooo fooo \n.fooo  "
    );

    // check that we don't wrap unnecessarily
    assert_eq!(softwrap_text("\t\txxxx1xxxx2xx\n"), "    xxxx1xxxx2xx \n ");
}

#[test]
fn softwrap_indentation() {
    assert_eq!(
        softwrap_text("\t\tfoo1 foo2 foo3 foo4 foo5 foo6\n"),
        "    foo1 foo2 \n.....foo3 foo4 \n.....foo5 foo6 \n "
    );
    assert_eq!(
        softwrap_text("\t\t\tfoo1 foo2 foo3 foo4 foo5 foo6\n"),
        "      foo1 foo2 \n.foo3 foo4 foo5 \n.foo6 \n "
    );
}

#[test]
fn long_word_softwrap() {
    assert_eq!(
        softwrap_text("\t\txxxx1xxxx2xxxx3xxxx4xxxx5xxxx6xxxx7xxxx8xxxx9xxx\n"),
        "    xxxx1xxxx2xxx\n.....x3xxxx4xxxx5\n.....xxxx6xxxx7xx\n.....xx8xxxx9xxx \n "
    );
    assert_eq!(
        softwrap_text("xxxxxxxx1xxxx2xxx\n"),
        "xxxxxxxx1xxxx2xxx\n. \n "
    );
    assert_eq!(
        softwrap_text("\t\txxxx1xxxx 2xxxx3xxxx4xxxx5xxxx6xxxx7xxxx8xxxx9xxx\n"),
        "    xxxx1xxxx \n.....2xxxx3xxxx4x\n.....xxx5xxxx6xxx\n.....x7xxxx8xxxx9\n.....xxx \n "
    );
    assert_eq!(
        softwrap_text("\t\txxxx1xxx 2xxxx3xxxx4xxxx5xxxx6xxxx7xxxx8xxxx9xxx\n"),
        "    xxxx1xxx 2xxx\n.....x3xxxx4xxxx5\n.....xxxx6xxxx7xx\n.....xx8xxxx9xxx \n "
    );
}

#[test]
fn softwrap_multichar_grapheme() {
    assert_eq!(
        softwrap_text("xxxx xxxx xxx a\u{0301}bc\n"),
        "xxxx xxxx xxx \n.ábc \n "
    )
}

fn softwrap_text_at_text_width(text: &str) -> String {
    let mut text_fmt = TextFormat::new_test(true);
    text_fmt.soft_wrap_at_text_width = true;
    let annotations = TextAnnotations::default();
    let mut formatter =
        DocumentFormatter::new_at_prev_checkpoint(text.into(), &text_fmt, &annotations, 0);
    formatter.collect_to_str()
}
#[test]
fn long_word_softwrap_text_width() {
    assert_eq!(
        softwrap_text_at_text_width("xxxxxxxx1xxxx2xxx\nxxxxxxxx1xxxx2xxx"),
        "xxxxxxxx1xxxx2xxx \nxxxxxxxx1xxxx2xxx "
    );
}

fn overlay_text(text: &str, char_pos: usize, softwrap: bool, overlays: &[Overlay]) -> String {
    DocumentFormatter::new_at_prev_checkpoint(
        text.into(),
        &TextFormat::new_test(softwrap),
        TextAnnotations::default().add_overlay(overlays, None),
        char_pos,
    )
    .collect_to_str()
}

#[test]
fn overlay() {
    assert_eq!(
        overlay_text(
            "foobar",
            0,
            false,
            &[Overlay::new(0, "X"), Overlay::new(2, "\t")],
        ),
        "Xo  bar "
    );
    assert_eq!(
        overlay_text(
            &"foo ".repeat(10),
            0,
            true,
            &[
                Overlay::new(2, "\t"),
                Overlay::new(5, "\t"),
                Overlay::new(16, "X"),
            ]
        ),
        "fo   f  o foo \n.foo Xoo foo foo \n.foo foo foo  "
    );
}

fn annotate_text(text: &str, softwrap: bool, annotations: &[InlineAnnotation]) -> String {
    DocumentFormatter::new_at_prev_checkpoint(
        text.into(),
        &TextFormat::new_test(softwrap),
        TextAnnotations::default().add_inline_annotations(annotations, None),
        0,
    )
    .collect_to_str()
}

#[test]
fn annotation() {
    assert_eq!(
        annotate_text("bar", false, &[InlineAnnotation::new(0, "foo")]),
        "foobar "
    );
    assert_eq!(
        annotate_text(
            &"foo ".repeat(10),
            true,
            &[InlineAnnotation::new(0, "foo ")]
        ),
        "foo foo foo foo \n.foo foo foo foo \n.foo foo foo  "
    );
}

#[test]
fn annotation_and_overlay() {
    let annotations = [InlineAnnotation {
        char_idx: 0,
        text: "fooo".into(),
    }];
    let overlay = [Overlay {
        char_idx: 0,
        grapheme: "\t".into(),
    }];
    assert_eq!(
        DocumentFormatter::new_at_prev_checkpoint(
            "bbar".into(),
            &TextFormat::new_test(false),
            TextAnnotations::default()
                .add_inline_annotations(annotations.as_slice(), None)
                .add_overlay(overlay.as_slice(), None),
            0,
        )
        .collect_to_str(),
        "fooo  bar "
    );
}

use crate::doc_formatter::hard_wrap_point;

fn hard_wrap_point_at(text: &str, text_width: usize, prefix_width: usize) -> Option<usize> {
    hard_wrap_point(text.into(), 4, text_width, prefix_width)
}

#[test]
fn hard_wrap_basic() {
    // a word overflowing the margin at the end of the line is broken off
    assert_eq!(hard_wrap_point_at("aaa bbbb cc", 10, 0), Some(9));
    assert_eq!(hard_wrap_point_at("aaa bbbb ccc", 10, 0), Some(9));
    // lines that fit (including a trailing space) are not wrapped
    assert_eq!(hard_wrap_point_at("aaa bbbb", 10, 0), None);
    assert_eq!(hard_wrap_point_at("aaa bbbbbb ", 10, 0), None);
    // overflow in the middle of the line: the first line is filled up to the
    // margin and everything after the last in-budget space moves down
    assert_eq!(hard_wrap_point_at("aaa bbb cc ddd eee", 10, 0), Some(11));
}

#[test]
fn hard_wrap_no_mid_word_split() {
    // a word longer than the width is moved whole, not split
    assert_eq!(hard_wrap_point_at("aaaa longword", 8, 0), Some(5));
    // a line that is a single unbreakable word is never wrapped
    assert_eq!(hard_wrap_point_at("aaaaaaaaaaaaaaaaaa", 8, 0), None);
}

#[test]
fn hard_wrap_prefix_budget() {
    // the wrapped word is measured next to the continuation prefix: a word that
    // fits on its own but not next to the prefix breaks one word earlier
    assert_eq!(hard_wrap_point_at("aaaa bb ccccccccc", 10, 4), Some(5));
    // a prefix that leaves no room makes the line unbreakable
    assert_eq!(hard_wrap_point_at("aaa bbb ccc", 5, 5), None);
}

#[test]
fn hard_wrap_tabs() {
    // tab expands to 4 columns: "xxx1" ends exactly at the margin, only "xx2"
    // wraps (char offset 6, visual column 9)
    assert_eq!(hard_wrap_point_at("\txxx1 xx2", 8, 0), Some(6));
}
