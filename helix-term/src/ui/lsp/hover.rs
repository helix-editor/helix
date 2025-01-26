use std::sync::Arc;

use arc_swap::ArcSwap;
use helix_core::syntax;
use helix_lsp::lsp;
use helix_view::graphics::{Margin, Rect, Style};
use helix_view::input::Event;
use once_cell::sync::OnceCell;
use tui::buffer::Buffer;
use tui::widgets::{BorderType, Paragraph, Widget, Wrap};

use crate::compositor::{Component, Context, EventResult};

use crate::alt;
use crate::ui::Markdown;

pub struct Hover {
    hovers: Vec<(String, lsp::Hover)>,
    active_index: usize,
    config_loader: Arc<ArcSwap<syntax::Loader>>,

    content: OnceCell<(Option<Markdown>, Markdown)>,
}

impl Hover {
    pub const ID: &'static str = "hover";

    pub fn new(
        hovers: Vec<(String, lsp::Hover)>,
        config_loader: Arc<ArcSwap<syntax::Loader>>,
    ) -> Self {
        Self {
            hovers,
            active_index: usize::default(),
            config_loader,
            content: OnceCell::new(),
        }
    }

    fn content(&self) -> &(Option<Markdown>, Markdown) {
        self.content.get_or_init(|| {
            let (server_name, hover) = &self.hovers[self.active_index];
            // Only render the header when there is more than one hover response.
            let header = (self.hovers.len() > 1).then(|| {
                Markdown::new(
                    format!(
                        "**[{}/{}] {}**",
                        self.active_index + 1,
                        self.hovers.len(),
                        server_name
                    ),
                    self.config_loader.clone(),
                )
            });
            let body = Markdown::new(
                hover_contents_to_string(&hover.contents),
                self.config_loader.clone(),
            );
            (header, body)
        })
    }

    fn set_index(&mut self, index: usize) {
        assert!((0..self.hovers.len()).contains(&index));
        self.active_index = index;
        // Reset the cached markdown:
        self.content.take();
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
        let contents_area = area
            .clip_top(if self.hovers.len() > 1 {
                HEADER_HEIGHT + SEPARATOR_HEIGHT
            } else {
                0
            })
            .clip_bottom(u16::from(cx.editor.popup_border()));
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
        let height = if self.hovers.len() > 1 {
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
                    .unwrap_or(self.hovers.len() - 1);
                self.set_index(index);
                EventResult::Consumed(None)
            }
            alt!('n') => {
                self.set_index((self.active_index + 1) % self.hovers.len());
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored(None),
        }
    }
}

fn hover_contents_to_string(contents: &lsp::HoverContents) -> String {
    fn marked_string_to_markdown(contents: &lsp::MarkedString) -> String {
        match contents {
            lsp::MarkedString::String(contents) => contents.clone(),
            lsp::MarkedString::LanguageString(string) => {
                if string.language == "markdown" {
                    string.value.clone()
                } else {
                    format!("```{}\n{}\n```", string.language, string.value)
                }
            }
        }
    }
    match contents {
        lsp::HoverContents::Scalar(contents) => marked_string_to_markdown(contents),
        lsp::HoverContents::Array(contents) => contents
            .iter()
            .map(marked_string_to_markdown)
            .collect::<Vec<_>>()
            .join("\n\n"),
        lsp::HoverContents::Markup(contents) => contents.value.clone(),
    }
}
