//! Native markdown rendering for the editor.
//!
//! The buffer text is never modified. A [`Decoration`] paints over the
//! rendered document to conceal markdown markers, draw heading backgrounds and
//! turn thematic breaks into full-width rules.

use std::collections::{HashMap, HashSet};

use helix_core::unicode::width::UnicodeWidthStr;
use helix_view::graphics::{Rect, Style};
use helix_view::{Document, Theme, View};

use crate::ui::document::{LinePos, TextRenderer};
use crate::ui::text_decorations::Decoration;

#[derive(Default)]
pub struct MarkdownRender {
    lines: HashMap<usize, LineRender>,
    inline_code: Vec<InlineCode>,
    cursor_line: usize,
    selected_lines: HashSet<usize>,
}

#[derive(Default)]
struct LineRender {
    background: Option<Style>,
    paints: Vec<Paint>,
    rule: Option<Style>,
    fence: Option<FenceRender>,
}

struct Paint {
    col: usize,
    text: String,
    style: Style,
}

struct FenceRender {
    background: Style,
    text: Option<(usize, String, Style)>,
}

struct InlineCode {
    start: usize,
    end: usize,
    style: Style,
}

impl MarkdownRender {
    /// Prepare rendering data for the visible section of a markdown document.
    pub fn prepare(doc: &Document, view: &View, anchor: usize, height: u16, theme: &Theme) -> Self {
        let mut render = Self::default();
        if doc.language_name() != Some("markdown") {
            return render;
        }

        let text = doc.text().slice(..);
        if text.len_lines() == 0 {
            return render;
        }

        let cursor = doc.selection(view.id).primary().cursor(text);
        render.cursor_line = text.char_to_line(cursor);
        for range in doc.selection(view.id).iter() {
            let start = text.char_to_line(range.from());
            let end = text.char_to_line(range.to());
            for line in start..=end {
                render.selected_lines.insert(line);
            }
        }

        let anchor = anchor.min(text.len_chars());
        let start_line = text.char_to_line(anchor);
        let end_line = (start_line + height as usize + 2).min(text.len_lines());

        let heading_fallback = theme.get("markup.heading");
        let list_style = theme.get("markup.list");
        let unchecked_style = theme.try_get("markup.list.unchecked").unwrap_or(list_style);
        let checked_style = theme.try_get("markup.list.checked").unwrap_or(list_style);
        let quote_style = theme.get("markup.quote");
        let rule_style = theme
            .try_get("ui.linenr")
            .unwrap_or_else(|| theme.get("ui.text.inactive"));
        let code_background =
            background_style(theme, &["ui.cursorline", "ui.selection"]).unwrap_or_default();
        let inline_code_background = background_style(theme, &["ui.cursorline", "ui.selection"]);
        let code_info_style = theme
            .try_get("label")
            .unwrap_or_else(|| theme.get("ui.text.inactive"));

        let mut list_indents: Vec<usize> = Vec::new();
        let mut in_code: Option<CodeFence> = None;

        for line_idx in 0..end_line {
            let line_start = text.line_to_char(line_idx);
            let raw = text.line(line_idx).to_string();
            let line = raw.trim_end_matches(['\r', '\n']);
            let chars: Vec<char> = line.chars().collect();
            let indent = chars.iter().take_while(|ch| **ch == ' ').count();
            let visible = line_idx >= start_line;

            if let Some(fence) = &in_code {
                let trimmed = line.trim_start();
                if is_fence(trimmed, fence.marker, fence.len) {
                    if visible {
                        render.lines.insert(
                            line_idx,
                            LineRender {
                                fence: Some(FenceRender {
                                    background: code_background,
                                    text: None,
                                }),
                                ..Default::default()
                            },
                        );
                    }
                    in_code = None;
                    continue;
                }

                if visible {
                    render.lines.entry(line_idx).or_default().background = Some(code_background);
                }
                continue;
            }

            if let Some((marker, len)) = fence_start(line) {
                let language = line
                    .trim_start()
                    .trim_start_matches(marker)
                    .trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .trim_matches(|ch| matches!(ch, '{' | '}' | '.' | ','))
                    .to_string();

                if visible {
                    let text = (!language.is_empty())
                        .then(|| (indent, format!("󰅩 {language}"), code_info_style));
                    render.lines.insert(
                        line_idx,
                        LineRender {
                            fence: Some(FenceRender {
                                background: code_background,
                                text,
                            }),
                            ..Default::default()
                        },
                    );
                }
                in_code = Some(CodeFence { marker, len });
                continue;
            }

            if indent >= chars.len() {
                continue;
            }

            // ATX headings: `### Title`.
            if chars[indent] == '#' {
                let level = chars[indent..].iter().take_while(|ch| **ch == '#').count();
                let after = chars.get(indent + level);
                if (1..=6).contains(&level) && matches!(after, None | Some(' ')) {
                    if visible {
                        let style = theme
                            .try_get(&format!("markup.heading.{level}"))
                            .unwrap_or(heading_fallback);
                        let marker_width = level + usize::from(after == Some(&' '));
                        let icon = format!("{} ", heading_icon(level));
                        let icon_width = icon.width();
                        let text = if icon_width < marker_width {
                            format!("{icon}{}", " ".repeat(marker_width - icon_width))
                        } else {
                            icon
                        };
                        let line_render = render.lines.entry(line_idx).or_default();
                        line_render.paints.push(Paint {
                            col: indent,
                            text,
                            style,
                        });
                        line_render.background = heading_background(theme, level);
                    }
                    continue;
                }
            }

            // Block quotes, including nested `>>`.
            let mut cursor = indent;
            let mut quoted = false;
            while chars.get(cursor) == Some(&'>') {
                quoted = true;
                if visible {
                    render
                        .lines
                        .entry(line_idx)
                        .or_default()
                        .paints
                        .push(Paint {
                            col: cursor,
                            text: "▋".to_string(),
                            style: quote_style,
                        });
                }
                cursor += 1;
                if chars.get(cursor) == Some(&' ') {
                    cursor += 1;
                }
            }
            if quoted {
                continue;
            }

            // Task lists: `- [ ] todo` / `- [x] done`.
            if let Some(task) = task_list(&chars, indent) {
                update_list_indent(&mut list_indents, indent);
                if visible {
                    let (checked, bracket_col) = task;
                    let style = if checked {
                        checked_style
                    } else {
                        unchecked_style
                    };
                    let line_render = render.lines.entry(line_idx).or_default();
                    // Conceal the bullet.
                    line_render.paints.push(Paint {
                        col: indent,
                        text: " ".to_string(),
                        style,
                    });
                    // Overlay `[x]` with the icon plus padding.
                    line_render.paints.push(Paint {
                        col: bracket_col,
                        text: format!("{}  ", if checked { "󰱒" } else { "󰄱" }),
                        style,
                    });
                }
                continue;
            }

            // Unordered lists.
            if matches!(chars[indent], '-' | '*' | '+') && chars.get(indent + 1) == Some(&' ') {
                let level = update_list_indent(&mut list_indents, indent);
                if visible {
                    let icon = ["●", "○", "◆", "◇"][(level.saturating_sub(1)) % 4];
                    render
                        .lines
                        .entry(line_idx)
                        .or_default()
                        .paints
                        .push(Paint {
                            col: indent,
                            text: icon.to_string(),
                            style: list_style,
                        });
                }
                continue;
            }

            // Horizontal rules.
            let trimmed = &chars[indent..];
            if trimmed.len() >= 3
                && matches!(trimmed[0], '-' | '*' | '_')
                && trimmed.iter().all(|ch| *ch == trimmed[0])
            {
                if visible {
                    render.lines.entry(line_idx).or_default().rule = Some(rule_style);
                }
                continue;
            }

            // Inline code. Paint over the backticks and record the content
            // range so `decorate_grapheme` can give it a background.
            if visible {
                if let Some(style) = inline_code_background {
                    for (open_start, open_end, close_start, close_end) in inline_code_ranges(&chars)
                    {
                        let line_render = render.lines.entry(line_idx).or_default();
                        line_render.paints.push(Paint {
                            col: open_start,
                            text: " ".repeat(open_end - open_start),
                            style,
                        });
                        line_render.paints.push(Paint {
                            col: close_start,
                            text: " ".repeat(close_end - close_start),
                            style,
                        });
                        render.inline_code.push(InlineCode {
                            start: line_start + open_end,
                            end: line_start + close_start,
                            style,
                        });
                    }
                }
            }
        }

        render.inline_code.sort_by_key(|inline| inline.start);
        render
    }

