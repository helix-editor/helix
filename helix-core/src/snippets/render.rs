use std::borrow::Cow;
use std::ops::{Index, IndexMut};
use std::sync::Arc;

use helix_stdx::Range;
use ropey::{Rope, RopeSlice};
use smallvec::SmallVec;

use crate::indent::{normalize_indentation, IndentStyle};
use crate::movement::Direction;
use crate::snippets::elaborate;
use crate::snippets::TabstopIdx;
use crate::snippets::{Snippet, SnippetElement, Transform};
use crate::{selection, Selection, Tendril, Transaction};

#[derive(Debug, Clone, PartialEq)]
pub enum TabstopKind {
    Choice { choices: Arc<[Tendril]> },
    Placeholder,
    Empty,
    Transform(Arc<Transform>),
}

#[derive(Debug, PartialEq)]
pub struct Tabstop {
    pub ranges: SmallVec<[Range; 1]>,
    pub parent: Option<TabstopIdx>,
    pub kind: TabstopKind,
}

impl Tabstop {
    pub fn has_placeholder(&self) -> bool {
        matches!(
            self.kind,
            TabstopKind::Choice { .. } | TabstopKind::Placeholder
        )
    }

    pub fn selection(
        &self,
        direction: Direction,
        primary_idx: usize,
        snippet_ranges: usize,
    ) -> Selection {
        Selection::new(
            self.ranges
                .iter()
                .map(|&range| {
                    let mut range = selection::Range::new(range.start, range.end);
                    if direction == Direction::Backward {
                        range = range.flip()
                    }
                    range
                })
                .collect(),
            primary_idx * (self.ranges.len() / snippet_ranges),
        )
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct RenderedSnippet {
    pub tabstops: Vec<Tabstop>,
    pub ranges: Vec<Range>,
}

impl RenderedSnippet {
    pub fn first_selection(&self, direction: Direction, primary_idx: usize) -> Selection {
        self.tabstops[0].selection(direction, primary_idx, self.ranges.len())
    }
}

impl Index<TabstopIdx> for RenderedSnippet {
    type Output = Tabstop;
    fn index(&self, index: TabstopIdx) -> &Tabstop {
        &self.tabstops[index.0]
    }
}

impl IndexMut<TabstopIdx> for RenderedSnippet {
    fn index_mut(&mut self, index: TabstopIdx) -> &mut Tabstop {
        &mut self.tabstops[index.0]
    }
}

impl Snippet {
    pub fn prepare_render(&self) -> RenderedSnippet {
        let tabstops =
            self.tabstops()
                .map(|tabstop| Tabstop {
                    ranges: SmallVec::new(),
                    parent: tabstop.parent,
                    kind: match &tabstop.kind {
                        elaborate::TabstopKind::Choice { choices } => TabstopKind::Choice {
                            choices: choices.clone(),
                        },
                        // start out as empty: the first non-empty placeholder will change this to
                        // a placeholder automatically
                        elaborate::TabstopKind::Empty
                        | elaborate::TabstopKind::Placeholder { .. } => TabstopKind::Empty,
                        elaborate::TabstopKind::Transform(transform) => {
                            TabstopKind::Transform(transform.clone())
                        }
                    },
                })
                .collect();
        RenderedSnippet {
            tabstops,
            ranges: Vec::new(),
        }
    }

    pub fn render_at(
        &self,
        snippet: &mut RenderedSnippet,
        indent: RopeSlice<'_>,
        at_newline: bool,
        ctx: &mut SnippetRenderCtx,
        pos: usize,
    ) -> (Tendril, usize) {
        let mut ctx = SnippetRender {
            dst: snippet,
            src: self,
            indent,
            text: Tendril::new(),
            off: pos,
            ctx,
            at_newline,
        };
        ctx.render_elements(self.elements());
        let end = ctx.off;
        let text = ctx.text;
        snippet.ranges.push(Range { start: pos, end });
        (text, end - pos)
    }

    pub fn render(
        &self,
        doc: &Rope,
        selection: &Selection,
        change_range: impl FnMut(&selection::Range) -> (usize, usize),
        ctx: &mut SnippetRenderCtx,
    ) -> (Transaction, Selection, RenderedSnippet) {
        let mut snippet = self.prepare_render();
        let mut off = 0;
        let (transaction, selection) = Transaction::change_by_selection_ignore_overlapping(
            doc,
            selection,
            change_range,
            |replacement_start, replacement_end| {
                let line_idx = doc.char_to_line(replacement_start);
                let line_start = doc.line_to_char(line_idx);
                let prefix = doc.slice(line_start..replacement_start);
                let indent_len = prefix.chars().take_while(|c| c.is_whitespace()).count();
                let indent = prefix.slice(..indent_len);
                let at_newline = indent_len == replacement_start - line_start;

                let (replacement, replacement_len) = self.render_at(
                    &mut snippet,
                    indent,
                    at_newline,
                    ctx,
                    (replacement_start as i128 + off) as usize,
                );
                off +=
                    replacement_start as i128 - replacement_end as i128 + replacement_len as i128;

                Some(replacement)
            },
        );
        (transaction, selection, snippet)
    }
}

pub type VariableResolver = dyn FnMut(&str) -> Option<Cow<str>>;
pub struct SnippetRenderCtx {
    pub resolve_var: Box<VariableResolver>,
    pub tab_width: usize,
    pub indent_style: IndentStyle,
    pub line_ending: &'static str,
}

impl SnippetRenderCtx {
    #[cfg(test)]
    pub(super) fn test_ctx() -> SnippetRenderCtx {
        SnippetRenderCtx {
            resolve_var: Box::new(|_| None),
            tab_width: 4,
            indent_style: IndentStyle::Spaces(4),
            line_ending: "\n",
        }
    }
}

struct SnippetRender<'a> {
    ctx: &'a mut SnippetRenderCtx,
    dst: &'a mut RenderedSnippet,
    src: &'a Snippet,
    indent: RopeSlice<'a>,
    text: Tendril,
    off: usize,
    at_newline: bool,
}

impl SnippetRender<'_> {
    fn render_elements(&mut self, elements: &[SnippetElement]) {
        for element in elements {
            self.render_element(element)
        }
    }

