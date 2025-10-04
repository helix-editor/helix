use crate::compositor::{Component, Context, Event, EventResult};
use crate::ui::gradient_border::GradientBorder;
use helix_core::unicode::width::UnicodeWidthStr;
use helix_event::request_redraw;
use helix_view::{
    graphics::{Color, Rect, Style},
    Editor,
    editor::{Notification, NotificationPosition, NotificationStyle, NotificationBorderStyle, Severity, CmdlineStyle},
};
use helix_view::theme::Modifier;
use std::time::Instant;
use tokio::time::{sleep as tokio_sleep, Instant as TokioInstant};
use tui::{
    buffer::Buffer as Surface,
    widgets::{Block, BorderType, Borders, Widget},
};

pub struct NotificationPopup {
    notifications: Vec<NotificationItem>,
    gradient_border: Option<GradientBorder>,
    last_update: Instant,
    // Cached layout params computed per render from editor config
    layout_thickness: u16,
    layout_rounded: bool,
    layout_padding: u16,
}

#[derive(Debug, Clone)]
struct NotificationItem {
    notification: Notification,
    area: Rect,
    fade_start: Option<Instant>,
    // When we've scheduled a wake-up for this item
    wake_until: Option<TokioInstant>,
}

impl NotificationItem {
    fn new(notification: Notification) -> Self {
        Self {
            notification,
            area: Rect::default(),
            fade_start: None,
            wake_until: None,
        }
    }

    fn start_fade(&mut self) {
        if self.fade_start.is_none() {
            self.fade_start = Some(Instant::now());
        }
    }

    fn is_fading(&self) -> bool {
        self.fade_start.is_some()
    }

    fn fade_progress(&self) -> f32 {
        if let Some(start) = self.fade_start {
            let elapsed = start.elapsed().as_millis() as f32;
            let fade_duration = 300.0; // 300ms fade
            (elapsed / fade_duration).min(1.0)
        } else {
            0.0
        }
    }

    fn should_remove(&self) -> bool {
        self.fade_progress() >= 1.0
    }
}

