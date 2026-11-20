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

const FOLD_TEXT: &str = "fn a() {\n  x\n  y\n}\nz\n";

fn fold_text(text: &str, softwrap: bool, folds: &[std::ops::Range<usize>]) -> String {
    let mut annotations = TextAnnotations::default();
    annotations.add_folds(folds);
    let res = DocumentFormatter::new_at_prev_checkpoint(
        text.into(),
        &TextFormat::new_test(softwrap),
        &annotations,
        0,
    )
    .collect_to_str();
    res
}

#[test]
fn fold() {
    let text = crate::Rope::from(FOLD_TEXT);
    let fold = crate::fold::fold_range_for_lines(text.slice(..), 0, 3).unwrap();
    assert_eq!(fold, 8..17);
    assert_eq!(
        fold_text(FOLD_TEXT, false, std::slice::from_ref(&fold)),
        "fn a() {⋯} \nz \n "
    );
    assert_eq!(
        fold_text(FOLD_TEXT, true, std::slice::from_ref(&fold)),
        "fn a() {⋯} \nz \n "
    );
}

#[test]
fn nested_fold() {
    // the inner fold is hidden by the outer fold
    let folds = [8..17, 12..15];
    assert_eq!(fold_text(FOLD_TEXT, false, &folds), "fn a() {⋯} \nz \n ");
    // only the inner fold
    assert_eq!(
        fold_text(FOLD_TEXT, false, &folds[1..]),
        "fn a() { \n  x⋯y \n} \nz \n "
    );
}

#[test]
fn fold_line_indices_and_positions() {
    use crate::{char_idx_at_visual_offset, visual_offset_from_block, Position};

    let text = crate::Rope::from(FOLD_TEXT);
    let text = text.slice(..);
    let fold = 8..17;
    let mut annotations = TextAnnotations::default();
    annotations.add_folds(std::slice::from_ref(&fold));
    let text_fmt = TextFormat::new_test(false);

    // starting the formatter inside the fold starts at the fold header
    let mut formatter =
        DocumentFormatter::new_at_prev_checkpoint(text, &text_fmt, &annotations, 14);
    assert_eq!(formatter.next_char_pos(), 0);
    let graphemes: Vec<_> = formatter.by_ref().collect();
    let z = graphemes.iter().find(|g| g.char_idx == 19).unwrap();
    assert_eq!(z.line_idx, 4);
    assert_eq!(z.visual_pos, Position::new(1, 0));
    let closing = graphemes.iter().find(|g| g.char_idx == 17).unwrap();
    assert_eq!(closing.line_idx, 3);
    assert_eq!(closing.visual_pos, Position::new(0, 9));

    // hidden chars are displayed at the fold marker
    assert_eq!(
        visual_offset_from_block(text, 0, 12, &text_fmt, &annotations).0,
        Position::new(0, 8)
    );
    // moving down from the header skips the hidden lines
    assert_eq!(
        char_idx_at_visual_offset(text, 0, 1, 0, &text_fmt, &annotations),
        (19, 0)
    );
    // moving up from the line after the fold lands on the header
    assert_eq!(
        char_idx_at_visual_offset(text, 19, -1, 2, &text_fmt, &annotations),
        (2, 0)
    );
}

#[test]
fn move_vertically_skips_folds() {
    use crate::movement::{move_vertically, Direction, Movement};
    use crate::Range;

    let text = crate::Rope::from(FOLD_TEXT);
    let text = text.slice(..);
    let fold = 8..17;
    let mut annotations = TextAnnotations::default();
    annotations.add_folds(std::slice::from_ref(&fold));
    let text_fmt = TextFormat::new_test(false);
    let mut move_line = |pos: usize, dir| {
        move_vertically(
            text,
            Range::point(pos),
            dir,
            1,
            Movement::Move,
            &text_fmt,
            &mut annotations,
        )
        .head
    };
    // `fn a() {⋯}` -> `z`
    assert_eq!(move_line(0, Direction::Forward), 19);
    // `z` -> `fn a() {⋯}`
    assert_eq!(move_line(19, Direction::Backward), 0);
}

#[test]
fn move_horizontally_skips_folds() {
    use crate::movement::{move_horizontally, Direction, Movement};
    use crate::Range;

    let text = crate::Rope::from(FOLD_TEXT);
    let text = text.slice(..);
    let fold = 8..17;
    let mut annotations = TextAnnotations::default();
    annotations.add_folds(std::slice::from_ref(&fold));
    let text_fmt = TextFormat::new_test(false);
    let mut move_char = |pos: usize, dir, count| {
        move_horizontally(
            text,
            Range::point(pos),
            dir,
            count,
            Movement::Move,
            &text_fmt,
            &mut annotations,
        )
        .head
    };
    // `{` -> `⋯` -> `}`
    assert_eq!(move_char(7, Direction::Forward, 1), 8);
    assert_eq!(move_char(8, Direction::Forward, 1), 17);
    assert_eq!(move_char(7, Direction::Forward, 2), 17);
    // `}` -> `⋯` -> `{`
    assert_eq!(move_char(17, Direction::Backward, 1), 8);
    assert_eq!(move_char(17, Direction::Backward, 2), 7);
    // `z` -> line ending after `}`
    assert_eq!(move_char(19, Direction::Backward, 1), 18);
}
