use std::sync::Arc;

use arc_swap::ArcSwap;
use helix_core::syntax;
use helix_lsp::lsp;
use helix_view::graphics::{Margin, Rect, Style};
use helix_view::input::Event;
use tui::buffer::Buffer;
use tui::widgets::{BorderType, Paragraph, Widget, Wrap};

use crate::compositor::{Component, Context, EventResult};

use crate::alt;
use crate::ui::Markdown;

pub struct Hover {
    active_index: usize,
    contents: Vec<(Option<Markdown>, Markdown)>,
}

impl Hover {
    pub const ID: &'static str = "hover";

    pub fn new(
        hovers: Vec<(String, lsp::Hover)>,
        config_loader: Arc<ArcSwap<syntax::Loader>>,
    ) -> Self {
        let n_hovers = hovers.len();
        let contents = hovers
            .into_iter()
            .enumerate()
            .map(|(idx, (server_name, hover))| {
                let header = (n_hovers > 1).then(|| {
                    Markdown::new(
                        format!("**[{}/{}] {}**", idx + 1, n_hovers, server_name),
                        config_loader.clone(),
                    )
                });
                let body = Markdown::new(
                    hover_contents_to_string(hover.contents),
                    config_loader.clone(),
                );
                (header, body)
            })
            .collect();

        Self {
            active_index: usize::default(),
            contents,
        }
    }

    fn has_header(&self) -> bool {
        self.contents.len() > 1
    }

    fn content(&self) -> &(Option<Markdown>, Markdown) {
        &self.contents[self.active_index]
    }

    pub fn content_string(&self) -> String {
        self.contents
            .iter()
            .map(|(header, body)| {
                if let Some(header) = header {
                    format!("{}\n{}", header.contents.trim(), body.contents.trim())
                } else {
                    body.contents.trim().to_owned()
                }
            })
            .collect::<Vec<String>>()
            .join("\n\n---\n\n")
            + "\n"
    }

    fn set_index(&mut self, index: usize) {
        assert!((0..self.contents.len()).contains(&index));
        self.active_index = index;
    }
}

const PADDING_HORIZONTAL: u16 = 2;
const PADDING_TOP: u16 = 1;
const PADDING_BOTTOM: u16 = 1;
const HEADER_HEIGHT: u16 = 1;
const SEPARATOR_HEIGHT: u16 = 1;

impl Component for Hover {
    fn render(&mut self, area: Rect, surface: &mut Buffer, cx: &mut Context) {
        let margin = Margin::all(1);
        let area = area.inner(margin);

        let (header, contents) = self.content();

        // show header and border only when more than one results
        if let Some(header) = header {
            // header LSP Name
            let header = header.parse(Some(&cx.editor.theme));
            let header = Paragraph::new(&header);
            header.render(area.with_height(HEADER_HEIGHT), surface);

            // border
            let sep_style = Style::default();
            let borders = BorderType::line_symbols(BorderType::Plain);
            for x in area.left()..area.right() {
                if let Some(cell) = surface.get_mut(x, area.top() + HEADER_HEIGHT) {
                    cell.set_symbol(borders.horizontal).set_style(sep_style);
                }
            }
        }

        // hover content
        let contents = contents.parse(Some(&cx.editor.theme));
        let contents_area = area.clip_top(if self.has_header() {
            HEADER_HEIGHT + SEPARATOR_HEIGHT
        } else {
            0
        });
        let contents_para = Paragraph::new(&contents)
            .wrap(Wrap { trim: false })
            .scroll((cx.scroll.unwrap_or_default() as u16, 0));
        contents_para.render(contents_area, surface);
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        let max_text_width = viewport.0.saturating_sub(PADDING_HORIZONTAL).clamp(10, 120);

        let (header, contents) = self.content();

        let header_width = header
            .as_ref()
            .map(|header| {
                let header = header.parse(None);
                let (width, _height) = crate::ui::text::required_size(&header, max_text_width);
                width
            })
            .unwrap_or_default();

        let contents = contents.parse(None);
        let (content_width, content_height) =
            crate::ui::text::required_size(&contents, max_text_width);

        let width = PADDING_HORIZONTAL + header_width.max(content_width);
        let height = if self.has_header() {
            PADDING_TOP + HEADER_HEIGHT + SEPARATOR_HEIGHT + content_height + PADDING_BOTTOM
        } else {
            PADDING_TOP + content_height + PADDING_BOTTOM
        };

        Some((width, height))
    }

    fn handle_event(&mut self, event: &Event, _ctx: &mut Context) -> EventResult {
        let Event::Key(event) = event else {
            return EventResult::Ignored(None);
        };

        match event {
            alt!('p') => {
                let index = self
                    .active_index
                    .checked_sub(1)
                    .unwrap_or(self.contents.len() - 1);
                self.set_index(index);
                EventResult::Consumed(None)
            }
            alt!('n') => {
                self.set_index((self.active_index + 1) % self.contents.len());
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored(None),
        }
    }
}

fn hover_contents_to_string(contents: lsp::HoverContents) -> String {
    fn marked_string_to_markdown(contents: lsp::MarkedString) -> String {
        match contents {
            lsp::MarkedString::String(contents) => contents,
            lsp::MarkedString::LanguageString(string) => {
                if string.language == "markdown" {
                    string.value
                } else {
                    format!("```{}\n{}\n```", string.language, string.value)
                }
            }
        }
    }
    match contents {
        lsp::HoverContents::Scalar(contents) => marked_string_to_markdown(contents),
        lsp::HoverContents::Array(contents) => contents
            .into_iter()
            .map(marked_string_to_markdown)
            .collect::<Vec<_>>()
            .join("\n\n"),
        lsp::HoverContents::Markup(contents) => contents.value,
    }
}
