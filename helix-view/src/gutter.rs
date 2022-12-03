use std::fmt::Write;

use crate::{
    editor::GutterType,
    graphics::{Color, Style, UnderlineStyle},
    Document, Editor, Theme, View,
};

fn count_digits(n: usize) -> usize {
    // NOTE: if int_log gets standardized in stdlib, can use checked_log10
    // (https://github.com/rust-lang/rust/issues/70887#issue)
    std::iter::successors(Some(n), |&n| (n >= 10).then(|| n / 10)).count()
}

pub type GutterFn<'doc> = Box<dyn FnMut(usize, bool, &mut String) -> Option<Style> + 'doc>;
pub type Gutter =
    for<'doc> fn(&'doc Editor, &'doc Document, &View, &Theme, bool, usize) -> GutterFn<'doc>;

impl GutterType {
    pub fn style<'doc>(
        self,
        editor: &'doc Editor,
        doc: &'doc Document,
        view: &View,
        theme: &Theme,
        is_focused: bool,
    ) -> GutterFn<'doc> {
        match self {
            GutterType::Diagnostics => {
                diagnostics_or_breakpoints(editor, doc, view, theme, is_focused)
            }
            GutterType::LineNumbers => line_numbers(editor, doc, view, theme, is_focused),
            GutterType::Spacer => padding(editor, doc, view, theme, is_focused),
            GutterType::Diff => diff(editor, doc, view, theme, is_focused),
        }
    }

    pub fn width(self, _view: &View, doc: &Document) -> usize {
        match self {
            GutterType::Diagnostics => 1,
            GutterType::LineNumbers => line_numbers_width(_view, doc),
            GutterType::Spacer => 1,
            GutterType::Diff => 1,
        }
    }
}

pub fn diagnostic<'doc>(
    _editor: &'doc Editor,
    doc: &'doc Document,
    _view: &View,
    theme: &Theme,
    _is_focused: bool,
) -> GutterFn<'doc> {
    let warning = theme.get("warning");
    let error = theme.get("error");
    let info = theme.get("info");
    let hint = theme.get("hint");
    let diagnostics = doc.diagnostics();

    Box::new(move |line: usize, _selected: bool, out: &mut String| {
        use helix_core::diagnostic::Severity;
        if let Ok(index) = diagnostics.binary_search_by_key(&line, |d| d.line) {
            let after = diagnostics[index..].iter().take_while(|d| d.line == line);

            let before = diagnostics[..index]
                .iter()
                .rev()
                .take_while(|d| d.line == line);

            let diagnostics_on_line = after.chain(before);

            // This unwrap is safe because the iterator cannot be empty as it contains at least the item found by the binary search.
            let diagnostic = diagnostics_on_line.max_by_key(|d| d.severity).unwrap();

            write!(out, "●").unwrap();
            return Some(match diagnostic.severity {
                Some(Severity::Error) => error,
                Some(Severity::Warning) | None => warning,
                Some(Severity::Info) => info,
                Some(Severity::Hint) => hint,
            });
        }
        None
    })
}

pub fn diff<'doc>(
    _editor: &'doc Editor,
    doc: &'doc Document,
    _view: &View,
    theme: &Theme,
    _is_focused: bool,
) -> GutterFn<'doc> {
    let added = theme.get("diff.plus");
    let deleted = theme.get("diff.minus");
    let modified = theme.get("diff.delta");
    if let Some(diff_handle) = doc.diff_handle() {
        let hunks = diff_handle.hunks();
        let mut hunk_i = 0;
        let mut hunk = hunks.nth_hunk(hunk_i);
        Box::new(move |line: usize, _selected: bool, out: &mut String| {
            // truncating the line is fine here because we don't compute diffs
            // for files with more lines than i32::MAX anyways
            // we need to special case removals here
            // these technically do not have a range of lines to highlight (`hunk.after.start == hunk.after.end`).
            // However we still want to display these hunks correctly we must not yet skip to the next hunk here
            while hunk.after.end < line as u32
                || !hunk.is_pure_removal() && line as u32 == hunk.after.end
            {
                hunk_i += 1;
                hunk = hunks.nth_hunk(hunk_i);
            }

            if hunk.after.start > line as u32 {
                return None;
            }

            let (icon, style) = if hunk.is_pure_insertion() {
                ("▍", added)
            } else if hunk.is_pure_removal() {
                ("▔", deleted)
            } else {
                ("▍", modified)
            };

            write!(out, "{}", icon).unwrap();
            Some(style)
        })
    } else {
        Box::new(move |_, _, _| None)
    }
}

