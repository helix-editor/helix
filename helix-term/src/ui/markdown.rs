use crate::compositor::{Component, Context};
use arc_swap::ArcSwap;
use tui::{
    buffer::Buffer as Surface,
    layout::{Alignment as TuiAlignment, Constraint},
    text::{Span, Spans, Text},
    widgets::{Cell, Paragraph, Row, Table, TableState, Widget, Wrap},
};

use std::sync::Arc;

use pulldown_cmark::{
    Alignment as MarkdownAlignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag,
    TagEnd,
};

use helix_core::{
    syntax::{self, HighlightEvent, OverlayHighlights},
    RopeSlice, Syntax,
};
use helix_view::{
    graphics::{Margin, Rect, Style},
    theme::Modifier,
    Theme,
};

fn styled_multiline_text<'a>(text: &str, style: Style) -> Text<'a> {
    let spans: Vec<_> = text
        .lines()
        .map(|line| Span::styled(line.to_string(), style))
        .map(Spans::from)
        .collect();
    Text::from(spans)
}

pub fn highlighted_code_block<'a>(
    text: &str,
    language: &str,
    theme: Option<&Theme>,
    loader: &syntax::Loader,
    // Optional overlay highlights to mix in with the syntax highlights.
    //
    // Note that `OverlayHighlights` is typically used with char indexing but the only caller
    // which passes this parameter currently passes **byte indices** instead.
    additional_highlight_spans: Option<OverlayHighlights>,
) -> Text<'a> {
    let mut spans = Vec::new();
    let mut lines = Vec::new();

    let get_theme = |key: &str| -> Style { theme.map(|t| t.get(key)).unwrap_or_default() };
    let text_style = get_theme(Markdown::TEXT_STYLE);
    let code_style = get_theme(Markdown::BLOCK_STYLE);

    let theme = match theme {
        Some(t) => t,
        None => return styled_multiline_text(text, code_style),
    };

    let ropeslice = RopeSlice::from(text);
    let Some(syntax) = loader
        .language_for_match(RopeSlice::from(language))
        .and_then(|lang| Syntax::new(ropeslice, lang, loader).ok())
    else {
        return styled_multiline_text(text, code_style);
    };

    let mut syntax_highlighter = syntax.highlighter(ropeslice, loader, ..);
    let mut syntax_highlight_stack = Vec::new();
    let mut overlay_highlight_stack = Vec::new();
    let mut overlay_highlighter = syntax::OverlayHighlighter::new(additional_highlight_spans);
    let mut pos = 0;

    while pos < ropeslice.len_bytes() as u32 {
        if pos == syntax_highlighter.next_event_offset() {
            let (event, new_highlights) = syntax_highlighter.advance();
            if event == HighlightEvent::Refresh {
                syntax_highlight_stack.clear();
            }
            syntax_highlight_stack.extend(new_highlights);
        } else if pos == overlay_highlighter.next_event_offset() as u32 {
            let (event, new_highlights) = overlay_highlighter.advance();
            if event == HighlightEvent::Refresh {
                overlay_highlight_stack.clear();
            }
            overlay_highlight_stack.extend(new_highlights)
        }

        let start = pos;
        pos = syntax_highlighter
            .next_event_offset()
            .min(overlay_highlighter.next_event_offset() as u32);
        if pos == u32::MAX {
            pos = ropeslice.len_bytes() as u32;
        }
        if pos == start {
            continue;
        }
        // The highlighter should always move forward.
        // If the highlighter malfunctions, bail on syntax highlighting and log an error.
        debug_assert!(pos > start);
        if pos < start {
            log::error!("Failed to highlight '{language}': {text:?}");
            return styled_multiline_text(text, code_style);
        }

        let style = syntax_highlight_stack
            .iter()
            .chain(overlay_highlight_stack.iter())
            .fold(text_style, |acc, highlight| {
                acc.patch(theme.highlight(*highlight))
            });

        let mut slice = &text[start as usize..pos as usize];
        // TODO: do we need to handle all unicode line endings
        // here, or is just '\n' okay?
        while let Some(end) = slice.find('\n') {
            // emit span up to newline
            let text = &slice[..end];
            let text = text.replace('\t', "    "); // replace tabs
            let span = Span::styled(text, style);
            spans.push(span);

            // truncate slice to after newline
            slice = &slice[end + 1..];

            // make a new line
            let spans = std::mem::take(&mut spans);
            lines.push(Spans::from(spans));
        }

        if !slice.is_empty() {
            let span = Span::styled(slice.replace('\t', "    "), style);
            spans.push(span);
        }
    }

    if !spans.is_empty() {
        let spans = std::mem::take(&mut spans);
        lines.push(Spans::from(spans));
    }

    Text::from(lines)
}

