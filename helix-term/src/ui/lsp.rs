use std::sync::Arc;

use arc_swap::ArcSwap;
use helix_core::syntax;
use helix_view::graphics::{Margin, Rect, Style};
use helix_view::input::Event;
use tui::buffer::Buffer;
use tui::layout::Alignment;
use tui::text::Text;
use tui::widgets::{BorderType, Paragraph, Widget, Wrap};

use crate::compositor::{Component, Compositor, Context, EventResult};

use crate::alt;
use crate::ui::Markdown;

use super::Popup;

pub struct Signature {
    pub signature: String,
    pub signature_doc: Option<String>,
    /// Part of signature text
    pub active_param_range: Option<(usize, usize)>,
}

pub struct SignatureHelp {
    language: String,
    config_loader: Arc<ArcSwap<syntax::Loader>>,
    active_signature: usize,
    signatures: Vec<Signature>,
}

impl SignatureHelp {
    pub const ID: &'static str = "signature-help";

    pub fn new(
        language: String,
        config_loader: Arc<ArcSwap<syntax::Loader>>,
        active_signature: usize,
        signatures: Vec<Signature>,
    ) -> Self {
        Self {
            language,
            config_loader,
            active_signature,
            signatures,
        }
    }

    pub fn active_signature(&self) -> usize {
        self.active_signature
    }

    pub fn visible_popup(compositor: &mut Compositor) -> Option<&mut Popup<Self>> {
        compositor.find_id::<Popup<Self>>(Self::ID)
    }

    fn signature_index(&self) -> String {
        format!("({}/{})", self.active_signature + 1, self.signatures.len())
    }
}

impl Component for SignatureHelp {
    fn handle_event(&mut self, event: &Event, _cx: &mut Context) -> EventResult {
        let Event::Key(event) = event else {
            return EventResult::Ignored(None);
        };

        if self.signatures.len() <= 1 {
            return EventResult::Ignored(None);
        }

        match event {
            alt!('p') => {
                self.active_signature = self
                    .active_signature
                    .checked_sub(1)
                    .unwrap_or(self.signatures.len() - 1);
                EventResult::Consumed(None)
            }
            alt!('n') => {
                self.active_signature = (self.active_signature + 1) % self.signatures.len();
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored(None),
        }
    }

    fn render(&mut self, area: Rect, surface: &mut Buffer, cx: &mut Context) {
        let margin = Margin::horizontal(1);

        let signature = &self.signatures[self.active_signature];

        let active_param_span = signature.active_param_range.map(|(start, end)| {
            vec![(
                cx.editor
                    .theme
                    .find_scope_index_exact("ui.selection")
                    .unwrap(),
                start..end,
            )]
        });

        let sig = &self.signatures[self.active_signature];
        let sig_text = crate::ui::markdown::highlighted_code_block(
            sig.signature.as_str(),
            &self.language,
            Some(&cx.editor.theme),
            Arc::clone(&self.config_loader),
            active_param_span,
        );

        if self.signatures.len() > 1 {
            let signature_index = self.signature_index();
            let text = Text::from(signature_index);
            let paragraph = Paragraph::new(&text).alignment(Alignment::Right);
            paragraph.render(area.clip_top(1).with_height(1).clip_right(1), surface);
        }

        let (_, sig_text_height) = crate::ui::text::required_size(&sig_text, area.width);
        let sig_text_area = area.clip_top(1).with_height(sig_text_height);
        let sig_text_area = sig_text_area.inner(&margin).intersection(surface.area);
        let sig_text_para = Paragraph::new(&sig_text).wrap(Wrap { trim: false });
        sig_text_para.render(sig_text_area, surface);

        if sig.signature_doc.is_none() {
            return;
        }

        let sep_style = Style::default();
        let borders = BorderType::line_symbols(BorderType::Plain);
        for x in sig_text_area.left()..sig_text_area.right() {
            if let Some(cell) = surface.get_mut(x, sig_text_area.bottom()) {
                cell.set_symbol(borders.horizontal).set_style(sep_style);
            }
        }

        let sig_doc = match &sig.signature_doc {
            None => return,
            Some(doc) => Markdown::new(doc.clone(), Arc::clone(&self.config_loader)),
        };
        let sig_doc = sig_doc.parse(Some(&cx.editor.theme));
        let sig_doc_area = area
            .clip_top(sig_text_area.height + 2)
            .clip_bottom(u16::from(cx.editor.popup_border()));
        let sig_doc_para = Paragraph::new(&sig_doc)
            .wrap(Wrap { trim: false })
            .scroll((cx.scroll.unwrap_or_default() as u16, 0));
        sig_doc_para.render(sig_doc_area.inner(&margin), surface);
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        const PADDING: u16 = 2;
        const SEPARATOR_HEIGHT: u16 = 1;

        let sig = &self.signatures[self.active_signature];

        if PADDING >= viewport.1 || PADDING >= viewport.0 {
            return None;
        }
        let max_text_width = (viewport.0 - PADDING).min(120);

        let signature_text = crate::ui::markdown::highlighted_code_block(
            sig.signature.as_str(),
            &self.language,
            None,
            Arc::clone(&self.config_loader),
            None,
        );
        let (sig_width, sig_height) =
            crate::ui::text::required_size(&signature_text, max_text_width);

        let (width, height) = match sig.signature_doc {
            Some(ref doc) => {
                let doc_md = Markdown::new(doc.clone(), Arc::clone(&self.config_loader));
                let doc_text = doc_md.parse(None);
                let (doc_width, doc_height) =
                    crate::ui::text::required_size(&doc_text, max_text_width);
                (
                    sig_width.max(doc_width),
                    sig_height + SEPARATOR_HEIGHT + doc_height,
                )
            }
            None => (sig_width, sig_height),
        };

        let sig_index_width = if self.signatures.len() > 1 {
            self.signature_index().len() + 1
        } else {
            0
        };

        Some((width + PADDING + sig_index_width as u16, height + PADDING))
    }
}
