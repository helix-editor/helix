use crate::compositor::{Component, Context, Event, EventResult};
use crate::ui::prompt::{Completion, Prompt, PromptEvent};
use crate::ui::{self, gradient_border::GradientBorder};
use helix_core::{Position, unicode::width::UnicodeWidthStr};
use helix_view::{
    graphics::{CursorKind, Rect},
    Editor,
    editor::CmdlineStyle,
};
use std::borrow::Cow;
use tui::{
    buffer::Buffer as Surface,
    widgets::{Block, BorderType, Widget},
};

pub struct CmdlinePopup {
    prompt: Prompt,
    style: CmdlineStyle,
    // Popup-specific properties
    popup_area: Rect,
    min_width: u16,
    max_width: u16,
    padding: u16,
    // Gradient border for cmdline popup
    gradient_border: Option<GradientBorder>,
}

impl CmdlinePopup {
    pub fn new(
        prompt_text: Cow<'static, str>,
        history_register: Option<char>,
        completion_fn: impl FnMut(&Editor, &str) -> Vec<Completion> + 'static,
        callback_fn: impl FnMut(&mut Context, &str, PromptEvent) + 'static,
        style: CmdlineStyle,
    ) -> Self {
        Self {
            prompt: Prompt::new(prompt_text, history_register, completion_fn, callback_fn),
            style,
            popup_area: Rect::default(),
            min_width: 40,
            max_width: 80,
            padding: 2,
            gradient_border: None,
        }
    }

    pub fn with_line(mut self, line: String, editor: &Editor) -> Self {
        self.prompt = self.prompt.with_line(line, editor);
        self
    }

    pub fn with_language(
        mut self,
        language: &'static str,
        loader: std::sync::Arc<arc_swap::ArcSwap<helix_core::syntax::Loader>>,
    ) -> Self {
        self.prompt = self.prompt.with_language(language, loader);
        self
    }

    /// Calculate optimal popup dimensions and position
    fn calculate_popup_area(&self, viewport: Rect) -> Rect {
        let content_width = self.prompt.line().width().max(self.min_width as usize);
        let width = (content_width as u16 + self.padding * 2)
            .min(self.max_width)
            .min(viewport.width.saturating_sub(4));

        let height = 3; // Base height for single line + borders

        let x = viewport.x + (viewport.width.saturating_sub(width)) / 2;
        let y = viewport.y + (viewport.height.saturating_sub(height)) / 3; // Position in upper third

        Rect::new(x, y, width, height)
    }