pub struct Markdown {
    contents: String,

    config_loader: Arc<ArcSwap<syntax::Loader>>,
}

struct MarkdownTableCell<'a> {
    content: Spans<'a>,
    alignment: TuiAlignment,
}

struct MarkdownTable<'a> {
    rows: Vec<Vec<MarkdownTableCell<'a>>>,
    column_widths: Vec<u16>,
    separator_style: Style,
}

impl MarkdownTable<'_> {
    const SEPARATOR: &'static str = " │ ";
    const SEPARATOR_WIDTH: u16 = 3;

    fn width(&self) -> u16 {
        let columns = self.column_widths.len().min(u16::MAX as usize) as u16;
        let separators = columns
            .saturating_sub(1)
            .saturating_mul(Self::SEPARATOR_WIDTH);
        self.column_widths
            .iter()
            .fold(separators, |width, column| width.saturating_add(*column))
    }

    fn height(&self) -> u16 {
        self.rows.len().min(u16::MAX as usize) as u16
    }
}

enum MarkdownBlock<'a> {
    Text(Text<'a>),
    Table(MarkdownTable<'a>),
}

struct ParsedMarkdown<'a> {
    blocks: Vec<MarkdownBlock<'a>>,
}

impl<'a> ParsedMarkdown<'a> {
    fn block_size(block: &MarkdownBlock, max_width: u16, trailing_spacing: bool) -> (u16, u16) {
        match block {
            MarkdownBlock::Text(text) => Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .required_size(max_width),
            MarkdownBlock::Table(table) => (
                table.width().min(max_width),
                table.height().saturating_add(u16::from(trailing_spacing)),
            ),
        }
    }

    fn required_size(&self, max_width: u16) -> (u16, u16) {
        if max_width == 0 {
            return (0, 0);
        }

        let block_count = self.blocks.len();
        self.blocks
            .iter()
            .enumerate()
            .fold((0u16, 0u16), |(width, height), (index, block)| {
                let (block_width, block_height) =
                    Self::block_size(block, max_width, index + 1 < block_count);
                (width.max(block_width), height.saturating_add(block_height))
            })
    }

    fn render(self, area: Rect, surface: &mut Surface, mut scroll: usize) {
        if area.area() == 0 {
            return;
        }

        let block_count = self.blocks.len();
        let mut y = area.top();
        let mut remaining_height = area.height;

        for (index, block) in self.blocks.into_iter().enumerate() {
            let (_, block_height) = Self::block_size(&block, area.width, index + 1 < block_count);
            let block_height = block_height as usize;
            if scroll >= block_height {
                scroll -= block_height;
                continue;
            }

            let visible_height = (block_height - scroll)
                .min(remaining_height as usize)
                .min(u16::MAX as usize) as u16;
            if visible_height == 0 {
                break;
            }

            let block_area = Rect::new(area.left(), y, area.width, visible_height);
            match block {
                MarkdownBlock::Text(text) => {
                    Paragraph::new(&text)
                        .wrap(Wrap { trim: false })
                        .scroll((scroll.min(u16::MAX as usize) as u16, 0))
                        .render(block_area, surface);
                }
                MarkdownBlock::Table(table) => {
                    let table_height = table.height() as usize;
                    if scroll < table_height {
                        let rows_visible = (table_height - scroll).min(visible_height as usize);
                        let constraints = table
                            .column_widths
                            .iter()
                            .copied()
                            .map(Constraint::Length)
                            .collect::<Vec<_>>();
                        let rows = table.rows.into_iter().map(|row| {
                            Row::new(
                                row.into_iter()
                                    .map(|cell| Cell::from(cell.content).alignment(cell.alignment)),
                            )
                        });
                        let table =
                            Table::new(rows)
                                .widths(&constraints)
                                .column_separator(Span::styled(
                                    MarkdownTable::SEPARATOR,
                                    table.separator_style,
                                ));
                        table.render_table(
                            block_area.with_height(rows_visible as u16),
                            surface,
                            &mut TableState {
                                offset: scroll,
                                selected: None,
                            },
                            false,
                        );
                    }
                }
            }

            y = y.saturating_add(visible_height);
            remaining_height = remaining_height.saturating_sub(visible_height);
            if remaining_height == 0 {
                break;
            }
            scroll = 0;
        }
    }