    fn concealed(&self, doc_line: usize) -> bool {
        doc_line == self.cursor_line || self.selected_lines.contains(&doc_line)
    }
}

impl Decoration for MarkdownRender {
    fn decorate_line(&mut self, renderer: &mut TextRenderer, pos: LinePos) {
        if self.concealed(pos.doc_line) {
            return;
        }
        let Some(line) = self.lines.get(&pos.doc_line) else {
            return;
        };
        if let Some(background) = line.background {
            renderer.set_style(
                Rect::new(
                    renderer.viewport.x,
                    pos.visual_line,
                    renderer.viewport.width,
                    1,
                ),
                background,
            );
        }
    }

    fn decorate_line_end(&mut self, renderer: &mut TextRenderer, pos: LinePos) {
        if self.concealed(pos.doc_line) {
            return;
        }
        let Some(line) = self.lines.get(&pos.doc_line) else {
            return;
        };

        if let Some(style) = line.rule {
            let rule = "─".repeat(renderer.viewport.width as usize);
            renderer.set_stringn(
                renderer.viewport.x,
                pos.visual_line,
                &rule,
                renderer.viewport.width as usize,
                style,
            );
            return;
        }

        if let Some(fence) = &line.fence {
            let blank = " ".repeat(renderer.viewport.width as usize);
            renderer.set_stringn(
                renderer.viewport.x,
                pos.visual_line,
                &blank,
                renderer.viewport.width as usize,
                fence.background,
            );
            if let (Some((col, text, style)), true) = (&fence.text, pos.first_visual_line) {
                if *col >= renderer.offset.col {
                    let x = renderer.viewport.x + (col - renderer.offset.col) as u16;
                    renderer.set_stringn(
                        x,
                        pos.visual_line,
                        text,
                        (renderer.viewport.right().saturating_sub(x)) as usize,
                        *style,
                    );
                }
            }
            return;
        }

        if pos.first_visual_line {
            for paint in &line.paints {
                if paint.col < renderer.offset.col {
                    continue;
                }
                let x = renderer.viewport.x + (paint.col - renderer.offset.col) as u16;
                if x >= renderer.viewport.right() {
                    continue;
                }
                renderer.set_stringn(
                    x,
                    pos.visual_line,
                    &paint.text,
                    (renderer.viewport.right().saturating_sub(x)) as usize,
                    paint.style,
                );
            }
        }
    }

