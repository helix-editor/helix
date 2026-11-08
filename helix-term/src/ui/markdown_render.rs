//! Native markdown rendering for the editor.
//!
//! The buffer text is never modified. A [`Decoration`] paints over the
//! rendered document to conceal markdown markers, draw heading backgrounds and
//! turn thematic breaks into full-width rules.

use std::collections::{HashMap, HashSet};

use helix_core::unicode::width::UnicodeWidthStr;
use helix_view::graphics::{Rect, Style};
use helix_view::{editor::MarkdownLinkIcon, Document, Theme, View};

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
        let link_icon_style = theme
            .try_get("markup.link.label")
            .unwrap_or_else(|| theme.get("markup.link"));
        let table_border_style = theme.get("markup.raw");
        let table_head_background =
            background_style(theme, &["ui.menu", "ui.cursorline", "ui.selection"]);

        let link_custom: Vec<MarkdownLinkIcon> = doc.config.load().markdown.link_icons.clone();
        let table_kinds = detect_tables(text, start_line.saturating_sub(8), text.len_lines());
        let callout_body_styles: HashMap<usize, Style> = detect_callout_bodies(text, 0, end_line)
            .into_iter()
            .map(|(line, name)| (line, theme.try_get(name).unwrap_or(quote_style)))
            .collect();

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

            // Pipe tables take precedence over lists/quotes: rows start with
            // `|` which no other block marker uses.
            if let Some(kind) = table_kinds.get(&line_idx) {
                if visible {
                    let line_render = render.lines.entry(line_idx).or_default();
                    line_render.table = Some(*kind);
                    for (col, ch) in chars.iter().enumerate() {
                        if *ch == '|' {
                            line_render.paints.push(Paint {
                                col,
                                text: "│".to_string(),
                                style: table_border_style,
                            });
                        } else if *kind == TableKind::Delimiter && matches!(*ch, '-' | ':') {
                            line_render.paints.push(Paint {
                                col,
                                text: if *ch == '-' {
                                    "─".to_string()
                                } else {
                                    "━".to_string()
                                },
                                style: table_border_style,
                            });
                        }
                    }
                    if *kind == TableKind::Header {
                        line_render.background = table_head_background;
                    }
                }
                if visible {
                    paint_inline_links(
                        &mut render,
                        line_idx,
                        &chars,
                        0,
                        link_icon_style,
                        &link_custom,
                    );
                    if let Some(style) = inline_code_background {
                        paint_inline_code(&mut render, line_idx, line_start, &chars, style);
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
                            col: indent,
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
                    if chars.get(cursor) == Some(&' ') {
                        cursor += 1;
                    }
                }
                if !markers.is_empty() {
                    // Obsidian callout: `> [!NOTE] Title`.
                    let callout = parse_callout(&chars, cursor);
                    // Continuation lines of a callout block reuse its
                    // highlight for the quote markers.
                    let body_style = callout_body_styles.get(&line_idx).copied();
                    if visible {
                        if let Some((name_end, rendered, style_name)) = &callout {
                            let style = theme.try_get(style_name).unwrap_or(quote_style);
                            let line_render = render.lines.entry(line_idx).or_default();
                            for col in &markers {
                                line_render.paints.push(Paint {
                                    col: *col,
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
                                col: cursor,
                                text,
                                style,
                            });
                        } else {
                            let style = body_style.unwrap_or(quote_style);
                            let line_render = render.lines.entry(line_idx).or_default();
                            for col in &markers {
                                line_render.paints.push(Paint {
                                    col: *col,
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
                        inline_skip = cursor;
                        handled_block = true;
                        if !visible {
                            continue;
                        }
                    }
                }
            }

            // Task lists: `- [ ] todo` / `- [x] done`.
            if !handled_block {
                if let Some(task) = task_list(&chars, indent) {
                    update_list_indent(&mut list_indents, indent);
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
                && matches!(chars[indent], '-' | '*' | '+')
                && chars.get(indent + 1) == Some(&' ')
            {
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
                    // Fall through to inline link scanning for item content.
                    inline_skip = indent + 2;
                }
                handled_block = true;
                if !visible {
                    continue;
                }
            }

            // Horizontal rules.
            if !handled_block {
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
                );
                if let Some(style) = inline_code_background {
                    paint_inline_code(&mut render, line_idx, line_start, &chars, style);
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
    if lower.starts_with("http") {
        return Some("󰖟");
    }
    None
}

/// Paint inline link markers with icons while concealing destinations. All
/// replacements preserve width so text never shifts:
///
/// - `[[target|alias]]` → `󱗖 alias` (target concealed)
/// - `[[target]]` → `󱗖 target` (brackets concealed)
/// - `[label](dest)` → `[label]󰌹…` (icon where the destination was)
/// - `![alt](dest)` / `![[file]]` → image icon variants
/// - `<url>` / `<mail>` → icon over `<`, space over `>`
/// - `[^footnote]` → `󰯔 footnote`
fn paint_inline_links(
    render: &mut MarkdownRender,
    line_idx: usize,
    chars: &[char],
    skip_until: usize,
    icon_style: Style,
    custom_icons: &[MarkdownLinkIcon],
) {
    let mut index = skip_until.max(0);
    while index < chars.len() {
        // Image wiki link: `![[file]]`.
        if chars[index] == '!'
            && chars.get(index + 1) == Some(&'[')
            && chars.get(index + 2) == Some(&'[')
        {
            if let Some(end) = find_closing(chars, index + 3, "]]") {
                let line_render = render.lines.entry(line_idx).or_default();
                line_render.paints.push(Paint {
                    col: index,
                    text: "󰥶 ".to_string(),
                    style: icon_style,
                });
                line_render.paints.push(Paint {
                    col: index + 2,
                    text: " ".to_string(),
                    style: Style::default(),
                });
                line_render.paints.push(Paint {
                    col: end,
                    text: "  ".to_string(),
                    style: Style::default(),
                });
                index = end + 2;
                continue;
            }
            index += 1;
            continue;
        }
        // Image link: `![alt](dest)`.
        if chars[index] == '!' && chars.get(index + 1) == Some(&'[') {
            if let Some(label_end) = find_char(chars, index + 2, ']') {
                if chars.get(label_end + 1) == Some(&'(') {
                    if let Some(dest_end) = find_char(chars, label_end + 2, ')') {
                        // Cover `(dest)` inclusive: width is end - start + 1.
                        let width = dest_end - (label_end + 1) + 1;
                        let mut text = String::from("󰥶");
                        while text.width() < width {
                            text.push(' ');
                        }
                        render
                            .lines
                            .entry(line_idx)
                            .or_default()
                            .paints
                            .push(Paint {
                                col: label_end + 1,
                                text,
                                style: icon_style,
                            });
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
                let line_render = render.lines.entry(line_idx).or_default();
                line_render.paints.push(Paint {
                    col: index,
                    text: "󱗖 ".to_string(),
                    style: icon_style,
                });
                if let Some(pipe) = inner.find('|') {
                    let target_len = pipe + 1;
                    line_render.paints.push(Paint {
                        col: index + 2,
                        text: " ".repeat(target_len),
                        style: Style::default(),
                    });
                }
                line_render.paints.push(Paint {
                    col: end,
                    text: "  ".to_string(),
                    style: Style::default(),
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
                    render
                        .lines
                        .entry(line_idx)
                        .or_default()
                        .paints
                        .push(Paint {
                            col: index,
                            text: "󰯔 ".to_string(),
                            style: icon_style,
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
                        let icon = domain_icon(&dest, custom_icons).unwrap_or("󰌹");
                        // Cover `(dest)` inclusive: width is end - start + 1.
                        let width = dest_end - (label_end + 1) + 1;
                        let mut text = String::from(icon);
                        while text.width() < width {
                            text.push(' ');
                        }
                        render
                            .lines
                            .entry(line_idx)
                            .or_default()
                            .paints
                            .push(Paint {
                                col: label_end + 1,
                                text,
                                style: icon_style,
                            });
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
                    col: index,
                    text: icon.to_string(),
                    style: icon_style,
                });
                line_render.paints.push(Paint {
                    col: end,
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
) {
    for (open_start, open_end, close_start, close_end) in inline_code_ranges(chars) {
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
        assert_eq!(domain_icon("https://example.com", empty), Some("\u{F059F}"));
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
        paint_inline_links(&mut render, 0, &line, 0, style, &[]);
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
    fn paints_wiki_and_inline_links() {
        let style = Style::default();
        let mut render = MarkdownRender::default();
        let line: Vec<char> = "see [[Target|Alias]] and [[Plain]] end".chars().collect();
        paint_inline_links(&mut render, 0, &line, 0, style, &[]);
        let paints = &render.lines.get(&0).unwrap().paints;
        // [[Target|Alias]]: icon + concealed target + concealed brackets.
        assert!(paints.iter().any(|p| p.col == 4 && p.text == "󱗖 "));
        assert!(paints.iter().any(|p| p.col == 6 && p.text == " ".repeat(7)));
        // [[Plain]]: icon at 25, concealed close at 32.
        assert!(paints.iter().any(|p| p.col == 25 && p.text == "󱗖 "));
        assert!(paints.iter().any(|p| p.col == 32 && p.text == "  "));

        let mut render = MarkdownRender::default();
        let line: Vec<char> = "a [label](https://x.y) b".chars().collect();
        paint_inline_links(&mut render, 1, &line, 0, style, &[]);
        let paints = &render.lines.get(&1).unwrap().paints;
        assert!(paints.iter().any(|p| p.col == 9 && p.text.starts_with("󰖟")));
    }

    #[test]
    fn paints_autolinks_and_footnotes() {
        let style = Style::default();
        let mut render = MarkdownRender::default();
        let line: Vec<char> = "a <https://x.y> b [^fn]".chars().collect();
        paint_inline_links(&mut render, 0, &line, 0, style, &[]);
        let paints = &render.lines.get(&0).unwrap().paints;
        assert!(paints.iter().any(|p| p.col == 2 && p.text == "󰖟"));
        assert!(paints.iter().any(|p| p.col == 18 && p.text == "󰯔 "));
    }
}