    /// Get command type icon based on the input
    fn get_command_icon<'a>(&self, config: &'a helix_view::editor::CmdlineIcons) -> &'a str {
        let line = self.prompt.line();
        if line.starts_with("search:") || line.starts_with("/") || line.starts_with("?") {
            &config.search
        } else if line.starts_with(":") {
            &config.command
        } else if line.starts_with('!') {
            &config.shell
        } else {
            // Check if this is a regex prompt by looking at the prompt text
            match self.prompt.prompt() {
                s if s.starts_with("search:") => &config.search,
                _ => &config.general
            }
        }
    }

    /// Render popup-style cmdline
    fn render_popup(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let popup_area = self.calculate_popup_area(area);
        self.popup_area = popup_area;

        let theme = &cx.editor.theme;
        let config = &cx.editor.config().gradient_borders;

        // Clear the area
        surface.clear_with(popup_area, theme.get("ui.popup"));

        // Use gradient border if enabled, otherwise use default border
        let inner_area = if config.enable && self.style == CmdlineStyle::Popup {
            // Initialize gradient border if needed
            if self.gradient_border.is_none() {
                self.gradient_border = Some(GradientBorder::from_theme(theme, config));
            }

            // Render gradient border
            if let Some(ref mut gradient_border) = self.gradient_border {
                let rounded = cx.editor.config().rounded_corners;
                gradient_border.render(popup_area, surface, theme, rounded);
            }

            // Calculate inner area manually (same as Block::inner)
            Rect {
                x: popup_area.x + 1,
                y: popup_area.y + 1,
                width: popup_area.width.saturating_sub(2),
                height: popup_area.height.saturating_sub(2),
            }
        } else {
            // Use traditional border
            let border_style = theme.get("ui.popup.border");
            let background_style = theme.get("ui.popup");

            let border_type = BorderType::new(cx.editor.config().rounded_corners);
            let block = Block::default()
                .borders(tui::widgets::Borders::ALL)
                .border_type(border_type)
                .border_style(border_style)
                .style(background_style);

            let inner_area = block.inner(popup_area);
            block.render(popup_area, surface);
            inner_area
        };

        // Render command icon and prompt
        let config = cx.editor.config();
        let icon = if config.cmdline.show_icons {
            self.get_command_icon(&config.cmdline.icons)
        } else {
            ""
        };
        let prompt_text = if icon.is_empty() {
            format!("{} ", self.prompt.prompt()) // Add space after prompt for input
        } else {
            format!("{} {} ", icon, self.prompt.prompt()) // Add space after prompt for input
        };

        let prompt_color = theme.get("ui.text.focus");
        surface.set_string(
            inner_area.x,
            inner_area.y,
            &prompt_text,
            prompt_color,
        );

        // Calculate input area
        let input_area = Rect::new(
            inner_area.x + prompt_text.width() as u16,
            inner_area.y,
            inner_area.width.saturating_sub(prompt_text.width() as u16),
            1,
        );

        // Render input text with syntax highlighting if available
        self.render_input_text(input_area, surface, cx);

        // Render completion popup if needed
        if !self.prompt.completions().is_empty() {
            self.render_completion_popup(popup_area, surface, cx);
        }
    }

    /// Render the input text with syntax highlighting
    fn render_input_text(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let theme = &cx.editor.theme;
        let text_style = theme.get("ui.text");

        if let Some((language, loader)) = self.prompt.language().as_ref() {
            // Use syntax highlighting for known languages
            let mut text: ui::text::Text = crate::ui::markdown::highlighted_code_block(
                self.prompt.line(),
                language,
                Some(&cx.editor.theme),
                &loader.load(),
                None,
            )
            .into();
            text.render(area, surface, cx);
        } else {
            // Fallback to plain text
            surface.set_string(area.x, area.y, self.prompt.line(), text_style);
        }
    }

    /// Render completion popup
    fn render_completion_popup(&mut self, base_area: Rect, surface: &mut Surface, cx: &Context) {
        let theme = &cx.editor.theme;
        let completion_bg = theme.get("ui.menu");
        let selected_style = theme.get("ui.menu.selected");

        // Position completion popup below the main popup
        let comp_height = (self.prompt.completions().len() as u16).min(8) + 2; // Max 8 items + borders
        let comp_width = base_area.width;
        let comp_area = Rect::new(
            base_area.x,
            base_area.y + base_area.height + 1,
            comp_width,
            comp_height,
        );

        // Clear and render completion background
        surface.clear_with(comp_area, completion_bg);

        let config = &cx.editor.config().gradient_borders;

        // Use gradient border for completion if enabled
        let inner_area = if config.enable && self.style == CmdlineStyle::Popup {
            // Reuse the gradient border for completion popup
            if let Some(ref mut gradient_border) = self.gradient_border {
                let rounded = cx.editor.config().rounded_corners;
                gradient_border.render(comp_area, surface, &cx.editor.theme, rounded);
            }

            // Calculate inner area manually
            Rect {
                x: comp_area.x + 1,
                y: comp_area.y + 1,
                width: comp_area.width.saturating_sub(2),
                height: comp_area.height.saturating_sub(2),
            }
        } else {
            // Use traditional border
            let border_type = BorderType::new(cx.editor.config().rounded_corners);
            let block = Block::default()
                .borders(tui::widgets::Borders::ALL)
                .border_type(border_type)
                .border_style(completion_bg)
                .style(completion_bg);

            let inner_area = block.inner(comp_area);
            block.render(comp_area, surface);
            inner_area
        };

        // Render completion items
        for (i, (_range, completion)) in self.prompt.completions().iter().enumerate().take(8) {
            let y = inner_area.y + i as u16;
            let is_selected = self.prompt.selection() == Some(i);
            let item_style = if is_selected {
                selected_style
            } else {
                completion_bg.patch(completion.style)
            };

            let prefix = if is_selected {
                format!("{} ", cx.editor.config().picker_symbol)
            } else {
                "  ".to_string()
            };
            let text = format!("{}{}", prefix, completion.content);

            surface.set_stringn(
                inner_area.x,
                y,
                &text,
                inner_area.width as usize,
                item_style,
            );
        }
    }

    /// Render traditional bottom cmdline
    fn render_bottom(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        // Delegate to the original prompt rendering
        self.prompt.render_prompt(area, surface, cx);
    }
}

impl Component for CmdlinePopup {
    fn handle_event(&mut self, event: &Event, cx: &mut Context) -> EventResult {
        // Delegate event handling to the underlying prompt
        self.prompt.handle_event(event, cx)
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        match self.style {
            CmdlineStyle::Popup => self.render_popup(area, surface, cx),
            CmdlineStyle::Bottom => self.render_bottom(area, surface, cx),
        }
    }

    fn cursor(&self, area: Rect, editor: &Editor) -> (Option<Position>, CursorKind) {
        match self.style {
            CmdlineStyle::Popup => {
                // Calculate cursor position for popup style
                let config = editor.config();
                let icon = if config.cmdline.show_icons {
                    self.get_command_icon(&config.cmdline.icons)
                } else {
                    ""
                };
                let prompt_text = if icon.is_empty() {
                    format!("{} ", self.prompt.prompt()) // Add space after prompt for input
                } else {
                    format!("{} {} ", icon, self.prompt.prompt()) // Add space after prompt for input
                };
                let total_prefix_width = prompt_text.width(); // No extra space needed

                let inner_area = Block::default()
                    .borders(tui::widgets::Borders::ALL)
                    .inner(self.popup_area);

                // For now, just put cursor at end of input
                // TODO: Fix cursor positioning to match actual cursor in prompt
                let cursor_pos = self.prompt.line().width();

                let cursor_x = inner_area.x as usize + total_prefix_width + cursor_pos;
                let cursor_y = inner_area.y as usize;

                (
                    Some(Position::new(cursor_y, cursor_x)),
                    editor.config().cursor_shape.from_mode(helix_view::document::Mode::Insert),
                )
            }
            CmdlineStyle::Bottom => {
                // Delegate to original prompt cursor calculation
                self.prompt.cursor(area, editor)
            }
        }
    }
}