    fn decorate_grapheme(
        &mut self,
        renderer: &mut TextRenderer,
        grapheme: &helix_core::doc_formatter::FormattedGrapheme,
    ) -> usize {
        let char_idx = grapheme.char_idx;
        if self.inline_code.is_empty() {
            return usize::MAX;
        }
        let Some(inline) = self
            .inline_code
            .iter()
            .find(|inline| inline.start <= char_idx && char_idx < inline.end)
        else {
            return usize::MAX;
        };
        let col = grapheme.visual_pos.col;
        if renderer.column_in_bounds(col, grapheme.width()) {
            let x = renderer.viewport.x + (col - renderer.offset.col) as u16;
            renderer.set_style(
                Rect::new(
                    x,
                    grapheme.visual_pos.row as u16,
                    grapheme.width() as u16,
                    1,
                ),
                inline.style,
            );
        }
        usize::MAX
    }
}

struct CodeFence {
    marker: char,
    len: usize,
}

fn fence_start(line: &str) -> Option<(char, usize)> {
    let trimmed = line.trim_start();
    let marker = trimmed.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let len = trimmed.chars().take_while(|ch| *ch == marker).count();
    (len >= 3).then_some((marker, len))
}

fn is_fence(line: &str, marker: char, min_len: usize) -> bool {
    let len = line.chars().take_while(|ch| *ch == marker).count();
    len >= min_len && line.chars().skip(len).all(|ch| ch == ' ' || ch == '\t')
}

