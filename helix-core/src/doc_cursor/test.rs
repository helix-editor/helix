use crate::doc_cursor::{CursorConfig, DocumentCursor};

const WRAP_INDENT: u16 = 1;
impl CursorConfig {
    fn new_test(softwrap: bool) -> CursorConfig {
        CursorConfig {
            softwrap,
            tab_width: 2,
            max_wrap: 3,
            max_indent_retain: 4,
            wrap_indent: WRAP_INDENT,
            // use a prime number to allow linging up too often with repear
            viewport_width: 17,
        }
    }
}

impl<'t> DocumentCursor<'t, (), ()> {
    fn new_test(text: &'t str, char_pos: usize, softwrap: bool) -> Self {
        Self::new_at_prev_line(text.into(), CursorConfig::new_test(softwrap), char_pos, ())
    }

    fn collect_to_str(&mut self, res: &mut String) {
        use std::fmt::Write;
        let wrap_indent = self.config.wrap_indent;
        let viewport_width = self.config.viewport_width;
        let mut line_width = 0;

        while let Some(mut word) = self.advance() {
            let mut word_width_check = 0;
            let word_width = word.visual_width;
            for grapheme in word.consume_graphemes(self) {
                word_width_check += grapheme.width() as usize;
                write!(res, "{}", grapheme.grapheme).unwrap();
            }
            assert_eq!(word_width, word_width_check);
            line_width += word.visual_width;

            if let Some(line_break) = word.terminating_linebreak {
                assert!(
                    line_width <= viewport_width as usize,
                    "softwrapped failed {line_width}<={viewport_width}"
                );
                res.push('\n');
                if line_break.is_softwrap {
                    for i in 0..line_break.indent {
                        if i < wrap_indent {
                            res.push('.');
                        } else {
                            res.push(' ')
                        }
                    }
                } else {
                    assert_eq!(line_break.indent, 0);
                }
                line_width = line_break.indent as usize;
            }
        }

        for grapheme in self.finish().consume_graphemes(self) {
            write!(res, "{}", grapheme.grapheme).unwrap();
        }
        assert!(
            line_width <= viewport_width as usize,
            "softwrapped failed {line_width}<={viewport_width}"
        );
    }
}

fn softwrap_text(text: &str, char_pos: usize) -> String {
    let mut cursor = DocumentCursor::new_test(text, char_pos, true);
    let mut res = String::new();
    for i in 0..cursor.visual_pos().col {
        if i < WRAP_INDENT as usize {
            res.push('.');
        } else {
            res.push(' ')
        }
    }
    cursor.collect_to_str(&mut res);
    res
}

#[test]
fn basic_softwrap() {
    assert_eq!(
        softwrap_text(&"foo ".repeat(10), 0),
        "foo foo foo foo \n.foo foo foo foo \n.foo foo "
    );
    assert_eq!(
        softwrap_text(&"fooo ".repeat(10), 0),
        "fooo fooo fooo \n.fooo fooo fooo \n.fooo fooo fooo \n.fooo "
    );

    // check that we don't wrap unecessarly
    assert_eq!(
        softwrap_text("\t\txxxx1xxxx2xx\n", 0),
        "    xxxx1xxxx2xx \n"
    );
}

#[test]
fn softwrap_indentation() {
    assert_eq!(
        softwrap_text("\t\tfoo1 foo2 foo3 foo4 foo5 foo6\n", 0),
        "    foo1 foo2 \n.    foo3 foo4 \n.    foo5 foo6 \n"
    );
    assert_eq!(
        softwrap_text("\t\t\tfoo1 foo2 foo3 foo4 foo5 foo6\n", 0),
        "      foo1 foo2 \n.foo3 foo4 foo5 \n.foo6 \n"
    );
}

#[test]
fn long_word_softwrap() {
    assert_eq!(
        softwrap_text("\t\txxxx1xxxx2xxxx3xxxx4xxxx5xxxx6xxxx7xxxx8xxxx9xxx\n", 0),
        "    xxxx1xxxx2xxx\n.    x3xxxx4xxxx5\n.    xxxx6xxxx7xx\n.    xx8xxxx9xxx \n"
    );
    assert_eq!(
        softwrap_text("xxxxxxxx1xxxx2xxx\n", 0),
        "xxxxxxxx1xxxx2xxx\n. \n"
    );
    assert_eq!(
        softwrap_text("\t\txxxx1xxxx 2xxxx3xxxx4xxxx5xxxx6xxxx7xxxx8xxxx9xxx\n", 0),
        "    xxxx1xxxx \n.    2xxxx3xxxx4x\n.    xxx5xxxx6xxx\n.    x7xxxx8xxxx9\n.    xxx \n"
    );
    assert_eq!(
        softwrap_text("\t\txxxx1xxx 2xxxx3xxxx4xxxx5xxxx6xxxx7xxxx8xxxx9xxx\n", 0),
        "    xxxx1xxx 2xxx\n.    x3xxxx4xxxx5\n.    xxxx6xxxx7xx\n.    xx8xxxx9xxx \n"
    );
}

#[test]
fn softwrap_checkpoint() {
    assert_eq!(
        softwrap_text(&"foo ".repeat(10), 4),
        "foo foo foo foo \n.foo foo foo foo \n.foo foo "
    );
    let text = "foo ".repeat(10);
    assert_eq!(softwrap_text(&text, 18), ".foo foo foo foo \n.foo foo ");
    println!("{}", &text[32..]);
    assert_eq!(softwrap_text(&"foo ".repeat(10), 32), ".foo foo ");
}
