use std::sync::Arc;

use arc_swap::ArcSwap;
use helix_core::syntax;
use helix_lsp::lsp;
use helix_view::graphics::{Margin, Rect, Style};
use helix_view::input::Event;
use tui::buffer::Buffer;
use tui::widgets::{BorderType, Paragraph, Widget, Wrap};

use crate::compositor::{Component, Compositor, Context, EventResult};

use crate::alt;
use crate::ui::Markdown;

use crate::ui::Popup;

pub struct Hover {
    hovers: Vec<(String, lsp::Hover)>,
    active_index: usize,
    config_loader: Arc<ArcSwap<syntax::Loader>>,

    header: Option<Markdown>,
    contents: Option<Markdown>,
}

impl Hover {
    pub const ID: &'static str = "hover";

    pub fn new(
        hovers: Vec<(String, lsp::Hover)>,
        config_loader: Arc<ArcSwap<syntax::Loader>>,
    ) -> Self {
        let mut hover = Self {
            hovers,
            active_index: usize::default(),
            config_loader,
            header: None,
            contents: None,
        };
        hover.set_index(hover.active_index);
        hover
    }

    fn prepare_markdowns(&mut self) {
        let Some((lsp_name, hover)) = self.hovers.get(self.active_index) else {
            log::info!(
                "prepare_markdowns: failed \nindex:{}\ncount:{}",
                self.active_index,
                self.hovers.len()
            );
            return;
        };
        self.header = Some(Markdown::new(
            format!(
                "**[{}/{}] {}**",
                self.active_index + 1,
                self.hovers.len(),
                lsp_name
            ),
            self.config_loader.clone(),
        ));
        let contents = hover_contents_to_string(&hover.contents);
        self.contents = Some(Markdown::new(contents, self.config_loader.clone()));
    }

    pub fn set_hover(&mut self, hovers: Vec<(String, lsp::Hover)>) {
        self.hovers = hovers;
        self.set_index(usize::default());
    }

    fn set_index(&mut self, index: usize) {
        self.active_index = index;
        self.prepare_markdowns();
    }

    pub fn next_hover(&mut self) {
        let index = if self.active_index < self.hovers.len() - 1 {
            self.active_index + 1
        } else {
            usize::default()
        };
        self.set_index(index);
    }

    pub fn previous_hover(&mut self) {
        let index = if self.active_index > 0 {
            self.active_index - 1
        } else {
            self.hovers.len() - 1
        };
        self.set_index(index);
    }

    pub fn visible_popup(compositor: &mut Compositor) -> Option<&mut Popup<Self>> {
        compositor.find_id::<Popup<Self>>(Self::ID)
    }
}

const PADDING: u16 = 2;
const HEADER_HEIGHT: u16 = 1;
const SEPARATOR_HEIGHT: u16 = 1;

impl Component for Hover {
    fn render(&mut self, area: Rect, surface: &mut Buffer, cx: &mut Context) {
        let margin = Margin::horizontal(1);
        let area = area.inner(margin);

        let (Some(header), Some(contents)) = (self.header.as_ref(), self.contents.as_ref()) else {
            log::info!("markdown not ready");
            return;
        };

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

        // hover content
        let contents = contents.parse(Some(&cx.editor.theme));
        let contents_area = area
            .clip_top(2)
            .clip_bottom(u16::from(cx.editor.popup_border()));
        let contents_para = Paragraph::new(&contents)
            .wrap(Wrap { trim: false })
            .scroll((cx.scroll.unwrap_or_default() as u16, 0));
        contents_para.render(contents_area, surface);
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        let max_text_width = viewport.0.saturating_sub(PADDING).clamp(10, 120);

        let (Some(header), Some(contents)) = (self.header.as_ref(), self.contents.as_ref()) else {
            log::info!("markdown not ready");
            return None;
        };

        let header = header.parse(None);
        let (header_width, _header_height) =
            crate::ui::text::required_size(&header, max_text_width);

        let contents = contents.parse(None);
        let (content_width, content_height) =
            crate::ui::text::required_size(&contents, max_text_width);

        let (width, height) = (
            header_width.max(content_width),
            HEADER_HEIGHT + SEPARATOR_HEIGHT + content_height,
        );

        Some((width + PADDING, height + PADDING))
    }

    fn handle_event(&mut self, event: &Event, _ctx: &mut Context) -> EventResult {
        let Event::Key(event) = event else {
            return EventResult::Ignored(None);
        };

        match event {
            alt!('p') => {
                self.previous_hover();
                EventResult::Consumed(None)
            }
            alt!('n') => {
                self.next_hover();
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
