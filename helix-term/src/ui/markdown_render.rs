//! Native markdown rendering for the editor.
//!
//! The buffer text is never modified. A [`Decoration`] paints over the
//! rendered document to conceal markdown markers, draw heading backgrounds and
//! turn thematic breaks into full-width rules.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use helix_core::graphemes::grapheme_width;
use helix_core::unicode::width::{UnicodeWidthChar, UnicodeWidthStr};
use helix_view::graphics::{Modifier, Rect, Style};
use helix_view::{editor::MarkdownLinkIcon, Document, DocumentId, Theme, View};

use crate::ui::document::{LinePos, TextRenderer};
use crate::ui::text_decorations::Decoration;

/// Blanks the text viewport. Used for directly opened image files so the
/// binary buffer text doesn't show around the placed image.
pub struct BlankViewport {
    style: Style,
}

impl BlankViewport {
    pub fn new(theme: &Theme) -> Self {
        Self {
            style: theme.get("ui.background"),
        }
    }
}

impl Decoration for BlankViewport {
    fn decorate_line_end(&mut self, renderer: &mut TextRenderer, pos: LinePos) {
        let blank = " ".repeat(renderer.viewport.width as usize);
        renderer.set_stringn(
            renderer.viewport.x,
            pos.visual_line,
            &blank,
            renderer.viewport.width as usize,
            self.style,
        );
    }
}

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
    /// Display-column ranges to remove after painting (emphasis/link
    /// delimiters). Content to the right is shifted left, matching Neovim
    /// conceal rather than leaving a gap.
    removals: Vec<(usize, usize)>,
    rule: Option<Style>,
    fence: Option<FenceRender>,
    table: Option<TableKind>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TableKind {
    Header,
    Delimiter,
    Body,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TableAlignment {
    Default,
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug)]
struct TableColumn {
    width: usize,
    content_width: usize,
    padding_left: usize,
    padding_right: usize,
    alignment: TableAlignment,
}

/// Layout for one pipe table. `end` is exclusive.
#[derive(Clone, Debug)]
struct TableLayout {
    columns: Vec<TableColumn>,
    indent: usize,
    header: usize,
    end: usize,
}

