//! Hard wrapping of lines at input time (`[editor.hard-wrap]`, vim's `textwidth`).
//!
//! When a non-whitespace character is inserted past `text-width`, the affected line
//! is broken at its softwrap wrap point, materializing the break as a real newline
//! plus a continuation prefix (indentation, list marker, comment token).
//! See `docs/261010_hard_wrap.md` for the design.

use std::borrow::Cow;

use helix_core::doc_formatter::{display_width, hard_wrap_point};
use helix_core::graphemes::{grapheme_width, prev_grapheme_boundary, tab_width_at};
use helix_core::indent;
use helix_core::line_ending::LineEnding;
use helix_core::syntax::config::{match_builtin_list_marker, HardWrap, ListMarkersConfig};
use helix_core::{syntax, RopeSlice, Tendril, Transaction};
use helix_stdx::rope::RopeSliceExt;
use helix_view::document::Document;

use super::Context;

/// Applies hard wrap to the lines affected by the last insertion in insert mode.
///
/// Called from `insert::insert_char` right after the insert transaction is applied.
/// Follows vim's `textwidth` rule: a line is broken when a non-whitespace character
/// is entered after the margin. The wrap change is a second transaction which
/// composes into the same pending changeset as the insert (single undo step).
pub fn hard_wrap_after_insert(cx: &mut Context, c: char) {
    if c.is_whitespace() {
        return;
    }

    let (view, doc) = current_ref!(cx.editor);
    let view_id = view.id;
    let text = doc.text().slice(..);
    let selection = doc.selection(view_id);
    let loader = cx.editor.syn_loader.load();

    // `enable` is resolved per position (innermost language layer → document
    // language → global). In practice it is a document-level decision, so bail
    // out before doing any other work when it is off.
    if !selection.iter().any(|range| {
        let byte_pos = text.char_to_byte(range.cursor(text.slice(..)));
        doc.hard_wrap_config_at(&loader, byte_pos).enable == Some(true)
    }) {
        return;
    }

    let text_width = doc.text_width();
    let tab_width = doc.tab_width();

    // Collect the distinct lines on which some cursor is now past the margin.
    let mut lines: Vec<usize> = Vec::with_capacity(selection.len());
    for range in selection {
        let cursor = range.cursor(text.slice(..));
        if cursor == 0 {
            continue;
        }
        if visual_col(text, cursor, tab_width) > text_width {
            lines.push(text.char_to_line(cursor));
        }
    }
    if lines.is_empty() {
        return;
    }
    lines.sort_unstable();
    lines.dedup();
    let doc_id = doc.id();

    let changes = {
        let config = doc.config.load();
        lines
            .iter()
            .filter_map(|&line_idx| {
                wrap_line(doc, &loader, &config, line_idx, text_width, tab_width)
            })
            .collect::<Vec<_>>()
    };
    if changes.is_empty() {
        return;
    }

    let transaction = Transaction::change(doc.text(), changes.into_iter());
    let doc = doc_mut!(cx.editor, &doc_id);
    doc.apply(&transaction, view_id);
}