    fn into_text(self) -> Text<'a> {
        let block_count = self.blocks.len();
        let mut lines = Vec::new();
        for (index, block) in self.blocks.into_iter().enumerate() {
            match block {
                MarkdownBlock::Text(text) => lines.extend(text.lines),
                MarkdownBlock::Table(table) => {
                    for row in table.rows {
                        let mut spans = Vec::new();
                        let cell_count = row.len();
                        for (cell_index, cell) in row.into_iter().enumerate() {
                            spans.extend(cell.content.0);
                            if cell_index + 1 < cell_count {
                                spans.push(Span::styled(
                                    MarkdownTable::SEPARATOR,
                                    table.separator_style,
                                ));
                            }
                        }
                        lines.push(Spans::from(spans));
                    }
                    if index + 1 < block_count {
                        lines.push(Spans::default());
                    }
                }
            }
        }
        Text::from(lines)
    }
}

// TODO: pre-render and self reference via Pin
// better yet, just use Tendril + subtendril for references

impl Markdown {
    const TEXT_STYLE: &'static str = "ui.text";
    const BLOCK_STYLE: &'static str = "markup.raw.inline";
    const RULE_STYLE: &'static str = "punctuation.special";
    const UNNUMBERED_LIST_STYLE: &'static str = "markup.list.unnumbered";
    const NUMBERED_LIST_STYLE: &'static str = "markup.list.numbered";
    const HEADING_STYLES: [&'static str; 6] = [
        "markup.heading.1",
        "markup.heading.2",
        "markup.heading.3",
        "markup.heading.4",
        "markup.heading.5",
        "markup.heading.6",
    ];
    const INDENT: &'static str = "  ";

    pub fn new(contents: String, config_loader: Arc<ArcSwap<syntax::Loader>>) -> Self {
        Self {
            contents,
            config_loader,
        }
    }