/// Style with inherited emphasis/underline modifiers cleared. Painting over
/// cells that used to hold a link destination must not keep its underline.
fn plain(style: Style) -> Style {
    Style {
        sub_modifier: Modifier::all(),
        ..style
    }
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

/// Prefix state needed to render a markdown line without rescanning the
/// document from the top on every frame.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
struct LineMeta {
    /// Code fence active before this line.
    in_code: Option<CodeFence>,
    /// Callout block style inherited by a quote continuation line.
    callout: Option<&'static str>,
    /// Nesting level for a task/unordered list line.
    list_level: Option<usize>,
}

/// Parser state after a line. Cached per line so an edit can resume at the
/// changed line instead of rescanning the document prefix.
#[derive(Clone, Default, PartialEq, Eq)]
struct LineState {
    in_code: Option<CodeFence>,
    callout: Option<&'static str>,
    list_indents: Vec<usize>,
}

struct MarkdownCache {
    version: i32,
    text: helix_core::Rope,
    lines: Vec<LineMeta>,
    states: Vec<LineState>,
}

fn markdown_cache() -> &'static Mutex<HashMap<DocumentId, MarkdownCache>> {
    static CACHE: OnceLock<Mutex<HashMap<DocumentId, MarkdownCache>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn visible_line_meta(
    doc: &Document,
    text: helix_core::RopeSlice,
    start_line: usize,
    end_line: usize,
) -> Vec<LineMeta> {
    let version = doc.version();
    let mut cache = markdown_cache().lock().unwrap();
    let entry = cache.entry(doc.id()).or_insert_with(|| {
        let mut state = LineState::default();
        let (lines, states) = build_line_meta_range(text, 0, text.len_lines(), &mut state);
        MarkdownCache {
            version,
            text: doc.text().clone(),
            lines,
            states,
        }
    });
    if entry.version != version {
        entry.version = version;
        update_line_meta(entry, text);
    }
    entry
        .lines
        .get(start_line..end_line.min(entry.lines.len()))
        .unwrap_or_default()
        .to_vec()
}

/// Update cached line metadata after an edit. Lines before the first change
/// are reused as-is; the changed region is reparsed from the cached state
/// after the previous line, and the unaffected suffix is reused when the
/// parser state still matches.
fn update_line_meta(cache: &mut MarkdownCache, text: helix_core::RopeSlice) {
    let old = cache.text.slice(..);
    let min_lines = old.len_lines().min(text.len_lines());
    let mut prefix = 0;
    while prefix < min_lines && old.line(prefix) == text.line(prefix) {
        prefix += 1;
    }
    let mut old_suffix = old.len_lines();
    let mut new_suffix = text.len_lines();
    while old_suffix > prefix
        && new_suffix > prefix
        && old.line(old_suffix - 1) == text.line(new_suffix - 1)
    {
        old_suffix -= 1;
        new_suffix -= 1;
    }

    let prefix = prefix.min(cache.lines.len()).min(cache.states.len());
    let mut state = prefix
        .checked_sub(1)
        .and_then(|index| cache.states.get(index))
        .cloned()
        .unwrap_or_default();
    let mut lines = cache.lines[..prefix].to_vec();
    let mut states = cache.states[..prefix].to_vec();

    let mut line = prefix;
    while line < new_suffix {
        let raw = text.line(line).to_string();
        let meta = parse_markdown_line(raw.trim_end_matches(['\r', '\n']), &mut state);
        lines.push(meta);
        states.push(state.clone());
        line += 1;
    }

    let reusable_suffix = old_suffix > 0
        && old_suffix <= cache.lines.len()
        && cache.states.get(old_suffix - 1) == Some(&state);
    if reusable_suffix {
        lines.extend_from_slice(&cache.lines[old_suffix..]);
        states.extend_from_slice(&cache.states[old_suffix..]);
    } else {
        while line < text.len_lines() {
            let raw = text.line(line).to_string();
            let meta = parse_markdown_line(raw.trim_end_matches(['\r', '\n']), &mut state);
            lines.push(meta);
            states.push(state.clone());
            line += 1;
        }
    }

    cache.lines = lines;
    cache.states = states;
    cache.text = helix_core::Rope::from(text);
}

#[cfg(test)]
fn build_line_meta(text: helix_core::RopeSlice) -> Vec<LineMeta> {
    let mut state = LineState::default();
    build_line_meta_range(text, 0, text.len_lines(), &mut state).0
}

fn build_line_meta_range(
    text: helix_core::RopeSlice,
    start_line: usize,
    end_line: usize,
    state: &mut LineState,
) -> (Vec<LineMeta>, Vec<LineState>) {
    let mut lines = Vec::with_capacity(end_line.saturating_sub(start_line));
    let mut states = Vec::with_capacity(end_line.saturating_sub(start_line));
    for line_idx in start_line..end_line {
        let raw = text.line(line_idx).to_string();
        let meta = parse_markdown_line(raw.trim_end_matches(['\r', '\n']), state);
        lines.push(meta);
        states.push(state.clone());
    }
    (lines, states)
}

fn parse_markdown_line(line: &str, state: &mut LineState) -> LineMeta {
    let chars: Vec<char> = line.chars().collect();
    let indent = chars.iter().take_while(|ch| **ch == ' ').count();
    let mut line_meta = LineMeta {
        in_code: state.in_code,
        ..Default::default()
    };

    if let Some(fence) = state.in_code {
        if is_fence(line.trim_start(), fence.marker, fence.len) {
            state.in_code = None;
        }
        return line_meta;
    }

    if let Some((marker, len)) = fence_start(line) {
        state.callout = None;
        state.in_code = Some(CodeFence { marker, len });
        return line_meta;
    }

    let mut content_indent = indent;
    if indent < chars.len() && chars[indent] == '>' {
        let mut cursor = indent + 1;
        while chars.get(cursor) == Some(&' ') {
            cursor += 1;
        }
        if let Some((_, _, style)) = parse_callout(&chars, cursor) {
            state.callout = Some(style);
        } else {
            line_meta.callout = state.callout;
            content_indent = cursor;
        }
    } else {
        state.callout = None;
    }

    if task_list(&chars, content_indent).is_some() {
        update_list_indent(&mut state.list_indents, content_indent);
    } else if content_indent < chars.len()
        && matches!(chars[content_indent], '-' | '*' | '+')
        && chars.get(content_indent + 1) == Some(&' ')
    {
        line_meta.list_level = Some(update_list_indent(&mut state.list_indents, content_indent));
    }

    line_meta
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
        let tab_width = doc.tab_width().max(1);

        let cursor = doc.selection(view.id).primary().cursor(text);
        render.cursor_line = text.char_to_line(cursor);
        for range in doc.selection(view.id).iter() {
            let start = text.char_to_line(range.from());
            // `to()` is exclusive: if it sits at a line start, don't mark
            // that line as selected (it has no selected characters).
            let end = if range.to() > range.from() {
                text.char_to_line(range.to().saturating_sub(1))
            } else {
                start
            };
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
        let link_icon_style = theme
            .try_get("markup.link.label")
            .unwrap_or_else(|| theme.get("markup.link"));
        let table_border_style = theme.get("markup.raw");
        let table_head_background =
            background_style(theme, &["ui.menu", "ui.cursorline", "ui.selection"]);

        let link_custom: Vec<MarkdownLinkIcon> = doc.config.load().markdown.link_icons.clone();
        let available_width = view.inner_width(doc) as usize;
        // Walk back to the start of a table that straddles the top of the
        // viewport so a scrolled-off header still has a known layout.
        let mut table_from = start_line.saturating_sub(8);
        while table_from > 0 && is_table_row(&text.line(table_from - 1).to_string()) {
            table_from -= 1;
        }
        // Only classify rows near the viewport. Scanning to the end of the
        // document here made table detection O(file size) on every frame.
        let table_kinds = detect_tables(text, table_from, end_line);
        let mut table_layouts: HashMap<usize, TableLayout> = HashMap::new();
        let mut table_borders: HashMap<usize, String> = HashMap::new();
        let mut seen_table_headers = HashSet::new();
        for (&header, &kind) in &table_kinds {
            if kind != TableKind::Header || !seen_table_headers.insert(header) {
                continue;
            }
            let Some(layout) = parse_table_layout(text, header, available_width) else {
                continue;
            };
            for row in layout.header..layout.end {
                table_layouts.insert(row, layout.clone());
            }
            // render-markdown.nvim draws the outer border on adjacent blank
            // lines when possible, and only falls back to virtual lines.
            let prefix = " ".repeat(layout.indent);
            if header > 0 && text.line(header - 1).to_string().trim().is_empty() {
                let border = format_table_border(&layout.columns, '┌', '┬', '┐');
                table_borders.insert(header - 1, format!("{prefix}{border}"));
            }
            if layout.end < text.len_lines() && text.line(layout.end).to_string().trim().is_empty()
            {
                let border = format_table_border(&layout.columns, '└', '┴', '┘');
                table_borders.insert(layout.end, format!("{prefix}{border}"));
            }
        }
        for (line, border) in &table_borders {
            render.lines.entry(*line).or_default().paints.push(Paint {
                col: 0,
                text: border.clone(),
                style: plain(table_border_style),
            });
        }
        // Prefix state is cached per document version, so scrolling and
        // redraws only parse the visible lines.
        let line_meta = visible_line_meta(doc, text, start_line, end_line);

        for (offset, line_idx) in (start_line..end_line).enumerate() {
            let meta = line_meta.get(offset).copied().unwrap_or_default();
            let line_start = text.line_to_char(line_idx);
            let raw = text.line(line_idx).to_string();
            let line = raw.trim_end_matches(['\r', '\n']);
            let chars: Vec<char> = line.chars().collect();
            let mut indent = chars.iter().take_while(|ch| **ch == ' ').count();
            let visible = true;

            if let Some(fence) = meta.in_code {
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
                    continue;
                }

                if visible {
                    render.lines.entry(line_idx).or_default().background = Some(code_background);
                }
                continue;
            }

            if let Some((marker, _)) = fence_start(line) {
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
                continue;
            }

            if indent >= chars.len() {
                continue;
            }

            // Pipe tables take precedence over lists/quotes: rows start with
            // `|` which no other block marker uses.
            if let (Some(kind), Some(layout)) =
                (table_kinds.get(&line_idx), table_layouts.get(&line_idx))
            {
                if visible {
                    let line_render = render.lines.entry(line_idx).or_default();
                    line_render.table = Some(*kind);
                    if *kind == TableKind::Header {
                        line_render.background = table_head_background;
                    }
                    let border_style = plain(table_border_style);
                    if *kind == TableKind::Delimiter {
                        line_render.paints.push(Paint {
                            col: layout.indent,
                            text: format_table_separator(&layout.columns),
                            style: border_style,
                        });
                    } else {
                        let cells = table_cells(line);
                        paint_table_row(
                            line_render,
                            layout.indent,
                            &cells,
                            &layout.columns,
                            plain(theme.get("ui.text")),
                            border_style,
                        );
                    }
                }
                continue;
            }

            // ATX headings: `### Title`.
            let mut inline_skip = 0;
            let mut handled_block = false;
            if chars[indent] == '#' {
                let level = chars[indent..].iter().take_while(|ch| **ch == '#').count();
                let after = chars.get(indent + level);
                if (1..=6).contains(&level) && matches!(after, None | Some(' ')) {
                    let marker_width = level + usize::from(after == Some(&' '));
                    if visible {
                        let style = theme
                            .try_get(&format!("markup.heading.{level}"))
                            .unwrap_or(heading_fallback);
                        let icon = format!("{} ", heading_icon(level));
                        let icon_width = icon.width();
                        let text = if icon_width < marker_width {
                            format!("{icon}{}", " ".repeat(marker_width - icon_width))
                        } else {
                            icon
                        };
                        let line_render = render.lines.entry(line_idx).or_default();
                        line_render.paints.push(Paint {
                            col: display_col(&chars, indent, tab_width),
                            text,
                            style,
                        });
                        line_render.background = heading_background(theme, level);
                        // Fall through to inline link scanning for heading content.
                        inline_skip = indent + marker_width;
                    }
                    handled_block = true;
                    if !visible {
                        continue;
                    }
                }
                // Not a heading: fall through to the other block checks.
            }

            // Block quotes, including nested `>>` and Obsidian callouts.
            if !handled_block {
                let mut cursor = indent;
                let mut markers = Vec::new();
                while chars.get(cursor) == Some(&'>') {
                    markers.push(cursor);
                    cursor += 1;
                    while chars.get(cursor) == Some(&' ') {
                        cursor += 1;
                    }
                }
                if !markers.is_empty() {
                    // Obsidian callout: `> [!NOTE] Title`.
                    let callout = parse_callout(&chars, cursor);
                    // Continuation lines of a callout block reuse its
                    // highlight for the quote markers.
                    let body_style = meta
                        .callout
                        .map(|name| theme.try_get(name).unwrap_or(quote_style));
                    if visible {
                        if let Some((name_end, rendered, style_name)) = &callout {
                            let style = theme.try_get(style_name).unwrap_or(quote_style);
                            let line_render = render.lines.entry(line_idx).or_default();
                            for col in &markers {
                                line_render.paints.push(Paint {
                                    col: display_col(&chars, *col, tab_width),
                                    text: "▋".to_string(),
                                    style,
                                });
                            }
                            let width = name_end - cursor;
                            let mut text = rendered.clone();
                            while text.width() < width {
                                text.push(' ');
                            }
                            line_render.paints.push(Paint {
                                col: display_col(&chars, cursor, tab_width),
                                text,
                                style,
                            });
                        } else {
                            let style = body_style.unwrap_or(quote_style);
                            let line_render = render.lines.entry(line_idx).or_default();
                            for col in &markers {
                                line_render.paints.push(Paint {
                                    col: display_col(&chars, *col, tab_width),
                                    text: "▋".to_string(),
                                    style,
                                });
                            }
                        }
                    }
                    if callout.is_some() {
                        // Fall through to link scanning for the title text
                        // after the callout marker.
                        if let Some((end, _, _)) = &callout {
                            inline_skip = *end;
                        }
                        handled_block = true;
                        if !visible {
                            continue;
                        }
                    } else {
                        // The quote marker is painted; continue processing
                        // the quoted content (lists, links, emphasis) with the
                        // content start as the new indent.
                        indent = cursor;
                        inline_skip = cursor;
                    }
                }
            }

            // Task lists: `- [ ] todo` / `- [x] done`.
            if !handled_block {
                if let Some(task) = task_list(&chars, indent) {
                    let (checked, bracket_col) = task;
                    if visible {
                        let style = if checked {
                            checked_style
                        } else {
                            unchecked_style
                        };
                        let line_render = render.lines.entry(line_idx).or_default();
                        // Conceal the bullet.
                        line_render.paints.push(Paint {
                            col: display_col(&chars, indent, tab_width),
                            text: " ".to_string(),
                            style,
                        });
                        // Overlay `[x]` with the icon plus padding.
                        line_render.paints.push(Paint {
                            col: display_col(&chars, bracket_col, tab_width),
                            text: format!("{}  ", if checked { "󰱒" } else { "󰄱" }),
                            style,
                        });
                        // Fall through to inline link scanning for item content.
                        inline_skip = bracket_col + 4;
                    }
                    handled_block = true;
                    if !visible {
                        continue;
                    }
                }
            }

            // Unordered lists.
            if !handled_block
                && indent < chars.len()
                && matches!(chars[indent], '-' | '*' | '+')
                && chars.get(indent + 1) == Some(&' ')
            {
                let level = meta.list_level.unwrap_or(1);
                if visible {
                    let icon = ["●", "○", "◆", "◇"][(level.saturating_sub(1)) % 4];
                    render
                        .lines
                        .entry(line_idx)
                        .or_default()
                        .paints
                        .push(Paint {
                            col: display_col(&chars, indent, tab_width),
                            text: icon.to_string(),
                            style: list_style,
                        });
                    // Fall through to inline link scanning for item content.
                    inline_skip = indent + 2;
                }
                handled_block = true;
                if !visible {
                    continue;
                }
            }

            // Horizontal rules.
            if !handled_block && indent < chars.len() {
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
            }

            // Inline links, then inline code. Both paint over markers while
            // keeping widths stable so text never shifts.
            if visible {
                paint_inline_links(
                    &mut render,
                    line_idx,
                    &chars,
                    inline_skip,
                    link_icon_style,
                    &link_custom,
                    tab_width,
                );
                paint_inline_emphasis(&mut render, line_idx, &chars, inline_skip, tab_width);
                if let Some(style) = inline_code_background {
                    paint_inline_code(&mut render, line_idx, line_start, &chars, style, tab_width);
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

            // Apply right-to-left so earlier ranges stay in their original
            // columns. Painting happened first, so icons/labels shift with
            // the concealed text and retain their styling.
            let mut removals = line.removals.clone();
            removals.sort_unstable_by_key(|(start, _)| std::cmp::Reverse(*start));
            for (start, end) in removals {
                renderer.shift_line_left(pos.visual_line, start, end);
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

/// Detect pipe-table blocks. Only lines inside `from..len_lines` are
/// considered; callers filter to the visible range. `from` should be a few
/// lines above the viewport so bodies whose header scrolled off are still
/// classified.
fn detect_tables(
    text: helix_core::RopeSlice,
    from: usize,
    len_lines: usize,
) -> HashMap<usize, TableKind> {
    let mut kinds = HashMap::new();
    let mut index = from;
    while index < len_lines {
        let line = text.line(index).to_string();
        if is_table_row(&line) && index + 1 < len_lines {
            let next = text.line(index + 1).to_string();
            if is_delimiter_row(&next) {
                kinds.insert(index, TableKind::Header);
                kinds.insert(index + 1, TableKind::Delimiter);
                let mut body = index + 2;
                while body < len_lines && is_table_row(&text.line(body).to_string()) {
                    kinds.insert(body, TableKind::Body);
                    body += 1;
                }
                index = body;
                continue;
            }
        }
        index += 1;
    }
    kinds
}

fn is_table_row(line: &str) -> bool {
    line.contains('|') && !line.trim().is_empty()
}

fn is_delimiter_row(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains('|')
        && trimmed.contains('-')
        && trimmed
            .chars()
            .all(|ch| matches!(ch, '|' | '-' | ':' | ' ' | '\t'))
}

/// Split a pipe-table row into `(trimmed content, raw display width)` for
/// each cell. The raw width includes the spaces between the pipes so column
/// widths can start from the delimiter row's formatting.
fn table_cells(line: &str) -> Vec<(String, usize)> {
    let line = line.trim_end_matches(['\r', '\n']);
    let chars: Vec<char> = line.chars().collect();
    let Some(first_pipe) = chars.iter().position(|ch| *ch == '|') else {
        return Vec::new();
    };
    let leading_pipe = chars[..first_pipe].iter().all(|ch| ch.is_whitespace());
    let mut cells = Vec::new();
    let mut start = if leading_pipe { first_pipe + 1 } else { 0 };
    let mut index = start;
    while index <= chars.len() {
        let at_pipe = index < chars.len() && chars[index] == '|';
        if at_pipe || index == chars.len() {
            let raw = &chars[start..index];
            let only_space = raw.iter().all(|ch| ch.is_whitespace());
            // Ignore the empty cell produced by a trailing pipe.
            if !only_space || (!leading_pipe && start == 0 && !raw.is_empty()) {
                let content: String = raw.iter().collect::<String>().trim().to_string();
                let width = raw.iter().map(|ch| ch.width().unwrap_or(0)).sum::<usize>();
                cells.push((content, width));
            }
            start = index + 1;
        }
        index += 1;
    }
    cells
}

fn parse_table_layout(
    text: helix_core::RopeSlice,
    header: usize,
    available_width: usize,
) -> Option<TableLayout> {
    if header + 1 >= text.len_lines() {
        return None;
    }
    let header_line = text.line(header).to_string();
    let delimiter_line = text.line(header + 1).to_string();
    if !is_delimiter_row(&delimiter_line) {
        return None;
    }
    let delimiter_cells = table_cells(&delimiter_line);
    if delimiter_cells.is_empty() {
        return None;
    }
    let mut columns: Vec<TableColumn> = delimiter_cells
        .iter()
        .map(|(content, raw_width)| {
            let alignment = if content.starts_with(':') && content.ends_with(':') {
                TableAlignment::Center
            } else if content.starts_with(':') {
                TableAlignment::Left
            } else if content.ends_with(':') {
                TableAlignment::Right
            } else {
                TableAlignment::Default
            };
            let content_width = (*raw_width).saturating_sub(2).max(content.width()).max(1);
            TableColumn {
                width: content_width + 2,
                content_width,
                padding_left: 1,
                padding_right: 1,
                alignment,
            }
        })
        .collect();

    let header_cells = table_cells(&header_line);
    for (index, (content, _)) in header_cells.iter().enumerate() {
        if let Some(column) = columns.get_mut(index) {
            column.content_width = column.content_width.max(content.width());
        }
    }

    let mut end = header + 2;
    while end < text.len_lines() {
        let line = text.line(end).to_string();
        if !is_table_row(&line) {
            break;
        }
        for (index, (content, _)) in table_cells(&line).iter().enumerate() {
            if let Some(column) = columns.get_mut(index) {
                column.content_width = column.content_width.max(content.width());
            }
        }
        end += 1;
    }

    let indent = header_line.chars().take_while(|ch| *ch == ' ').count();
    for column in &mut columns {
        column.width = column.content_width + column.padding_left + column.padding_right;
    }
    fit_table_columns(&mut columns, available_width, indent);
    Some(TableLayout {
        columns,
        indent,
        header,
        end,
    })
}

/// Shrink column padding until the table fits the text viewport. Wrapping a
/// cell would require extra visual lines, so prefer a tighter table over
/// clipping the rightmost columns.
fn fit_table_columns(columns: &mut [TableColumn], available_width: usize, indent: usize) {
    let separators = columns.len() + 1;
    let field_space = available_width.saturating_sub(indent + separators);
    let mut total: usize = columns.iter().map(|column| column.width).sum();
    while total > field_space {
        let Some(column) = columns
            .iter_mut()
            .filter(|column| column.padding_right > 0)
            .max_by_key(|column| column.width)
        else {
            break;
        };
        column.padding_right = 0;
        column.width -= 1;
        total -= 1;
    }
    while total > field_space {
        let Some(column) = columns
            .iter_mut()
            .filter(|column| column.padding_left > 0)
            .max_by_key(|column| column.width)
        else {
            break;
        };
        column.padding_left = 0;
        column.width -= 1;
        total -= 1;
    }
    // Content wider than the viewport can only be clipped, but keep the
    // column widths at least as wide as their content.
    for column in columns {
        column.width = column
            .width
            .max(column.content_width + column.padding_left + column.padding_right);
    }
}

fn format_table_separator(columns: &[TableColumn]) -> String {
    let mut out = String::from("├");
    for (index, column) in columns.iter().enumerate() {
        let mut part: Vec<char> = std::iter::repeat_n('─', column.width).collect();
        // render-markdown highlights explicit column alignment with a thick
        // line character at the aligned edge (or both edges for center).
        if column.width >= 3 {
            match column.alignment {
                TableAlignment::Left => part[0] = '━',
                TableAlignment::Right => part[column.width - 1] = '━',
                TableAlignment::Center => {
                    part[0] = '━';
                    part[column.width - 1] = '━';
                }
                TableAlignment::Default => {}
            }
        }
        out.extend(part);
        out.push(if index + 1 == columns.len() {
            '┤'
        } else {
            '┼'
        });
    }
    out
}

fn format_table_border(columns: &[TableColumn], left: char, cross: char, right: char) -> String {
    let mut out = String::new();
    out.push(left);
    for (index, column) in columns.iter().enumerate() {
        out.push_str(&"─".repeat(column.width));
        if index + 1 < columns.len() {
            out.push(cross);
        }
    }
    out.push(right);
    out
}

fn paint_table_row(
    line_render: &mut LineRender,
    indent: usize,
    cells: &[(String, usize)],
    columns: &[TableColumn],
    text_style: Style,
    border_style: Style,
) {
    let mut col = indent;
    line_render.paints.push(Paint {
        col,
        text: "│".to_string(),
        style: border_style,
    });
    col += 1;
    for (index, column) in columns.iter().enumerate() {
        let content = cells
            .get(index)
            .map(|(content, _)| content.as_str())
            .unwrap_or("");
        let content_width = content.width();
        let inner = column
            .width
            .saturating_sub(column.padding_left + column.padding_right);
        let pad = inner.saturating_sub(content_width);
        let (align_left, align_right) = match column.alignment {
            TableAlignment::Right => (pad, 0),
            TableAlignment::Center => (pad / 2, pad - pad / 2),
            TableAlignment::Default | TableAlignment::Left => (0, pad),
        };
        let mut field = String::with_capacity(column.width);
        field.push_str(&" ".repeat(column.padding_left + align_left));
        field.push_str(content);
        field.push_str(&" ".repeat(align_right + column.padding_right));
        line_render.paints.push(Paint {
            col,
            text: field,
            style: text_style,
        });
        col += column.width;
        line_render.paints.push(Paint {
            col,
            text: "│".to_string(),
            style: border_style,
        });
        col += 1;
    }
}

/// Parse an Obsidian callout marker at `start` (e.g. `[!NOTE]` with an
/// optional `-`/`+` fold suffix). Returns the end column, the rendered title
/// text, and the theme scope for the callout.
fn parse_callout(chars: &[char], start: usize) -> Option<(usize, String, &'static str)> {
    if chars.get(start) != Some(&'[') || chars.get(start + 1) != Some(&'!') {
        return None;
    }
    let close = (start + 2..chars.len()).find(|idx| chars[*idx] == ']')?;
    let mut name: String = chars[start + 2..close].iter().collect();
    let mut end = close + 1;
    // Foldable callouts: `[!NOTE]-` / `[!NOTE]+`.
    if chars.get(end) == Some(&'-') || chars.get(end) == Some(&'+') {
        end += 1;
    }
    name = name.to_uppercase();
    let (rendered, scope) = match name.as_str() {
        "NOTE" | "ABSTRACT" | "SUMMARY" | "TLDR" | "INFO" | "TODO" => {
            let title = capitalize(&name);
            (format!("󰋽 {title}"), "diagnostic.info")
        }
        "TIP" | "HINT" | "SUCCESS" | "CHECK" | "DONE" | "IMPORTANT" | "EXAMPLE" => {
            let title = capitalize(&name);
            let icon = match name.as_str() {
                "TIP" | "HINT" => "󰌶",
                "IMPORTANT" => "󰅾",
                "EXAMPLE" => "󰉹",
                _ => "󰄬",
            };
            (format!("{icon} {title}"), "diagnostic.hint")
        }
        "QUESTION" | "HELP" | "FAQ" | "ATTENTION" | "WARNING" | "CAUTION" => {
            let title = capitalize(&name);
            (format!("󰀪 {title}"), "diagnostic.warning")
        }
        "FAILURE" | "FAIL" | "MISSING" | "DANGER" | "ERROR" | "BUG" => {
            let title = capitalize(&name);
            let icon = match name.as_str() {
                "DANGER" | "ERROR" => "󱐌",
                "BUG" => "󰨰",
                _ => "󰅖",
            };
            (format!("{icon} {title}"), "diagnostic.error")
        }
        "QUOTE" | "CITE" => {
            let title = capitalize(&name);
            (format!("󱆨 {title}"), "markup.quote")
        }
        _ => return None,
    };
    Some((end, rendered, scope))
}

fn capitalize(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => format!("{first}{}", chars.as_str().to_lowercase()),
        None => String::new(),
    }
}

/// Map visible callout continuation lines to the callout theme scope. A
/// callout block starts at a `> [!NAME]` title; following consecutive
/// blockquote lines (including bare `>` lines) belong to the block until a
/// blank or non-quote line ends it.
#[allow(dead_code)]
fn detect_callout_bodies(
    text: helix_core::RopeSlice,
    from: usize,
    len_lines: usize,
) -> HashMap<usize, &'static str> {
    let mut bodies = HashMap::new();
    let mut active: Option<&'static str> = None;
    let mut index = from;
    while index < len_lines {
        let raw = text.line(index).to_string();
        let line = raw.trim_end_matches(['\r', '\n']);
        let chars: Vec<char> = line.chars().collect();
        let indent = chars.iter().take_while(|ch| **ch == ' ').count();
        if indent < chars.len() && chars[indent] == '>' {
            let mut cursor = indent + 1;
            if chars.get(cursor) == Some(&' ') {
                cursor += 1;
            }
            if let Some((_, _, style_name)) = parse_callout(&chars, cursor) {
                active = Some(style_name);
            } else if let Some(name) = active {
                bodies.insert(index, name);
            }
        } else if indent >= chars.len() {
            // Blank line ends the callout block.
            active = None;
        } else {
            active = None;
        }
        index += 1;
    }
    bodies
}

/// Per-destination link icons, mirroring render-markdown's `link.custom`
/// table. User-configured patterns are checked first (case-insensitive
/// substring match), then the built-in host table; `None` keeps the
/// generic hyperlink icon.
fn domain_icon<'a>(dest: &str, custom: &'a [MarkdownLinkIcon]) -> Option<&'a str> {
    let lower = dest.to_lowercase();
    for custom in custom {
        if !custom.pattern.is_empty() && lower.contains(&custom.pattern.to_lowercase()) {
            return Some(custom.icon.as_str());
        }
    }
    // Specific hosts first; generic `http` match last.
    const DOMAINS: &[(&str, &str)] = &[
        ("github.com", "󰊤"),
        ("gitlab.com", "󰮠"),
        ("google.", "󰊭"),
        ("microsoft.", ""),
        ("apple.", ""),
        ("discord.", "󰙯"),
        ("slack.", "󰒱"),
        ("reddit.", "󰑍"),
        ("stackoverflow.", "󰓌"),
        ("steampowered.", ""),
        ("neovim.io", ""),
        ("ycombinator.", ""),
        ("linkedin.", "󰌻"),
    ];
    for (host, icon) in DOMAINS {
        if lower.contains(host) {
            return Some(icon);
        }
    }
    None
}

/// Paint inline link markers with icons while concealing destinations. All
/// replacements preserve width so text never shifts:
///
/// - `[[target|alias]]` → `󱗖 alias` (target concealed)
/// - `[[target]]` → `󱗖 target` (brackets concealed)
/// - `[label](dest)` → `󰌹 label` (destination concealed)
/// - `![alt](dest)` / `![[file]]` → image icon variants
/// - `<url>` / `<mail>` → icon over `<`, space over `>`
/// - `[^footnote]` → `󰯔 footnote`
/// Conceal emphasis/strong/strikethrough/highlight markers without changing
/// the line width. Syntax highlighting still styles the content itself.
fn paint_inline_emphasis(
    render: &mut MarkdownRender,
    line_idx: usize,
    chars: &[char],
    skip_until: usize,
    tab_width: usize,
) {
    let code_ranges = inline_code_ranges(chars);
    let in_code = |index: usize| {
        code_ranges
            .iter()
            .any(|(start, _, _, end)| index >= *start && index < *end)
    };
    // Closing delimiters stay in the scan range so nested emphasis can be
    // processed before the outer close, but they must not be treated as the
    // opening delimiter of a later span.
    let mut consumed = vec![false; chars.len()];
    let mut index = skip_until;
    while index < chars.len() {
        if consumed[index] || in_code(index) {
            index += 1;
            continue;
        }
        let marker = chars[index];
        if !matches!(marker, '*' | '_' | '~' | '=') {
            index += 1;
            continue;
        }
        let run = chars[index..]
            .iter()
            .take_while(|ch| **ch == marker)
            .count();
        let marker_len = if marker == '~' || marker == '=' {
            if run < 2 {
                index += 1;
                continue;
            }
            2
        } else if run >= 3 {
            3
        } else if run >= 2 {
            2
        } else {
            if chars.get(index + 1).is_none_or(|ch| ch.is_whitespace()) {
                index += 1;
                continue;
            }
            1
        };
        // Avoid treating snake_case underscores as emphasis.
        let prev_is_word = index > 0 && chars[index - 1].is_alphanumeric();
        if marker == '_' && prev_is_word {
            index += 1;
            continue;
        }

        let mut close = index + marker_len;
        let mut found = None;
        while close < chars.len() {
            if consumed[close] || in_code(close) {
                close += 1;
                continue;
            }
            if chars[close] == marker {
                let close_run = chars[close..]
                    .iter()
                    .take_while(|ch| **ch == marker)
                    .count();
                if close_run >= marker_len
                    && !chars
                        .get(close + marker_len)
                        .is_some_and(|ch| ch.is_alphanumeric() && marker == '_')
                {
                    found = Some(close);
                    break;
                }
                close += close_run.max(1);
            } else {
                close += 1;
            }
        }
        let Some(close) = found else {
            index += 1;
            continue;
        };
        let line_render = render.lines.entry(line_idx).or_default();
        let open_col = display_col(chars, index, tab_width);
        let close_col = display_col(chars, close, tab_width);
        line_render.removals.push((open_col, open_col + marker_len));
        line_render
            .removals
            .push((close_col, close_col + marker_len));
        for consumed_marker in &mut consumed[close..close + marker_len] {
            *consumed_marker = true;
        }
        // Continue inside the span so nested delimiters are concealed too.
        index += marker_len;
    }
}

fn paint_inline_links(
    render: &mut MarkdownRender,
    line_idx: usize,
    chars: &[char],
    skip_until: usize,
    icon_style: Style,
    custom_icons: &[MarkdownLinkIcon],
    tab_width: usize,
) {
    let code_ranges = inline_code_ranges(chars);
    let in_code = |index: usize| {
        code_ranges
            .iter()
            .any(|(start, _, _, end)| index >= *start && index < *end)
    };
    let mut index = skip_until;
    while index < chars.len() {
        if in_code(index) {
            index += 1;
            continue;
        }
        // Image wiki link: `![[file]]`.
        if chars[index] == '!'
            && chars.get(index + 1) == Some(&'[')
            && chars.get(index + 2) == Some(&'[')
        {
            if let Some(end) = find_closing(chars, index + 3, "]]") {
                let line_render = render.lines.entry(line_idx).or_default();
                line_render.paints.push(Paint {
                    col: display_col(&chars, index, tab_width),
                    text: "󰥶 ".to_string(),
                    style: icon_style,
                });
                line_render.paints.push(Paint {
                    col: display_col(&chars, index + 2, tab_width),
                    text: " ".to_string(),
                    style: Style::default(),
                });
                line_render.paints.push(Paint {
                    col: display_col(&chars, end, tab_width),
                    text: "  ".to_string(),
                    style: Style::default(),
                });
                index = end + 2;
                continue;
            }
            index += 1;
            continue;
        }
        // Image link: `![alt](dest)` → `󰥶 alt`.
        if chars[index] == '!' && chars.get(index + 1) == Some(&'[') {
            if let Some(label_end) = find_char(chars, index + 2, ']') {
                if chars.get(label_end + 1) == Some(&'(') {
                    if let Some(dest_end) = find_char(chars, label_end + 2, ')') {
                        let alt: String = chars[index + 2..label_end].iter().collect();
                        let line_render = render.lines.entry(line_idx).or_default();
                        // The replacement covers `![alt]` exactly; the
                        // destination is removed afterwards.
                        line_render.paints.push(Paint {
                            col: display_col(&chars, index, tab_width),
                            text: format!("󰥶 {alt} "),
                            style: plain(icon_style),
                        });
                        line_render.removals.push((
                            display_col(&chars, label_end + 1, tab_width),
                            display_col(&chars, dest_end + 1, tab_width),
                        ));
                        index = dest_end + 1;
                        continue;
                    }
                }
            }
            index += 1;
            continue;
        }
        // Wiki link: `[[target|alias]]` or `[[target]]`.
        if chars[index] == '[' && chars.get(index + 1) == Some(&'[') {
            if let Some(end) = find_closing(chars, index + 2, "]]") {
                let inner: String = chars[index + 2..end].iter().collect();
                let (target, alias) = match inner.split_once('|') {
                    Some((target, alias)) => (target, Some(alias)),
                    None => (inner.as_str(), None),
                };
                let display = alias.unwrap_or(target);
                let region_width =
                    display_col(&chars, end + 2, tab_width) - display_col(&chars, index, tab_width);
                let mut text = format!("󱗖 {display}");
                // Cover the whole `[[...]]` region so no stale brackets or
                // concealed target remain.
                while text.width() < region_width {
                    text.push(' ');
                }
                render
                    .lines
                    .entry(line_idx)
                    .or_default()
                    .paints
                    .push(Paint {
                        col: display_col(&chars, index, tab_width),
                        text,
                        style: plain(icon_style),
                    });
                index = end + 2;
                continue;
            }
            index += 1;
            continue;
        }
        // Inline link `[label](dest)` or footnote `[^x]`.
        if chars[index] == '[' {
            if chars.get(index + 1) == Some(&'^') {
                if let Some(end) = find_char(chars, index + 2, ']') {
                    let label: String = chars[index + 2..end].iter().collect();
                    render
                        .lines
                        .entry(line_idx)
                        .or_default()
                        .paints
                        .push(Paint {
                            col: display_col(&chars, index, tab_width),
                            text: format!("󰯔 {label} "),
                            style: plain(icon_style),
                        });
                    index = end + 1;
                    continue;
                }
                index += 1;
                continue;
            }
            if let Some(label_end) = find_char(chars, index + 1, ']') {
                if chars.get(label_end + 1) == Some(&'(') {
                    if let Some(dest_end) = find_char(chars, label_end + 2, ')') {
                        let dest: String = chars[label_end + 2..dest_end].iter().collect();
                        let label: String = chars[index + 1..label_end].iter().collect();
                        let icon = domain_icon(&dest, custom_icons).unwrap_or("󰌹");
                        let line_render = render.lines.entry(line_idx).or_default();
                        // `[label]` and `󰌹 label` are the same width. Paint
                        // the icon before the label (matching render-markdown)
                        // and remove `(destination)` as a single range.
                        line_render.paints.push(Paint {
                            col: display_col(&chars, index, tab_width),
                            text: format!("{icon} {label}"),
                            style: plain(icon_style),
                        });
                        line_render.removals.push((
                            display_col(&chars, label_end + 1, tab_width),
                            display_col(&chars, dest_end + 1, tab_width),
                        ));
                        index = dest_end + 1;
                        continue;
                    }
                }
            }
            index += 1;
            continue;
        }
        // Autolink `<https://…>` or `<mail@…>`.
        if chars[index] == '<' {
            if let Some(end) = find_char(chars, index + 1, '>') {
                let inner: String = chars[index + 1..end].iter().collect();
                let icon = if inner.contains('@') {
                    "󰀓"
                } else {
                    domain_icon(&inner, custom_icons).unwrap_or("󰌹")
                };
                let line_render = render.lines.entry(line_idx).or_default();
                line_render.paints.push(Paint {
                    col: display_col(&chars, index, tab_width),
                    text: icon.to_string(),
                    style: icon_style,
                });
                line_render.paints.push(Paint {
                    col: display_col(&chars, end, tab_width),
                    text: " ".to_string(),
                    style: Style::default(),
                });
                index = end + 1;
                continue;
            }
            index += 1;
            continue;
        }
        index += 1;
    }
}

/// Convert a char index to a terminal display column. Paint positions are
/// compared against `TextRenderer.offset.col`, which is in cells, so using
/// raw char indices shifts icons on lines containing CJK/emoji/combining
/// characters. Tabs advance to the next tab stop and a ZWJ sequence counts
/// as a single double-width grapheme.
fn display_col(chars: &[char], index: usize, tab_width: usize) -> usize {
    use unicode_segmentation::UnicodeSegmentation;

    let end = index.min(chars.len());
    let text: String = chars[..end].iter().collect();
    let tab_width = tab_width.max(1);
    let mut col = 0;
    for grapheme in text.graphemes(true) {
        if grapheme == "\t" {
            col += tab_width - (col % tab_width);
        } else if grapheme.contains('\u{200d}') {
            col += 2;
        } else {
            col += grapheme_width(grapheme);
        }
    }
    col
}

fn find_char(chars: &[char], from: usize, target: char) -> Option<usize> {
    (from..chars.len()).find(|idx| chars[*idx] == target)
}

fn find_closing(chars: &[char], from: usize, closing: &str) -> Option<usize> {
    let closing: Vec<char> = closing.chars().collect();
    (from..chars.len()).find(|idx| {
        chars[*idx..]
            .iter()
            .zip(closing.iter())
            .all(|(a, b)| a == b)
            && chars.len() - idx >= closing.len()
    })
}

/// Paint over inline-code backticks and record content ranges for background.
fn paint_inline_code(
    render: &mut MarkdownRender,
    line_idx: usize,
    line_start: usize,
    chars: &[char],
    style: Style,
    tab_width: usize,
) {
    for (open_start, open_end, close_start, close_end) in inline_code_ranges(chars) {
        let line_render = render.lines.entry(line_idx).or_default();
        line_render.paints.push(Paint {
            col: display_col(&chars, open_start, tab_width),
            text: " ".repeat(open_end - open_start),
            style,
        });
        line_render.paints.push(Paint {
            col: display_col(&chars, close_start, tab_width),
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

    #[test]
    fn detects_callouts() {
        let line: Vec<char> = "[!NOTE] Title".chars().collect();
        let (end, rendered, scope) = parse_callout(&line, 0).unwrap();
        assert_eq!(end, 7);
        assert_eq!(rendered, "󰋽 Note");
        assert_eq!(scope, "diagnostic.info");

        let line: Vec<char> = "[!warning]- Foldable".chars().collect();
        let (end, rendered, scope) = parse_callout(&line, 0).unwrap();
        assert_eq!(end, 11);
        assert_eq!(rendered, "󰀪 Warning");
        assert_eq!(scope, "diagnostic.warning");

        let line: Vec<char> = "[!bogus] x".chars().collect();
        assert!(parse_callout(&line, 0).is_none());

        let line: Vec<char> = "plain".chars().collect();
        assert!(parse_callout(&line, 0).is_none());
    }

    #[test]
    fn detects_table_rows() {
        assert!(is_table_row("| a | b |"));
        assert!(is_table_row("a | b"));
        assert!(!is_table_row("plain"));
        assert!(!is_table_row(""));
        assert!(is_delimiter_row("| --- | :---: | ---: |"));
        assert!(is_delimiter_row("|---|---|"));
        assert!(!is_delimiter_row("| a | b |"));
        assert!(!is_delimiter_row("plain"));
    }

    #[test]
    fn detects_table_blocks() {
        use helix_core::Rope;
        let text = Rope::from("| a | b |\n| --- | --- |\n| c | d |\n\nplain\n");
        let kinds = detect_tables(text.slice(..), 0, text.len_lines());
        assert_eq!(kinds.get(&0), Some(&TableKind::Header));
        assert_eq!(kinds.get(&1), Some(&TableKind::Delimiter));
        assert_eq!(kinds.get(&2), Some(&TableKind::Body));
        assert_eq!(kinds.get(&4), None);
    }

    #[test]
    fn table_layout_uses_content_widths() {
        use helix_core::Rope;
        let text = Rope::from(
            "| Features | TCP | UDP |\n| --- | --- | --- |\n| Connection | Requires an established connection | Connectionless |\n",
        );
        let layout = parse_table_layout(text.slice(..), 0, 200).unwrap();
        assert_eq!(layout.columns.len(), 3);
        assert_eq!(layout.columns[0].width, 12);
        assert_eq!(layout.columns[1].width, 36);
        assert_eq!(layout.columns[2].width, 16);
        assert_eq!(layout.end, 3);
        let separator = format_table_separator(&layout.columns);
        assert_eq!(separator.width(), 12 + 36 + 16 + 4);
        assert!(separator.starts_with('├'));
        assert!(separator.ends_with('┤'));

        // Narrower viewports lose trailing padding before any cell content.
        let layout = parse_table_layout(text.slice(..), 0, 66).unwrap();
        assert_eq!(layout.columns[0].width, 12);
        assert_eq!(layout.columns[1].width, 35);
        assert_eq!(layout.columns[2].width, 15);
        assert_eq!(layout.columns[1].padding_right, 0);
        assert_eq!(layout.columns[2].padding_right, 0);
    }

    #[test]
    fn detects_domain_icons() {
        let empty: &[MarkdownLinkIcon] = &[];
        assert_eq!(
            domain_icon("https://github.com/x", empty),
            Some("\u{F02A4}")
        );
        assert_eq!(
            domain_icon("https://www.google.com/search", empty),
            Some("\u{F02AD}")
        );
        assert_eq!(domain_icon("https://example.com", empty), None);
        assert_eq!(
            domain_icon("https://neovim.io/doc", empty),
            Some("\u{e6ae}")
        );
        assert_eq!(domain_icon("notes/other.md", empty), None);
    }

    #[test]
    fn custom_icons_take_precedence() {
        let custom = [MarkdownLinkIcon {
            pattern: "intranet".to_string(),
            icon: "X".to_string(),
        }];
        assert_eq!(domain_icon("https://intranet.corp/x", &custom), Some("X"));
        // Built-ins still apply when no custom pattern matches.
        assert_eq!(
            domain_icon("https://github.com/x", &custom),
            Some("\u{F02A4}")
        );
    }

    #[test]
    fn paints_domain_icon_for_links() {
        let style = Style::default();
        let mut render = MarkdownRender::default();
        let line: Vec<char> = "see [code](https://github.com/x/y) end".chars().collect();
        paint_inline_links(&mut render, 0, &line, 0, style, &[], 4);
        let paints = &render.lines.get(&0).unwrap().paints;
        assert!(paints.iter().any(|p| p.text.starts_with("󰊤")));
    }

    #[test]
    fn detects_callout_bodies() {
        use helix_core::Rope;
        let text = Rope::from("> [!NOTE] Title\n> body one\n> body two\n\nplain\n");
        let bodies = detect_callout_bodies(text.slice(..), 0, text.len_lines());
        assert_eq!(bodies.get(&1), Some(&"diagnostic.info"));
        assert_eq!(bodies.get(&2), Some(&"diagnostic.info"));
        assert_eq!(bodies.get(&0), None);
        assert_eq!(bodies.get(&4), None);
    }

    #[test]
    fn prefix_cache_tracks_fences_lists_and_callouts() {
        use helix_core::Rope;
        let text = Rope::from(
            "```rust\nfn main() {}\n```\n\n- [ ] task\n  - nested\n\n> [!NOTE] Title\n> body\n",
        );
        let meta = build_line_meta(text.slice(..));
        assert!(meta[0].in_code.is_none());
        assert!(meta[1].in_code.is_some());
        assert!(meta[2].in_code.is_some());
        assert!(meta[3].in_code.is_none());
        assert_eq!(meta[5].list_level, Some(2));
        assert_eq!(meta[8].callout, Some("diagnostic.info"));
    }

    #[test]
    fn incremental_line_meta_matches_a_fresh_build() {
        use helix_core::Rope;
        fn cache_for(text: &Rope) -> MarkdownCache {
            let mut state = LineState::default();
            let (lines, states) =
                build_line_meta_range(text.slice(..), 0, text.len_lines(), &mut state);
            MarkdownCache {
                version: 0,
                text: text.clone(),
                lines,
                states,
            }
        }

        let initial = Rope::from(
            "# Title\n\n- [ ] one\n  - two\n\n```rust\nfn main() {}\n```\n\n> [!NOTE] note\n> body\n\nplain\n",
        );
        let mut cache = cache_for(&initial);
        for updated in [
            // Edit inside a list item.
            Rope::from(
                "# Title\n\n- [x] one changed\n  - two\n\n```rust\nfn main() {}\n```\n\n> [!NOTE] note\n> body\n\nplain\n",
            ),
            // Insert a line before the fence.
            Rope::from(
                "# Title\n\nnew paragraph\n\n- [x] one changed\n  - two\n\n```rust\nfn main() {}\n```\n\n> [!NOTE] note\n> body\n\nplain\n",
            ),
            // Delete the list item and the callout body.
            Rope::from("# Title\n\n```rust\nfn main() {}\n```\n\n> [!NOTE] note\n\nplain\n"),
            // Unclosed fence extends to the end.
            Rope::from("# Title\n\n```rust\nfn main() {}\nstill code\n"),
        ] {
            update_line_meta(&mut cache, updated.slice(..));
            assert_eq!(cache.lines, build_line_meta(updated.slice(..)));
            assert_eq!(cache.lines.len(), updated.len_lines());
            assert_eq!(cache.states.len(), updated.len_lines());
        }
    }

    #[test]
    fn paint_columns_use_display_cells() {
        let line: Vec<char> = "日 [x](y)".chars().collect();
        let mut render = MarkdownRender::default();
        paint_inline_links(&mut render, 0, &line, 0, Style::default(), &[], 4);
        let paints = &render.lines.get(&0).unwrap().paints;
        // `[` starts after `日 ` (3 cells) and the icon replaces it.
        assert!(paints
            .iter()
            .any(|p| p.col == 3 && p.text.starts_with('\u{F0339}')));
    }

    #[test]
    fn display_columns_expand_tabs_and_join_emoji() {
        let text: Vec<char> = "a\tb".chars().collect();
        assert_eq!(display_col(&text, 3, 4), 5);
        assert_eq!(display_col(&text, 3, 8), 9);

        // Family emoji: a single ZWJ grapheme is two cells, not eight.
        let emoji: Vec<char> = "👨\u{200d}👩\u{200d}👧 x".chars().collect();
        assert_eq!(display_col(&emoji[..5], 5, 4), 2);
        assert_eq!(display_col(&emoji, emoji.len(), 4), 4);
    }

    #[test]
    fn emphasis_markers_are_concealed() {
        let line: Vec<char> = "**bold** and *italic* and ~~gone~~".chars().collect();
        let mut render = MarkdownRender::default();
        paint_inline_emphasis(&mut render, 0, &line, 0, 4);
        let removals = &render.lines.get(&0).unwrap().removals;
        assert!(removals.contains(&(0, 2)));
        assert!(removals.contains(&(6, 8)));
        assert!(removals.contains(&(13, 14)));
        assert!(removals.contains(&(20, 21)));
        assert!(removals.contains(&(26, 28)));
        assert!(removals.contains(&(32, 34)));
    }

    #[test]
    fn nested_emphasis_delimiters_are_concealed() {
        let line: Vec<char> = "**_x_** and **a** **b**".chars().collect();
        let mut render = MarkdownRender::default();
        paint_inline_emphasis(&mut render, 0, &line, 0, 4);
        let removals = &render.lines.get(&0).unwrap().removals;
        // Outer strong markers and the inner italic markers.
        assert!(removals.contains(&(0, 2)));
        assert!(removals.contains(&(5, 7)));
        assert!(removals.contains(&(2, 3)));
        assert!(removals.contains(&(4, 5)));
        // Separate spans must not merge the outer close with the next open.
        assert_eq!(removals.iter().filter(|(start, _)| *start >= 8).count(), 4);
    }

    #[test]
    fn quote_list_metadata_uses_content_indent() {
        use helix_core::Rope;
        let text = Rope::from("> * item\n>   - nested\n");
        let meta = build_line_meta(text.slice(..));
        assert_eq!(meta[0].list_level, Some(1));
        assert_eq!(meta[1].list_level, Some(2));
    }

    #[test]
    fn inline_code_is_not_scanned_for_links() {
        let line: Vec<char> = "`[x](https://github.com)` [y](https://github.com)"
            .chars()
            .collect();
        let mut render = MarkdownRender::default();
        paint_inline_links(&mut render, 0, &line, 0, Style::default(), &[], 4);
        let paints = &render.lines.get(&0).unwrap().paints;
        let icons = paints
            .iter()
            .filter(|p| p.text.starts_with('\u{F02A4}'))
            .count();
        assert_eq!(
            icons, 1,
            "only the link outside the code span should render"
        );
    }

    #[test]
    fn paints_wiki_and_inline_links() {
        let style = Style::default();
        let mut render = MarkdownRender::default();
        let line: Vec<char> = "see [[Target|Alias]] and [[Plain]] end".chars().collect();
        paint_inline_links(&mut render, 0, &line, 0, style, &[], 4);
        let paints = &render.lines.get(&0).unwrap().paints;
        // [[Target|Alias]]: icon + alias over the whole wiki-link region.
        assert!(paints
            .iter()
            .any(|p| p.col == 4 && p.text.starts_with("󱗖 Alias")));
        // [[Plain]]: icon + target.
        assert!(paints
            .iter()
            .any(|p| p.col == 25 && p.text.starts_with("󱗖 Plain")));

        let mut render = MarkdownRender::default();
        let line: Vec<char> = "a [label](https://x.y) b".chars().collect();
        paint_inline_links(&mut render, 1, &line, 0, style, &[], 4);
        let paints = &render.lines.get(&1).unwrap().paints;
        assert!(paints
            .iter()
            .any(|p| p.col == 2 && p.text.starts_with("󰌹 label")));
    }

    #[test]
    fn paints_autolinks_and_footnotes() {
        let style = Style::default();
        let mut render = MarkdownRender::default();
        let line: Vec<char> = "a <https://x.y> b [^fn]".chars().collect();
        paint_inline_links(&mut render, 0, &line, 0, style, &[], 4);
        let paints = &render.lines.get(&0).unwrap().paints;
        assert!(paints.iter().any(|p| p.col == 2 && p.text == "󰌹"));
        assert!(paints
            .iter()
            .any(|p| p.col == 18 && p.text.starts_with("󰯔 fn")));
    }
}