impl NotificationPopup {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
            gradient_border: None,
            last_update: Instant::now(),
            layout_thickness: 1,
            layout_rounded: false,
            layout_padding: 1,
        }
    }

    pub fn update(&mut self, editor: &Editor) {
        let config = &editor.config().notifications;
        
        // Only show popup notifications when cmdline style is Popup as requested
        if !config.enable
            || config.style != NotificationStyle::Popup
            || editor.config().cmdline.style != CmdlineStyle::Popup
        {
            self.notifications.clear();
            return;
        }

        // Get active notifications (this will filter out expired ones automatically)
        let active_notifications = editor.get_active_notifications();
        
        // Force cleanup of expired notifications on every render
        // Note: We can't call cleanup_expired here because editor is immutable,
        // but get_active_notifications already filters out expired ones
        
        // Remove expired checks/logs: get_active_notifications already filters them
        
        // Immediately drop notifications that are inactive or expired (no fading)
        self.notifications.retain_mut(|item| {
            let still_active = active_notifications.iter().any(|n| n.id == item.notification.id);
            let is_expired = item.notification.is_expired();
            still_active && !is_expired
        });

        // Add new notifications
        for notification in active_notifications {
            if !self.notifications.iter().any(|item| item.notification.id == notification.id) {
                let id = notification.id;
                let timeout_opt = notification.timeout;
                self.notifications.push(NotificationItem::new(notification.clone()));
                // New notification: ensure a redraw follows immediately
                request_redraw();

                // Schedule an exact-time dismiss on the UI thread if we have a timeout
                if let Some(timeout) = timeout_opt {
                    // Compute remaining time from the timestamp embedded in the cloned notification
                    let started = notification.timestamp; // tokio::time::Instant
                    let elapsed = started.elapsed();
                    let remaining = if timeout > elapsed { timeout - elapsed } else { std::time::Duration::from_millis(0) };

                    tokio::spawn(async move {
                        tokio_sleep(remaining).await;
                        // Post back to the UI thread to mutate Editor safely
                        crate::job::dispatch(move |editor: &mut helix_view::Editor, _compositor| {
                            editor.dismiss_notification(id);
                            helix_event::request_redraw();
                        })
                        .await;
                    });
                }
            }
        }

        // Avoid continuous redraws here to minimize refreshes.
    }

    fn calculate_notification_areas(&mut self, viewport: Rect, config: &helix_view::editor::NotificationConfig) {
        let max_width = config.max_width.min(viewport.width.saturating_sub(4));
        let spacing = 1u16;
        let mut y_offset = 0u16;

        // Calculate areas for each notification
        let mut areas = Vec::new();
        for item in &self.notifications {
            let message = &item.notification.message;
            // Estimate inner wrapping width: account for actual border thickness and padding only
            // Rounded corners do not require extra space; border thickness already accounts for frame size
            let wrap_inner_width = max_width
                .saturating_sub(self.layout_thickness * 2)
                .saturating_sub(self.layout_padding * 2)
                .max(1);

            // Build prefix for first line width calculation
            let mut prefix = String::new();
            if config.show_emojis {
                let emoji = self.get_notification_emoji(&item.notification.severity, &config.emojis);
                prefix.push_str(emoji);
                prefix.push(' ');
            } else if config.show_icons {
                let icon = self.get_notification_icon(&item.notification.severity, &config.icons);
                prefix.push_str(icon);
                prefix.push(' ');
            }
            let prefix_width = prefix.width() as u16;

            // Wrap content to the wrap_inner_width
            let content_lines = Self::wrap_text_static(message, wrap_inner_width);
            let content_height = content_lines.len() as u16;

            // Compute max visible line width including prefix on first line
            let mut content_max_w: u16 = 1;
            for (i, ln) in content_lines.iter().enumerate() {
                let mut w = ln.width() as u16;
                if i == 0 { w = w.saturating_add(prefix_width); }
                if w > content_max_w { content_max_w = w; }
            }

            // Outer size from content + padding + border thickness (no extra rounded margin)
            let mut width = content_max_w
                .saturating_add(self.layout_padding * 2)
                .saturating_add(self.layout_thickness * 2)
                .clamp(3, max_width);


            let mut height = content_height
                .saturating_add(self.layout_padding * 2)
                .saturating_add(self.layout_thickness * 2)
                .max(3);
            height = height.min(config.max_height.max(3));

            if log::log_enabled!(log::Level::Debug) {
                log::debug!(
                    "layout calc id={} wrap_inner_width={} content_max_w={} -> width={} height={} (padding={}, thick={})",
                    item.notification.id,
                    wrap_inner_width,
                    content_max_w,
                    width,
                    height,
                    self.layout_padding,
                    self.layout_thickness,
                );
            }

            let (x, y) = match config.position {
                NotificationPosition::TopLeft => (
                    viewport.x + 2,
                    viewport.y + y_offset + 1,
                ),
                NotificationPosition::TopCenter => (
                    viewport.x + (viewport.width.saturating_sub(width)) / 2,
                    viewport.y + y_offset + 1,
                ),
                NotificationPosition::TopRight => (
                    viewport.x + viewport.width.saturating_sub(width + 2),
                    viewport.y + y_offset + 1,
                ),
                NotificationPosition::BottomLeft => (
                    viewport.x + 2,
                    viewport.y + viewport.height.saturating_sub(height + y_offset + 1),
                ),
                NotificationPosition::BottomCenter => (
                    viewport.x + (viewport.width.saturating_sub(width)) / 2,
                    viewport.y + viewport.height.saturating_sub(height + y_offset + 1),
                ),
                NotificationPosition::BottomRight => (
                    viewport.x + viewport.width.saturating_sub(width + 2),
                    viewport.y + viewport.height.saturating_sub(height + y_offset + 1),
                ),
            };

            let rect = Rect::new(x, y, width, height);
            if log::log_enabled!(log::Level::Debug) {
                log::debug!(
                    "assign area id={} -> x={} y={} w={} h={}",
                    item.notification.id, rect.x, rect.y, rect.width, rect.height
                );
            }
            areas.push(rect);
            
            // For bottom positions, we need to stack upwards
            match config.position {
                NotificationPosition::BottomLeft | 
                NotificationPosition::BottomCenter | 
                NotificationPosition::BottomRight => {
                    y_offset += height + spacing;
                },
                _ => {
                    y_offset += height + spacing;
                }
            }
        }
        
        // Now assign the areas to the notifications
        for (item, area) in self.notifications.iter_mut().zip(areas.iter()) {
            item.area = *area;
        }
    }

    // No shims needed – sizing is conservative and independent of global gradient thickness.

    fn wrap_text_static(text: &str, max_width: u16) -> Vec<String> {
        let mut lines = Vec::new();
        let max_width = max_width as usize;

        for line in text.lines() {
            if line.width() <= max_width {
                lines.push(line.to_string());
            } else {
                let mut current_line = String::new();
                let mut current_width = 0;

                for word in line.split_whitespace() {
                    let word_width = word.width();
                    
                    if current_width + word_width + 1 <= max_width {
                        if !current_line.is_empty() {
                            current_line.push(' ');
                            current_width += 1;
                        }
                        current_line.push_str(word);
                        current_width += word_width;
                    } else {
                        if !current_line.is_empty() {
                            lines.push(current_line);
                            current_line = String::new();
                            current_width = 0;
                        }
                        
                        if word_width <= max_width {
                            current_line = word.to_string();
                            current_width = word_width;
                        } else {
                            // Word is too long, truncate it
                            let truncated = word.chars().take(max_width.saturating_sub(3)).collect::<String>() + "...";
                            lines.push(truncated);
                        }
                    }
                }
                
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
            }
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }

    fn wrap_text(&self, text: &str, max_width: u16) -> Vec<String> {
        Self::wrap_text_static(text, max_width)
    }

    fn get_notification_icon<'a>(&self, severity: &Severity, config: &'a helix_view::editor::NotificationIcons) -> &'a str {
        match severity {
            Severity::Error => &config.error,
            Severity::Warning => &config.warning,
            Severity::Info => &config.info,
            Severity::Hint => &config.info, // Use info icon for hints
        }
    }

    fn get_notification_emoji<'a>(&self, severity: &Severity, config: &'a helix_view::editor::NotificationEmojis) -> &'a str {
        match severity {
            Severity::Error => &config.error,
            Severity::Warning => &config.warning,
            Severity::Info => &config.info,
            Severity::Hint => &config.info, // Use info emoji for hints
        }
    }

    fn get_notification_style(&self, severity: &Severity, theme: &helix_view::Theme, fade_progress: f32) -> Style {
        let base_style = match severity {
            Severity::Error => theme.get("error"),
            Severity::Warning => theme.get("warning"),
            Severity::Info => theme.get("info"),
            Severity::Hint => theme.get("hint"),
        };

        if fade_progress > 0.0 {
            // Apply fade effect by reducing opacity (simulate with dimmed colors)
            // For now, just use a dimmed version by using a gray color
            if fade_progress > 0.5 {
                base_style.fg(Color::Gray)
            } else {
                base_style
            }
        } else {
            base_style
        }
    }

    fn get_border_type(&self, border_config: &helix_view::editor::NotificationBorderConfig, rounded: bool) -> BorderType {
        match border_config.style {
            NotificationBorderStyle::Solid => {
                if rounded || border_config.radius > 0 {
                    BorderType::Rounded
                } else {
                    BorderType::Plain
                }
            },
            NotificationBorderStyle::Dashed => BorderType::Double, // Approximate dashed with double
            NotificationBorderStyle::Dotted => BorderType::Thick,  // Approximate dotted with thick
        }
    }

    // Render a simple (non-gradient) border without painting the background.
    fn render_simple_border(
        &self,
        area: Rect,
        surface: &mut Surface,
        style: Style,
        rounded: bool,
        width: u8,
    ) {
        // Choose border glyphs
        let (h, v, tl, tr, bl, br) = if rounded {
            ("─", "│", "╭", "╮", "╰", "╯")
        } else {
            ("─", "│", "┌", "┐", "└", "┘")
        };

        let w = width.max(1) as u16;
        for s in 0..w {
            let x0 = area.x.saturating_add(s);
            let x1 = area.right().saturating_sub(1 + s);
            let y0 = area.y.saturating_add(s);
            let y1 = area.bottom().saturating_sub(1 + s);

            if x0 >= x1 || y0 >= y1 { break; }

            // Top and bottom lines
            for x in x0..=x1 {
                let ch_top = if x == x0 { tl } else if x == x1 { tr } else { h };
                let ch_bot = if x == x0 { bl } else if x == x1 { br } else { h };
                if let Some(cell) = surface.get_mut(x, y0) { cell.set_symbol(ch_top).set_style(style); }
                if let Some(cell) = surface.get_mut(x, y1) { cell.set_symbol(ch_bot).set_style(style); }
            }
            // Left and right lines
            for y in (y0+1)..y1 {
                if let Some(cell) = surface.get_mut(x0, y) { cell.set_symbol(v).set_style(style); }
                if let Some(cell) = surface.get_mut(x1, y) { cell.set_symbol(v).set_style(style); }
            }
        }
    }

    fn render_notification(&mut self, item: &NotificationItem, surface: &mut Surface, cx: &Context) {
        let config = &cx.editor.config().notifications;
        let theme = &cx.editor.theme;
        let fade_progress = item.fade_progress();

        if item.area.width < 4 || item.area.height < 3 {
            return; // Too small to render
        }

        // Optional drop shadow behind the popup (transparent overlay otherwise)
        if config.shadow.enable && item.area.width > 2 && item.area.height > 2 {
            let sx = item.area.x.saturating_add(config.shadow.offset_x);
            let sy = item.area.y.saturating_add(config.shadow.offset_y);
            let shadow_area = Rect {
                x: sx,
                y: sy,
                width: item.area.width,
                height: item.area.height,
            };
            // Use a dimmed background for shadow
            let mut shadow = theme.get("ui.popup");
            shadow = shadow.bg(Color::Rgb(0, 0, 0)).add_modifier(Modifier::DIM);
            surface.clear_with(shadow_area, shadow);
        }

        // Get notification style
        let notification_style = self.get_notification_style(&item.notification.severity, theme, fade_progress);
        let border_style = theme.get("ui.popup.border");
        let background_style = theme.get("ui.popup");

        // Per-notification corner radius override
        let corner_radius = item
            .notification
            .corner_radius
            .unwrap_or(config.border.radius);

        // Render border based on configuration
        let inner_area = if config.border.enable {
            if cx.editor.config().gradient_borders.enable {
                // Use gradient border
                if self.gradient_border.is_none() {
                    self.gradient_border = Some(GradientBorder::from_theme(theme, &cx.editor.config().gradient_borders));
                }

                if let Some(ref mut gradient_border) = self.gradient_border {
                    // Disable animation as requested
                    gradient_border.disable_animation();
                    let rounded = cx.editor.config().rounded_corners || corner_radius > 0;
                    gradient_border.render(item.area, surface, theme, rounded);
                }

                // Calculate inner area manually using configured gradient thickness
                {
                    let t: u16 = cx.editor.config().gradient_borders.thickness as u16;
                    Rect {
                        x: item.area.x + t,
                        y: item.area.y + t,
                        width: item.area.width.saturating_sub(t * 2),
                        height: item.area.height.saturating_sub(t * 2),
                    }
                }
            } else {
                // Render simple border without filling background
                let rounded = cx.editor.config().rounded_corners || corner_radius > 0;
                self.render_simple_border(
                    item.area,
                    surface,
                    border_style,
                    rounded,
                    config.border.width,
                );

                Rect {
                    x: item.area.x + config.border.width as u16,
                    y: item.area.y + config.border.width as u16,
                    width: item.area.width.saturating_sub(config.border.width as u16 * 2),
                    height: item.area.height.saturating_sub(config.border.width as u16 * 2),
                }
            }
        } else {
            // No border
            item.area
        };

        // Content area starts as inner area (no extra rounded clipping to preserve space)
        let mut content_area = inner_area;

        // Apply configured padding
        let pad = config.padding;
        if content_area.width > pad * 2 && content_area.height > pad * 2 {
            content_area = Rect {
                x: content_area.x + pad,
                y: content_area.y + pad,
                width: content_area.width - pad * 2,
                height: content_area.height - pad * 2,
            };
        }

        // Final safety clamps to ensure we have drawable area
        if content_area.width == 0 { content_area.width = 1; }
        if content_area.height == 0 { content_area.height = 1; }

        // Fill the entire inner area (card background). Overlay outside remains transparent.
        if inner_area.width > 0 && inner_area.height > 0 {
            surface.clear_with(inner_area, background_style);
        }

        // Render notification content inside the content_area
        let wrap_width = content_area.width.max(1);
        if log::log_enabled!(log::Level::Debug) {
            log::debug!(
                "render id={} item.area=({}, {}, {}, {}) inner=({}, {}, {}, {}) content=({}, {}, {}, {}) wrap_width={}",
                item.notification.id,
                item.area.x, item.area.y, item.area.width, item.area.height,
                inner_area.x, inner_area.y, inner_area.width, inner_area.height,
                content_area.x, content_area.y, content_area.width, content_area.height,
                wrap_width
            );
        }
        let content_lines = self.wrap_text(&item.notification.message, wrap_width);
        let mut y_pos = content_area.y;

        // Calculate prefix (icon/emoji + space)
        let mut prefix = String::new();
        if config.show_emojis {
            let emoji = self.get_notification_emoji(&item.notification.severity, &config.emojis);
            prefix.push_str(emoji);
            prefix.push(' ');
        } else if config.show_icons {
            let icon = self.get_notification_icon(&item.notification.severity, &config.icons);
            prefix.push_str(icon);
            prefix.push(' ');
        }

        let prefix_width = prefix.width() as u16;
        let show_prefix = !prefix.is_empty() && content_area.width > prefix_width + 1;

        for (i, line) in content_lines.iter().enumerate() {
            if y_pos >= content_area.y + content_area.height {
                break; // No more space
            }

            // Render prefix only on first line
            if i == 0 && show_prefix {
                surface.set_string(
                    content_area.x,
                    y_pos,
                    &prefix,
                    notification_style,
                );
            }

            // Render content
            let x_offset = if i == 0 && show_prefix { prefix_width } else { 0 };
            let available = content_area.width.saturating_sub(x_offset).max(1) as usize;
            surface.set_stringn(
                content_area.x + x_offset,
                y_pos,
                line,
                available,
                notification_style,
            );

            y_pos += 1;
        }
    }
}