    pub fn parse(&self, theme: Option<&Theme>) -> tui::text::Text<'_> {
        self.parse_blocks(theme).into_text()
    }

    pub fn render_content(
        &self,
        area: Rect,
        surface: &mut Surface,
        theme: Option<&Theme>,
        scroll: usize,
    ) {
        self.parse_blocks(theme).render(area, surface, scroll);
    }

    pub fn required_size_for_width(&self, max_width: u16) -> (u16, u16) {
        self.parse_blocks(None).required_size(max_width)
    }

    fn parse_blocks(&self, theme: Option<&Theme>) -> ParsedMarkdown<'_> {
        fn push_line<'a>(spans: &mut Vec<Span<'a>>, lines: &mut Vec<Spans<'a>>) {
            let spans = std::mem::take(spans);
            if !spans.is_empty() {
                lines.push(Spans::from(spans));
            }
        }

        fn push_text_block<'a>(lines: &mut Vec<Spans<'a>>, blocks: &mut Vec<MarkdownBlock<'a>>) {
            if !lines.is_empty() {
                blocks.push(MarkdownBlock::Text(Text::from(std::mem::take(lines))));
            }
        }

        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        let parser = Parser::new_ext(&self.contents, options);

        // TODO: if possible, render links as terminal hyperlinks: https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda
        let mut tags = Vec::new();
        let mut spans = Vec::new();
        let mut lines = Vec::new();
        let mut blocks = Vec::new();
        let mut list_stack = Vec::new();

        let get_indent = |level: usize| {
            if level < 1 {
                String::new()
            } else {
                Self::INDENT.repeat(level - 1)
            }
        };

        let get_theme = |key: &str| -> Style { theme.map(|t| t.get(key)).unwrap_or_default() };
        let text_style = get_theme(Self::TEXT_STYLE);
        let code_style = get_theme(Self::BLOCK_STYLE);
        let numbered_list_style = get_theme(Self::NUMBERED_LIST_STYLE);
        let unnumbered_list_style = get_theme(Self::UNNUMBERED_LIST_STYLE);
        let rule_style = get_theme(Self::RULE_STYLE);
        let heading_styles: Vec<Style> = Self::HEADING_STYLES
            .iter()
            .map(|key| get_theme(key))
            .collect();

        // Transform text in `<code>` blocks into `Event::Code`
        let mut in_code = false;
        let mut parser = parser.filter_map(|event| match event {
            Event::Html(tag)
                if tag.starts_with("<code") && matches!(tag.chars().nth(5), Some(' ' | '>')) =>
            {
                in_code = true;
                None
            }
            Event::Html(tag) if *tag == *"</code>" => {
                in_code = false;
                None
            }
            Event::Text(text) if in_code => Some(Event::Code(text)),
            _ => Some(event),
        });

        while let Some(event) = parser.next() {
            match event {
                Event::Start(Tag::Table(alignments)) => {
                    push_line(&mut spans, &mut lines);
                    push_text_block(&mut lines, &mut blocks);
                    blocks.push(MarkdownBlock::Table(Self::parse_table(
                        &mut parser,
                        alignments,
                        text_style,
                        code_style,
                        rule_style,
                    )));
                }
                Event::Start(Tag::List(list)) => {
                    // if the list stack is not empty this is a sub list, in that
                    // case we need to push the current line before proceeding
                    if !list_stack.is_empty() {
                        push_line(&mut spans, &mut lines);
                    }

                    list_stack.push(list);
                }
                Event::End(TagEnd::List(_)) => {
                    list_stack.pop();

                    // whenever top-level list closes, empty line
                    if list_stack.is_empty() {
                        lines.push(Spans::default());
                    }
                }
                Event::Start(Tag::Item) => {
                    if list_stack.is_empty() {
                        log::warn!("markdown parsing error, list item without list");
                    }

                    tags.push(Tag::Item);

                    // get the appropriate bullet for the current list
                    let (bullet, bullet_style) = list_stack
                        .last()
                        .unwrap_or(&None) // use the '- ' bullet in case the list stack would be empty
                        .map_or((String::from("• "), unnumbered_list_style), |number| {
                            (format!("{}. ", number), numbered_list_style)
                        });

                    // increment the current list number if there is one
                    if let Some(v) = list_stack.last_mut().unwrap_or(&mut None).as_mut() {
                        *v += 1;
                    }

                    let prefix = get_indent(list_stack.len()) + bullet.as_str();
                    spans.push(Span::styled(prefix, bullet_style));
                }
                Event::Start(tag) => {
                    tags.push(tag);
                    if spans.is_empty() && !list_stack.is_empty() {
                        // TODO: could push indent + 2 or 3 spaces to align with
                        // the rest of the list.
                        spans.push(Span::from(get_indent(list_stack.len())));
                    }
                }
                Event::End(tag) => {
                    tags.pop();
                    match tag {
                        TagEnd::Heading(_)
                        | TagEnd::Paragraph
                        | TagEnd::CodeBlock
                        | TagEnd::Item => {
                            push_line(&mut spans, &mut lines);
                        }
                        _ => (),
                    }

                    // whenever heading, code block or paragraph closes, empty line
                    match tag {
                        TagEnd::Heading(_) | TagEnd::Paragraph | TagEnd::CodeBlock => {
                            lines.push(Spans::default());
                        }
                        _ => (),
                    }
                }
                Event::Text(text) => {
                    if let Some(Tag::CodeBlock(kind)) = tags.last() {
                        let language = match kind {
                            CodeBlockKind::Fenced(language) => language,
                            CodeBlockKind::Indented => "",
                        };
                        let tui_text = highlighted_code_block(
                            &text,
                            language,
                            theme,
                            &self.config_loader.load(),
                            None,
                        );
                        lines.extend(tui_text.lines);
                    } else {
                        let style = match tags.last() {
                            Some(Tag::Heading { level, .. }) => match level {
                                HeadingLevel::H1 => heading_styles[0],
                                HeadingLevel::H2 => heading_styles[1],
                                HeadingLevel::H3 => heading_styles[2],
                                HeadingLevel::H4 => heading_styles[3],
                                HeadingLevel::H5 => heading_styles[4],
                                HeadingLevel::H6 => heading_styles[5],
                            },
                            Some(Tag::Emphasis) => text_style.add_modifier(Modifier::ITALIC),
                            Some(Tag::Strong) => text_style.add_modifier(Modifier::BOLD),
                            Some(Tag::Strikethrough) => {
                                text_style.add_modifier(Modifier::CROSSED_OUT)
                            }
                            _ => text_style,
                        };
                        spans.push(Span::styled(text, style));
                    }
                }
                Event::Code(text) | Event::Html(text) => {
                    spans.push(Span::styled(text, code_style));
                }
                Event::SoftBreak | Event::HardBreak => {
                    push_line(&mut spans, &mut lines);
                    if !list_stack.is_empty() {
                        // TODO: could push indent + 2 or 3 spaces to align with
                        // the rest of the list.
                        spans.push(Span::from(get_indent(list_stack.len())));
                    }
                }
                Event::Rule => {
                    lines.push(Spans::from(Span::styled("───", rule_style)));
                    lines.push(Spans::default());
                }
                // TaskListMarker(bool) true if checked
                _ => {
                    log::warn!("unhandled markdown event {:?}", event);
                }
            }
        }

        if !spans.is_empty() {
            lines.push(Spans::from(spans));
        }
        push_text_block(&mut lines, &mut blocks);

        // if last text line is empty, remove it
        if let Some(MarkdownBlock::Text(text)) = blocks.last_mut() {
            if text.lines.last().is_some_and(|line| line.0.is_empty()) {
                text.lines.pop();
            }
            if text.lines.is_empty() {
                blocks.pop();
            }
        }

        ParsedMarkdown { blocks }
    }

    fn parse_table<'a>(
        events: &mut impl Iterator<Item = Event<'a>>,
        alignments: Vec<MarkdownAlignment>,
        text_style: Style,
        code_style: Style,
        separator_style: Style,
    ) -> MarkdownTable<'a> {
        let cell_alignment = |column: usize| match alignments
            .get(column)
            .copied()
            .unwrap_or(MarkdownAlignment::None)
        {
            MarkdownAlignment::None | MarkdownAlignment::Left => TuiAlignment::Left,
            MarkdownAlignment::Center => TuiAlignment::Center,
            MarkdownAlignment::Right => TuiAlignment::Right,
        };
        let mut tags = Vec::new();
        let mut current_cell = Vec::new();
        let mut current_row = Vec::new();
        let mut rows = Vec::new();
        let mut in_header = false;

        for event in events {
            match event {
                Event::Start(Tag::TableHead) => in_header = true,
                Event::End(TagEnd::TableHead) => {
                    if !current_row.is_empty() {
                        rows.push(std::mem::take(&mut current_row));
                    }
                    in_header = false;
                }
                Event::Start(Tag::TableRow | Tag::TableCell) => (),
                Event::End(TagEnd::TableRow) => {
                    if !current_row.is_empty() {
                        rows.push(std::mem::take(&mut current_row));
                    }
                }
                Event::End(TagEnd::TableCell) => {
                    current_row.push(MarkdownTableCell {
                        content: Spans::from(std::mem::take(&mut current_cell)),
                        alignment: cell_alignment(current_row.len()),
                    });
                }
                Event::End(TagEnd::Table) => break,
                Event::Text(text) => {
                    let style = match tags.last() {
                        Some(Tag::Emphasis) => text_style.add_modifier(Modifier::ITALIC),
                        Some(Tag::Strong) => text_style.add_modifier(Modifier::BOLD),
                        Some(Tag::Strikethrough) => text_style.add_modifier(Modifier::CROSSED_OUT),
                        _ => text_style,
                    };
                    let style = if in_header {
                        style.add_modifier(Modifier::BOLD)
                    } else {
                        style
                    };
                    current_cell.push(Span::styled(text, style));
                }
                Event::Code(text) | Event::Html(text) => {
                    let style = if in_header {
                        code_style.add_modifier(Modifier::BOLD)
                    } else {
                        code_style
                    };
                    current_cell.push(Span::styled(text, style));
                }
                Event::SoftBreak | Event::HardBreak => {
                    current_cell.push(Span::styled(" ", text_style));
                }
                Event::Start(tag) => tags.push(tag),
                Event::End(_) => {
                    tags.pop();
                }
                _ => {
                    log::warn!("unhandled markdown event in table {:?}", event);
                }
            }
        }

        let column_count = alignments
            .len()
            .max(rows.iter().map(Vec::len).max().unwrap_or_default());
        for row in &mut rows {
            row.extend((row.len()..column_count).map(|column| MarkdownTableCell {
                content: Spans::default(),
                alignment: cell_alignment(column),
            }));
        }
        let mut column_widths = vec![0u16; column_count];
        for row in &rows {
            for (column, cell) in row.iter().enumerate() {
                let width = cell.content.width().min(u16::MAX as usize) as u16;
                column_widths[column] = column_widths[column].max(width);
            }
        }

        MarkdownTable {
            rows,
            column_widths,
            separator_style,
        }
    }
}