pub fn line_numbers<'doc>(
    editor: &'doc Editor,
    doc: &'doc Document,
    view: &View,
    theme: &Theme,
    is_focused: bool,
) -> GutterFn<'doc> {
    let text = doc.text().slice(..);
    let last_line = view.last_line(doc);
    let width = GutterType::LineNumbers.width(view, doc);

    // Whether to draw the line number for the last line of the
    // document or not.  We only draw it if it's not an empty line.
    let draw_last = text.line_to_byte(last_line) < text.len_bytes();

    let linenr = theme.get("ui.linenr");
    let linenr_select = theme.get("ui.linenr.selected");

    let current_line = doc
        .text()
        .char_to_line(doc.selection(view.id).primary().cursor(text));

    let line_number = editor.config().line_number;
    let mode = editor.mode;

    Box::new(move |line: usize, selected: bool, out: &mut String| {
        if line == last_line && !draw_last {
            write!(out, "{:>1$}", '~', width).unwrap();
            Some(linenr)
        } else {
            use crate::{document::Mode, editor::LineNumber};

            let relative = line_number == LineNumber::Relative
                && mode != Mode::Insert
                && is_focused
                && current_line != line;

            let display_num = if relative {
                abs_diff(current_line, line)
            } else {
                line + 1
            };

            let style = if selected && is_focused {
                linenr_select
            } else {
                linenr
            };

            write!(out, "{:>1$}", display_num, width).unwrap();
            Some(style)
        }
    })
}

pub fn line_numbers_width(_view: &View, doc: &Document) -> usize {
    let text = doc.text();
    let last_line = text.len_lines().saturating_sub(1);
    let draw_last = text.line_to_byte(last_line) < text.len_bytes();
    let last_drawn = if draw_last { last_line + 1 } else { last_line };

    // set a lower bound to 2-chars to minimize ambiguous relative line numbers
    std::cmp::max(count_digits(last_drawn), 2)
}

pub fn padding<'doc>(
    _editor: &'doc Editor,
    _doc: &'doc Document,
    _view: &View,
    _theme: &Theme,
    _is_focused: bool,
) -> GutterFn<'doc> {
    Box::new(|_line: usize, _selected: bool, _out: &mut String| None)
}

#[inline(always)]
const fn abs_diff(a: usize, b: usize) -> usize {
    if a > b {
        a - b
    } else {
        b - a
    }
}

pub fn breakpoints<'doc>(
    editor: &'doc Editor,
    doc: &'doc Document,
    _view: &View,
    theme: &Theme,
    _is_focused: bool,
) -> GutterFn<'doc> {
    let warning = theme.get("warning");
    let error = theme.get("error");
    let info = theme.get("info");

    let breakpoints = doc.path().and_then(|path| editor.breakpoints.get(path));

    let breakpoints = match breakpoints {
        Some(breakpoints) => breakpoints,
        None => return Box::new(move |_, _, _| None),
    };

    Box::new(move |line: usize, _selected: bool, out: &mut String| {
        let breakpoint = breakpoints
            .iter()
            .find(|breakpoint| breakpoint.line == line)?;

        let mut style = if breakpoint.condition.is_some() && breakpoint.log_message.is_some() {
            error.underline_style(UnderlineStyle::Line)
        } else if breakpoint.condition.is_some() {
            error
        } else if breakpoint.log_message.is_some() {
            info
        } else {
            warning
        };

        if !breakpoint.verified {
            // Faded colors
            style = if let Some(Color::Rgb(r, g, b)) = style.fg {
                style.fg(Color::Rgb(
                    ((r as f32) * 0.4).floor() as u8,
                    ((g as f32) * 0.4).floor() as u8,
                    ((b as f32) * 0.4).floor() as u8,
                ))
            } else {
                style.fg(Color::Gray)
            }
        };

        let sym = if breakpoint.verified { "▲" } else { "⊚" };
        write!(out, "{}", sym).unwrap();
        Some(style)
    })
}

pub fn diagnostics_or_breakpoints<'doc>(
    editor: &'doc Editor,
    doc: &'doc Document,
    view: &View,
    theme: &Theme,
    is_focused: bool,
) -> GutterFn<'doc> {
    let mut diagnostics = diagnostic(editor, doc, view, theme, is_focused);
    let mut breakpoints = breakpoints(editor, doc, view, theme, is_focused);

    Box::new(move |line, selected, out| {
        breakpoints(line, selected, out).or_else(|| diagnostics(line, selected, out))
    })
}