impl Component for NotificationPopup {
    fn handle_event(&mut self, _event: &Event, _cx: &mut Context) -> EventResult {
        // Notifications don't handle events directly
        EventResult::Ignored(None)
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        // Always update on every render call
        self.update(cx.editor);

        // Update layout params cache from editor config
        // Use configured thickness for sizing (gradient or simple)
        self.layout_thickness = if cx.editor.config().gradient_borders.enable {
            cx.editor.config().gradient_borders.thickness as u16
        } else {
            cx.editor.config().notifications.border.width as u16
        };
        self.layout_rounded = cx.editor.config().rounded_corners
            || cx.editor.config().notifications.border.radius > 0;
        self.layout_padding = cx.editor.config().notifications.padding;

        // No continuous redraws or fading logic – redraws only on add and timeout/dismiss.

        if self.notifications.is_empty() {
            return;
        }

        let config = &cx.editor.config().notifications;
        self.calculate_notification_areas(area, config);

        // Render notifications in reverse order so newer ones appear on top
        // Clone the notifications to avoid borrowing issues
        let notifications_to_render: Vec<_> = self.notifications.iter().cloned().collect();
        for item in notifications_to_render.iter().rev() {
            self.render_notification(item, surface, cx);
        }
    }
}

impl Default for NotificationPopup {
    fn default() -> Self {
        Self::new()
    }
}