/// Computes the hard wrap change for `line_idx`: the range and replacement text that
/// breaks the line at its wrap point, re-applying the continuation prefix.
fn wrap_line(
    doc: &Document,
    loader: &syntax::Loader,
    config: &helix_view::editor::Config,
    line_idx: usize,
    text_width: usize,
    tab_width: usize,
) -> Option<(usize, usize, Option<Tendril>)> {
    let text = doc.text().slice(..);
    let line_start = text.line_to_char(line_idx);
    let raw_end = line_start + text.line(line_idx).len_chars();
    // strip the line ending (the final line may not have one; mixed docs may
    // contain foreign endings, so detect via graphemes)
    let last_grapheme_start = prev_grapheme_boundary(text, raw_end);
    let content_end =
        if LineEnding::from_str(&text.slice(last_grapheme_start..raw_end).to_string()).is_some() {
            last_grapheme_start
        } else {
            raw_end
        };
    let line = text.slice(line_start..content_end);

    let first_non_ws = line.first_non_whitespace_char()?;

    // Comment token, reusing the Enter path's `continue-comments` detection. It is
    // matched before the indentation because the position right after the token is
    // where the (possibly injected) language layer and the list marker are resolved.
    let comment_token = if config.continue_comments {
        let byte_pos = text.char_to_byte(line_start + first_non_ws);
        super::continued_line_comment_token(doc, loader, text, line_idx, byte_pos)
    } else {
        None
    };
    // char offset into `line` from which the list marker is matched
    let mut content_chars = first_non_ws;
    if let Some(token) = comment_token {
        content_chars = first_non_ws + token.chars().count();
        // the token is normally followed by a space; skip it when present
        if line.get_char(content_chars) == Some(' ') {
            content_chars += 1;
        }
    }

    // Resolve the hard wrap configuration for this line. The language layer is
    // resolved at the line content (after the indentation and comment token) so that
    // embedded languages (e.g. markdown in doc comments) can override `list-markers`.
    let hard_wrap = doc.hard_wrap_config_at(loader, text.char_to_byte(line_start + content_chars));
    if hard_wrap.enable != Some(true) {
        return None;
    }

    // Indentation base: the line's leading whitespace, raised to the tree-sitter
    // indent for the wrap position when the language has an indent query. The `max`
    // direction is enforced by width: a tree-sitter indent can add structure but
    // never replaces the line's own (possibly wider) leading whitespace.
    let mut base = line.slice(..first_non_ws).to_string();
    if let Some(syntax) = doc.syntax() {
        let ts_indent = indent::indent_for_newline(
            loader,
            Some(syntax),
            &config.indent_heuristic,
            &doc.indent_style,
            tab_width,
            text,
            line_idx,
            line_start + first_non_ws,
            line_idx,
        );
        if display_width(&ts_indent, tab_width as u16) > display_width(&base, tab_width as u16) {
            base = ts_indent;
        }
    }

    // Continuation prefix = indentation (+ comment token + space) (+ marker spaces).
    // The list marker contributes its display width as plain spaces, aligned under
    // the item *content*, never a repeated marker (which would start a second item).
    let mut prefix = base;
    if let Some(token) = comment_token {
        prefix.push_str(token);
        prefix.push(' ');
    }
    if let Some(marker) = list_marker(line.slice(content_chars..), &hard_wrap) {
        let marker_width = display_width(&marker, tab_width as u16);
        if display_width(&prefix, tab_width as u16) + marker_width + 1 < text_width {
            prefix.extend(std::iter::repeat_n(' ', marker_width));
        }
    }

    let prefix_width = display_width(&prefix, tab_width as u16);
    let point = hard_wrap_point(line, tab_width as u16, text_width, prefix_width)?;

    // Absorb the space immediately before the wrapped word when there is one
    // (vim-like), otherwise insert the break before the word itself.
    let space = (point > 0 && line.get_char(point - 1) == Some(' ')).then(|| point - 1);
    let line_ending = doc.line_ending.as_str();
    let mut insert = String::with_capacity(line_ending.len() + prefix.len());
    insert.push_str(line_ending);
    insert.push_str(&prefix);
    let insert = Tendril::from(insert);
    Some(match space {
        Some(space) => (line_start + space, line_start + space + 1, Some(insert)),
        None => (line_start + point, line_start + point, Some(insert)),
    })
}

/// Returns the matched list-marker prefix for the line content, based on the
/// resolved `hard-wrap.list-markers` configuration.
fn list_marker(content: RopeSlice, hard_wrap: &HardWrap) -> Option<String> {
    match hard_wrap.list_markers.as_ref() {
        Some(ListMarkersConfig::Enable(true)) => {
            let content_str = content.to_string();
            match_builtin_list_marker(&content_str).map(|len| content_str[..len].to_string())
        }
        Some(ListMarkersConfig::Regex(pattern)) => {
            let regex = match helix_stdx::rope::Regex::new(pattern) {
                Ok(regex) => regex,
                Err(err) => {
                    log::error!("invalid `hard-wrap.list-markers` regex: {err}");
                    return None;
                }
            };
            // Match offsets are byte offsets relative to `content`; a marker must
            // start at the beginning of the content.
            let mat = regex.find(content.regex_input())?;
            if mat.start() != 0 {
                return None;
            }
            let content_str = content.to_string();
            (mat.end() > 0).then(|| content_str[..mat.end()].to_string())
        }
        Some(ListMarkersConfig::Enable(false)) | None => None,
    }
}

/// The visual column of `pos` on its line, expanding tabs.
fn visual_col(text: RopeSlice, pos: usize, tab_width: usize) -> usize {
    let line_start = text.line_to_char(text.char_to_line(pos));
    let mut col = 0;
    for grapheme in text.slice(line_start..pos).graphemes() {
        col += if grapheme == "\t" {
            tab_width_at(col, tab_width as u16)
        } else {
            let grapheme = Cow::from(grapheme);
            grapheme_width(&grapheme)
        };
    }
    col
}
