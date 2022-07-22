use std::fmt::Write;

use crate::{
    graphics::{Color, Modifier, Style},
    Document, Editor, Theme, View,
};

pub type GutterFn<'doc> = Box<dyn Fn(usize, bool, &mut String) -> Option<Style> + 'doc>;
pub type Gutter =
    for<'doc> fn(&'doc Editor, &'doc Document, &View, &Theme, bool, usize) -> GutterFn<'doc>;

pub fn diagnostic<'doc>(
    _editor: &'doc Editor,
    doc: &'doc Document,
    _view: &View,
    theme: &Theme,
    _is_focused: bool,
    _width: usize,
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

pub fn line_numbers<'doc>(
    editor: &'doc Editor,
    doc: &'doc Document,
    view: &View,
    theme: &Theme,
    is_focused: bool,
    width: usize,
) -> GutterFn<'doc> {
    let text = doc.text().slice(..);
    let last_line = view.last_line(doc);
    // Whether to draw the line number for the last line of the
    // document or not.  We only draw it if it's not an empty line.
    let draw_last = text.line_to_byte(last_line) < text.len_bytes();

    let linenr = theme.get("ui.linenr");
    let linenr_select = theme.get("ui.linenr.selected");

    let current_line = doc
        .text()
        .char_to_line(doc.selection(view.id).primary().cursor(text));

    let line_number = editor.config().line_number;
    let mode = doc.mode;

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

pub fn padding<'doc>(
    _editor: &'doc Editor,
    _doc: &'doc Document,
    _view: &View,
    _theme: &Theme,
    _is_focused: bool,
    _width: usize,
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
    _width: usize,
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
            error.add_modifier(Modifier::UNDERLINED)
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
    width: usize,
) -> GutterFn<'doc> {
    let diagnostics = diagnostic(editor, doc, view, theme, is_focused, width);
    let breakpoints = breakpoints(editor, doc, view, theme, is_focused, width);

    Box::new(move |line, selected, out| {
        breakpoints(line, selected, out).or_else(|| diagnostics(line, selected, out))
    })
}
