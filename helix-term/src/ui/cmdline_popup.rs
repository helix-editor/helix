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
                s if s.starts_with("search:") || s == "Search" => &config.search,
                s if s == "Cmdline" => &config.command,
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

            // Render gradient border with title
            if let Some(ref mut gradient_border) = self.gradient_border {
                let rounded = cx.editor.config().rounded_corners;
                gradient_border.render_with_title(popup_area, surface, theme, Some(self.prompt.prompt()), rounded);
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
                .style(background_style)
                .title(self.prompt.prompt());

            let inner_area = block.inner(popup_area);
            block.render(popup_area, surface);
            inner_area
        };

        // Render command icon (if enabled) but not the prompt text since it's now on the border
        let config = cx.editor.config();
        let icon = if config.cmdline.show_icons {
            self.get_command_icon(&config.cmdline.icons)
        } else {
            ""
        };
        // Render icon without trailing space to avoid extra padding before input
        let prefix_text = if icon.is_empty() { "".to_string() } else { icon.to_string() };

        if !prefix_text.is_empty() {
            let prompt_color = theme.get("ui.text.focus");
            // Make the icon more prominent with bold styling
            let icon_style = prompt_color.add_modifier(helix_view::theme::Modifier::BOLD);
            surface.set_string(
                inner_area.x,
                inner_area.y,
                &prefix_text,
                icon_style,
            );
        }

        // Calculate input area
        let input_area = Rect::new(
            inner_area.x + prefix_text.width() as u16,
            inner_area.y,
            inner_area.width.saturating_sub(prefix_text.width() as u16),
            1,
        );

        // Render input text with syntax highlighting if available
        self.render_input_text(input_area, surface, cx);

        // Render completion popup if needed
        if !self.prompt.completions().is_empty() {
            self.render_completion_popup(popup_area, surface, cx);
        }
    }

    /// Render the input text (popup: render plain to avoid padding/offsets)
    fn render_input_text(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let theme = &cx.editor.theme;
        let text_style = theme.get("ui.text");
        // Always render plain text for exact alignment with cursor positioning.
        surface.set_string(area.x, area.y, self.prompt.line(), text_style);
    }

    /// Render completion popup
    fn render_completion_popup(&mut self, base_area: Rect, surface: &mut Surface, cx: &Context) {
        let theme = &cx.editor.theme;
        // Match global autocomplete/picker colors
        let completion_bg = theme.get("ui.menu");
        let selected_row_bg = theme.get("ui.menu.selected");

        // Position completion popup below the main popup
        let max_display_items = 10; // Fixed maximum items to display
        let total_items = self.prompt.completions().len();
        let visible_items = total_items.min(max_display_items);
        let comp_height = visible_items as u16 + 2; // Fixed height based on visible items + borders
        let comp_width = base_area.width;
        let comp_area = Rect::new(
            base_area.x,
            base_area.y + base_area.height + 1,
            comp_width,
            comp_height,
        );

        // Clear and render completion background (match global autocomplete)
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
            // Use traditional border, matching popup card styling
            let border_type = BorderType::new(cx.editor.config().rounded_corners);
            let block = Block::default()
                .borders(tui::widgets::Borders::ALL)
                .border_type(border_type)
                .border_style(theme.get("ui.popup.border"))
                .style(completion_bg);

            let inner_area = block.inner(comp_area);
            block.render(comp_area, surface);
            inner_area
        };

        // Render completion items with scrolling support
        let config = cx.editor.config();
        let picker_symbol = config.picker_symbol.trim();
        let symbol_width = picker_symbol.width();
        
        let completions = self.prompt.completions();
        let selected_index = self.prompt.selection().unwrap_or(0);
        
        // Calculate scroll offset to keep selected item visible within the fixed window
        let scroll_offset = if selected_index >= max_display_items {
            selected_index.saturating_sub(max_display_items - 1)
        } else {
            0
        };
        
        // Render visible completion items
        for (display_idx, (completion_idx, (_range, completion))) in completions
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(max_display_items)
            .enumerate()
        {
            let y = inner_area.y + display_idx as u16;
            let is_selected = self.prompt.selection() == Some(completion_idx);
            let item_style = if is_selected {
                // Fill the whole selected row across the popup width.
                let spaces = " ".repeat(inner_area.width as usize);
                surface.set_stringn(
                    inner_area.x,
                    y,
                    &spaces,
                    inner_area.width as usize,
                    selected_row_bg,
                );
                // Use the theme's selected style for text (fg+bg from ui.menu.selected)
                selected_row_bg
            } else {
                completion_bg.patch(completion.style)
            };

            let prefix = if is_selected {
                picker_symbol.to_string()
            } else {
                " ".repeat(symbol_width)
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
        
        // Add scroll indicators if there are more items
        if total_items > max_display_items {
            let scroll_indicator_style = theme.get("ui.text.inactive");
            
            // Show up arrow if we can scroll up
            if scroll_offset > 0 {
                surface.set_string(
                    inner_area.x + inner_area.width.saturating_sub(1),
                    inner_area.y,
                    "↑",
                    scroll_indicator_style,
                );
            }
            
            // Show down arrow if we can scroll down
            if scroll_offset + max_display_items < total_items {
                surface.set_string(
                    inner_area.x + inner_area.width.saturating_sub(1),
                    inner_area.y + inner_area.height.saturating_sub(1),
                    "↓",
                    scroll_indicator_style,
                );
            }
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
                let prefix_width = if icon.is_empty() { 0 } else { icon.width() };

                // Compute inner area similarly to render: border consumes 1 px around
                let inner_area = Block::default()
                    .borders(tui::widgets::Borders::ALL)
                    .inner(self.popup_area);

                // Build the same input area used in render_popup
                let input_area = Rect::new(
                    inner_area.x + prefix_width as u16,
                    inner_area.y,
                    inner_area.width.saturating_sub(prefix_width as u16),
                    1,
                );

                // Compute cursor directly to avoid relying on bottom-mode internals
                let byte_pos = self.prompt.position();
                let line = self.prompt.line();
                let grapheme_w = line[..byte_pos].width() as u16;
                let clamped_w = grapheme_w.min(input_area.width.saturating_sub(1));
                let cursor_x = input_area.x as usize + clamped_w as usize;
                let cursor_y = input_area.y as usize;

                (
                    Some(Position::new(cursor_y, cursor_x)),
                    editor
                        .config()
                        .cursor_shape
                        .from_mode(helix_view::document::Mode::Insert),
                )
            }
            CmdlineStyle::Bottom => {
                // Delegate to original prompt cursor calculation
                self.prompt.cursor(area, editor)
            }
        }
    }
}
