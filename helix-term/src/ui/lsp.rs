use std::sync::Arc;

use helix_core::syntax;
use helix_view::graphics::{Margin, Rect, Style};
use tui::buffer::Buffer;
use tui::widgets::{BorderType, Paragraph, Widget, Wrap};

use crate::compositor::{Component, Compositor, Context};

use crate::ui::Markdown;

use super::Popup;

pub struct SignatureHelp {
    signature: String,
    signature_doc: Option<String>,
    /// Part of signature text
    active_param_range: Option<(usize, usize)>,

    language: String,
    config_loader: Arc<syntax::Loader>,
}

impl SignatureHelp {
    pub const ID: &'static str = "signature-help";

    pub fn new(signature: String, language: String, config_loader: Arc<syntax::Loader>) -> Self {
        Self {
            signature,
            signature_doc: None,
            active_param_range: None,
            language,
            config_loader,
        }
    }

    pub fn set_signature_doc(&mut self, signature_doc: Option<String>) {
        self.signature_doc = signature_doc;
    }

    pub fn set_active_param_range(&mut self, offset: Option<(usize, usize)>) {
        self.active_param_range = offset;
    }

    pub fn visible_popup(compositor: &mut Compositor) -> Option<&mut Popup<Self>> {
        compositor.find_id::<Popup<Self>>(Self::ID)
    }
}

impl Component for SignatureHelp {
    fn render(&mut self, area: Rect, surface: &mut Buffer, cx: &mut Context) {
        let margin = Margin::horizontal(1);

        let active_param_span = self.active_param_range.map(|(start, end)| {
            vec![(
                cx.editor
                    .theme
                    .find_scope_index_exact("ui.selection")
                    .unwrap(),
                start..end,
            )]
        });

        let sig_text = crate::ui::markdown::highlighted_code_block(
            &self.signature,
            &self.language,
            Some(&cx.editor.theme),
            Arc::clone(&self.config_loader),
            active_param_span,
        );

        let (_, sig_text_height) = crate::ui::text::required_size(&sig_text, area.width);
        let sig_text_area = area.clip_top(1).with_height(sig_text_height);
        let sig_text_area = sig_text_area.inner(&margin).intersection(surface.area);
        let sig_text_para = Paragraph::new(sig_text).wrap(Wrap { trim: false });
        sig_text_para.render(sig_text_area, surface);

        if self.signature_doc.is_none() {
            return;
        }

        let sep_style = Style::default();
        let borders = BorderType::line_symbols(BorderType::Plain);
        for x in sig_text_area.left()..sig_text_area.right() {
            if let Some(cell) = surface.get_mut(x, sig_text_area.bottom()) {
                cell.set_symbol(borders.horizontal).set_style(sep_style);
            }
        }

        let sig_doc = match &self.signature_doc {
            None => return,
            Some(doc) => Markdown::new(doc.clone(), Arc::clone(&self.config_loader)),
        };
        let sig_doc = sig_doc.parse(Some(&cx.editor.theme));
        let sig_doc_area = area
            .clip_top(sig_text_area.height + 2)
            .clip_bottom(u16::from(cx.editor.popup_border()));
        let sig_doc_para = Paragraph::new(sig_doc)
            .wrap(Wrap { trim: false })
            .scroll((cx.scroll.unwrap_or_default() as u16, 0));
        sig_doc_para.render(sig_doc_area.inner(&margin), surface);
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        const PADDING: u16 = 2;
        const SEPARATOR_HEIGHT: u16 = 1;

        if PADDING >= viewport.1 || PADDING >= viewport.0 {
            return None;
        }
        let max_text_width = (viewport.0 - PADDING).min(120);

        let signature_text = crate::ui::markdown::highlighted_code_block(
            &self.signature,
            &self.language,
            None,
            Arc::clone(&self.config_loader),
            None,
        );
        let (sig_width, sig_height) =
            crate::ui::text::required_size(&signature_text, max_text_width);

        let (width, height) = match self.signature_doc {
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

        Some((width + PADDING, height + PADDING))
    }
}