fn task_list(chars: &[char], indent: usize) -> Option<(bool, usize)> {
    if indent + 4 >= chars.len()
        || !matches!(chars[indent], '-' | '*' | '+')
        || chars[indent + 1] != ' '
        || chars[indent + 2] != '['
        || !matches!(chars[indent + 3], ' ' | 'x' | 'X')
        || chars[indent + 4] != ']'
    {
        return None;
    }
    Some((chars[indent + 3] != ' ', indent + 2))
}

fn update_list_indent(stack: &mut Vec<usize>, indent: usize) -> usize {
    while stack.last().is_some_and(|top| *top > indent) {
        stack.pop();
    }
    if stack.last() != Some(&indent) {
        stack.push(indent);
    }
    stack.len()
}

fn inline_code_ranges(chars: &[char]) -> Vec<(usize, usize, usize, usize)> {
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '`' {
            index += 1;
            continue;
        }
        let open_start = index;
        let open_len = chars[index..].iter().take_while(|ch| **ch == '`').count();
        let open_end = open_start + open_len;
        let Some(close_start) = (open_end..chars.len()).find(|idx| {
            chars[*idx] == '`'
                && chars[*idx..].iter().take_while(|ch| **ch == '`').count() >= open_len
        }) else {
            break;
        };
        let close_len = chars[close_start..]
            .iter()
            .take_while(|ch| **ch == '`')
            .count();
        ranges.push((open_start, open_end, close_start, close_start + close_len));
        index = close_start + close_len;
    }
    ranges
}

fn background_style(theme: &Theme, scopes: &[&str]) -> Option<Style> {
    scopes.iter().find_map(|scope| {
        theme
            .try_get(scope)
            .and_then(|style| style.bg)
            .map(|color| Style::default().bg(color))
    })
}

fn heading_background(theme: &Theme, level: usize) -> Option<Style> {
    let scopes: [&str; 6] = [
        "ui.selection",
        "ui.menu",
        "ui.popup",
        "ui.cursorline",
        "ui.selection",
        "ui.cursorline",
    ];
    background_style(theme, &[scopes[level.saturating_sub(1)], "ui.cursorline"])
}

fn heading_icon(level: usize) -> &'static str {
    match level {
        1 => "󰲡",
        2 => "󰲣",
        3 => "󰲥",
        4 => "󰲧",
        5 => "󰲩",
        _ => "󰲫",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_icons_cover_all_levels() {
        for level in 1..=6 {
            assert_eq!(heading_icon(level).chars().count(), 1);
        }
    }

    #[test]
    fn detects_tasks_and_bullets() {
        let line: Vec<char> = "- [ ] task".chars().collect();
        assert_eq!(task_list(&line, 0), Some((false, 2)));

        let line: Vec<char> = "- [x] done".chars().collect();
        assert_eq!(task_list(&line, 0), Some((true, 2)));

        let line: Vec<char> = "plain".chars().collect();
        assert_eq!(task_list(&line, 0), None);
    }

    #[test]
    fn tracks_list_nesting() {
        let mut stack = Vec::new();
        assert_eq!(update_list_indent(&mut stack, 0), 1);
        assert_eq!(update_list_indent(&mut stack, 2), 2);
        assert_eq!(update_list_indent(&mut stack, 4), 3);
        assert_eq!(update_list_indent(&mut stack, 0), 1);
    }

    #[test]
    fn finds_inline_code_ranges() {
        let line: Vec<char> = "a `code` b".chars().collect();
        assert_eq!(inline_code_ranges(&line), vec![(2, 3, 7, 8)]);
    }

    #[test]
    fn detects_fences() {
        assert_eq!(fence_start("```rust"), Some(('`', 3)));
        assert_eq!(fence_start("  ~~~~"), Some(('~', 4)));
        assert_eq!(fence_start("plain"), None);
        assert!(is_fence("```", '`', 3));
        assert!(!is_fence("```rust", '`', 3));
    }
}