impl Component for Markdown {
    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let margin = Margin::all(1);
        self.render_content(
            area.inner(margin),
            surface,
            Some(&cx.editor.theme),
            cx.scroll.unwrap_or_default(),
        );
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        let padding = 2;

        // TODO: account for tab width
        let max_text_width = (viewport.0.saturating_sub(padding)).min(120);
        let (width, height) = self.required_size_for_width(max_text_width);

        Some((width + padding, height + padding))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn markdown(contents: &str) -> Markdown {
        Markdown::new(
            contents.to_owned(),
            Arc::new(ArcSwap::from_pointee(syntax::Loader::default())),
        )
    }

    fn buffer_line(buffer: &Surface, y: u16) -> String {
        (buffer.area.left()..buffer.area.right())
            .map(|x| buffer[(x, y)].symbol.as_str())
            .collect()
    }

    #[test]
    fn parses_markdown_table_cells_and_alignment() {
        let markdown = markdown("| Left | Right |\n| :--- | ---: |\n| a | bb |");
        let parsed = markdown.parse_blocks(None);

        let [MarkdownBlock::Table(table)] = parsed.blocks.as_slice() else {
            panic!("expected one table block");
        };
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.column_widths, [4, 5]);
        assert_eq!(String::from(&table.rows[0][0].content), "Left");
        assert_eq!(String::from(&table.rows[1][1].content), "bb");
        assert_eq!(table.rows[0][0].alignment, TuiAlignment::Left);
        assert_eq!(table.rows[0][1].alignment, TuiAlignment::Right);
    }

    #[test]
    fn renders_tables_without_wrapping_rows() {
        let markdown = markdown("| Left | Right |\n| :--- | ---: |\n| a | b |\n\nafter");
        let area = Rect::new(0, 0, 12, 4);
        let mut buffer = Surface::empty(area);

        markdown.render_content(area, &mut buffer, None, 0);

        assert_eq!(buffer_line(&buffer, 0), "Left │ Right");
        assert_eq!(buffer_line(&buffer, 1), "a    │     b");
        assert_eq!(buffer_line(&buffer, 2), "            ");
        assert_eq!(buffer_line(&buffer, 3), "after       ");
        assert_eq!(markdown.required_size_for_width(area.width), (12, 4));
    }

    #[test]
    fn narrow_tables_are_truncated_instead_of_adding_visual_rows() {
        let markdown = markdown(
            "| First column | Second column |\n| --- | --- |\n| first value | second value |\n\nafter",
        );
        let area = Rect::new(0, 0, 8, 4);
        let mut buffer = Surface::empty(area);

        markdown.render_content(area, &mut buffer, None, 0);

        assert_eq!(buffer_line(&buffer, 2), "        ");
        assert_eq!(buffer_line(&buffer, 3), "after   ");
        assert_eq!(markdown.required_size_for_width(area.width).1, 4);
    }

    #[test]
    fn scrolling_across_a_table_keeps_following_blocks_aligned() {
        let markdown = markdown("| Left | Right |\n| :--- | ---: |\n| a | b |\n\nafter");
        let area = Rect::new(0, 0, 12, 3);
        let mut buffer = Surface::empty(area);

        markdown.render_content(area, &mut buffer, None, 1);

        assert_eq!(buffer_line(&buffer, 0), "a    │     b");
        assert_eq!(buffer_line(&buffer, 1), "            ");
        assert_eq!(buffer_line(&buffer, 2), "after       ");
    }
}