    fn render_element(&mut self, element: &SnippetElement) {
        match *element {
            SnippetElement::Tabstop { idx } => self.render_tabstop(idx),
            SnippetElement::Variable {
                ref name,
                ref default,
                ref transform,
            } => {
                // TODO: allow resolve_var access to the doc and make it return rope slice
                // so we can access selections and other document content without allocating
                if let Some(val) = (self.ctx.resolve_var)(name) {
                    if let Some(transform) = transform {
                        self.push_multiline_str(&transform.apply(
                            (&*val).into(),
                            Range {
                                start: 0,
                                end: val.chars().count(),
                            },
                        ));
                    } else {
                        self.push_multiline_str(&val)
                    }
                } else if let Some(default) = default {
                    self.render_elements(default)
                }
            }
            SnippetElement::Text(ref text) => self.push_multiline_str(text),
        }
    }

    fn push_multiline_str(&mut self, text: &str) {
        let mut lines = text
            .split('\n')
            .map(|line| line.strip_suffix('\r').unwrap_or(line));
        let first_line = lines.next().unwrap();
        self.push_str(first_line, self.at_newline);
        for line in lines {
            self.push_newline();
            self.push_str(line, true);
        }
    }

    fn push_str(&mut self, mut text: &str, at_newline: bool) {
        if at_newline {
            let old_len = self.text.len();
            let old_indent_len = normalize_indentation(
                self.indent,
                text.into(),
                &mut self.text,
                self.ctx.indent_style,
                self.ctx.tab_width,
            );
            // this is ok because indentation can only be ascii chars (' ' and '\t')
            self.off += self.text.len() - old_len;
            text = &text[old_indent_len..];
            if text.is_empty() {
                self.at_newline = true;
                return;
            }
        }
        self.text.push_str(text);
        self.off += text.chars().count();
    }

    fn push_newline(&mut self) {
        self.off += self.ctx.line_ending.chars().count() + self.indent.len_chars();
        self.text.push_str(self.ctx.line_ending);
        self.text.extend(self.indent.chunks());
    }

    fn render_tabstop(&mut self, tabstop: TabstopIdx) {
        let start = self.off;
        let end = match &self.src[tabstop].kind {
            elaborate::TabstopKind::Placeholder { default } if !default.is_empty() => {
                self.render_elements(default);
                self.dst[tabstop].kind = TabstopKind::Placeholder;
                self.off
            }
            _ => start,
        };
        self.dst[tabstop].ranges.push(Range { start, end });
    }
}

#[cfg(test)]
mod tests {
    use helix_stdx::Range;

    use crate::snippets::render::Tabstop;
    use crate::snippets::{Snippet, SnippetRenderCtx};

    use super::TabstopKind;

    fn assert_snippet(snippet: &str, expect: &str, tabstops: &[Tabstop]) {
        let snippet = Snippet::parse(snippet).unwrap();
        let mut rendered_snippet = snippet.prepare_render();
        let rendered_text = snippet
            .render_at(
                &mut rendered_snippet,
                "\t".into(),
                false,
                &mut SnippetRenderCtx::test_ctx(),
                0,
            )
            .0;
        assert_eq!(rendered_text, expect);
        assert_eq!(&rendered_snippet.tabstops, tabstops);
        assert_eq!(
            rendered_snippet.ranges.last().unwrap().end,
            rendered_text.chars().count()
        );
        assert_eq!(rendered_snippet.ranges.last().unwrap().start, 0)
    }

    #[test]
    fn rust_macro() {
        assert_snippet(
            "macro_rules! ${1:name} {\n\t($3) => {\n\t\t$2\n\t};\n}",
            "macro_rules! name {\n\t    () => {\n\t        \n\t    };\n\t}",
            &[
                Tabstop {
                    ranges: vec![Range { start: 13, end: 17 }].into(),
                    parent: None,
                    kind: TabstopKind::Placeholder,
                },
                Tabstop {
                    ranges: vec![Range { start: 42, end: 42 }].into(),
                    parent: None,
                    kind: TabstopKind::Empty,
                },
                Tabstop {
                    ranges: vec![Range { start: 26, end: 26 }].into(),
                    parent: None,
                    kind: TabstopKind::Empty,
                },
                Tabstop {
                    ranges: vec![Range { start: 53, end: 53 }].into(),
                    parent: None,
                    kind: TabstopKind::Empty,
                },
            ],
        );
    }
}
