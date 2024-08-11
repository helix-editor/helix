use crate::{
    commands::Open,
    compositor::{Callback, Component, Context, Event, EventResult},
    ctrl, key,
};
use tui::{
    buffer::Buffer as Surface,
    widgets::{Block, BorderType, Widget},
};

use helix_core::Position;
use helix_view::{
    graphics::{Margin, Rect},
    input::{MouseEvent, MouseEventKind},
    Editor,
};

const MIN_HEIGHT: u16 = 6;
const MAX_HEIGHT: u16 = 26;
const MAX_WIDTH: u16 = 120;

struct RenderInfo {
    area: Rect,
    child_height: u16,
    render_borders: bool,
    is_menu: bool,
}

// TODO: share logic with Menu, it's essentially Popup(render_fn), but render fn needs to return
// a width/height hint. maybe Popup(Box<Component>)

pub struct Popup<T: Component> {
    contents: T,
    position: Option<Position>,
    area: Rect,
    position_bias: Open,
    scroll_half_pages: usize,
    auto_close: bool,
    ignore_escape_key: bool,
    id: &'static str,
    has_scrollbar: bool,
}

impl<T: Component> Popup<T> {
    pub fn new(id: &'static str, contents: T) -> Self {
        Self {
            contents,
            position: None,
            position_bias: Open::Below,
            area: Rect::new(0, 0, 0, 0),
            scroll_half_pages: 0,
            auto_close: false,
            ignore_escape_key: false,
            id,
            has_scrollbar: true,
        }
    }

    /// Set the anchor position next to which the popup should be drawn.
    ///
    /// Note that this is not the position of the top-left corner of the rendered popup itself,
    /// but rather the screen-space position of the information to which the popup refers.
    pub fn position(mut self, pos: Option<Position>) -> Self {
        self.position = pos;
        self
    }

    pub fn get_position(&self) -> Option<Position> {
        self.position
    }

    /// Set the popup to prefer to render above or below the anchor position.
    ///
    /// This preference will be ignored if the viewport doesn't have enough space in the
    /// chosen direction.
    pub fn position_bias(mut self, bias: Open) -> Self {
        self.position_bias = bias;
        self
    }

    pub fn auto_close(mut self, auto_close: bool) -> Self {
        self.auto_close = auto_close;
        self
    }

    /// Ignores an escape keypress event, letting the outer layer
    /// (usually the editor) handle it. This is useful for popups
    /// in insert mode like completion and signature help where
    /// the popup is closed on the mode change from insert to normal
    /// which is done with the escape key. Otherwise the popup consumes
    /// the escape key event and closes it, and an additional escape
    /// would be required to exit insert mode.
    pub fn ignore_escape_key(mut self, ignore: bool) -> Self {
        self.ignore_escape_key = ignore;
        self
    }

    pub fn scroll_half_page_down(&mut self) {
        self.scroll_half_pages += 1;
    }

    pub fn scroll_half_page_up(&mut self) {
        self.scroll_half_pages = self.scroll_half_pages.saturating_sub(1);
    }

    /// Toggles the Popup's scrollbar.
    /// Consider disabling the scrollbar in case the child
    /// already has its own.
    pub fn with_scrollbar(mut self, enable_scrollbar: bool) -> Self {
        self.has_scrollbar = enable_scrollbar;
        self
    }

    pub fn contents(&self) -> &T {
        &self.contents
    }

    pub fn contents_mut(&mut self) -> &mut T {
        &mut self.contents
    }

    pub fn area(&mut self, viewport: Rect, editor: &Editor) -> Rect {
        self.render_info(viewport, editor).area
    }

    fn render_info(&mut self, viewport: Rect, editor: &Editor) -> RenderInfo {
        let mut position = editor.cursor().0.unwrap_or_default();
        if let Some(old_position) = self
            .position
            .filter(|old_position| old_position.row == position.row)
        {
            position = old_position;
        } else {
            self.position = Some(position);
        }

        let is_menu = self
            .contents
            .type_name()
            .starts_with("helix_term::ui::menu::Menu");

        let mut render_borders = if is_menu {
            editor.menu_border()
        } else {
            editor.popup_border()
        };

        // -- make sure frame doesn't stick out of bounds
        let mut rel_x = position.col as u16;
        let mut rel_y = position.row as u16;

        // if there's a orientation preference, use that
        // if we're on the top part of the screen, do below
        // if we're on the bottom part, do above
        let can_put_below = viewport.height > rel_y + MIN_HEIGHT;
        let can_put_above = rel_y.checked_sub(MIN_HEIGHT).is_some();
        let final_pos = match self.position_bias {
            Open::Below => match can_put_below {
                true => Open::Below,
                false => Open::Above,
            },
            Open::Above => match can_put_above {
                true => Open::Above,
                false => Open::Below,
            },
        };

        // compute maximum space available for child
        let mut max_height = match final_pos {
            Open::Above => rel_y,
            Open::Below => viewport.height.saturating_sub(1 + rel_y),
        };
        max_height = max_height.min(MAX_HEIGHT);
        let mut max_width = viewport.width.saturating_sub(2).min(MAX_WIDTH);
        render_borders = render_borders && max_height > 3 && max_width > 3;
        if render_borders {
            max_width -= 2;
            max_height -= 2;
        }

        // compute required child size and reclamp
        let (mut width, child_height) = self
            .contents
            .required_size((max_width, max_height))
            .expect("Component needs required_size implemented in order to be embedded in a popup");

        width = width.min(MAX_WIDTH);
        let height = if render_borders {
            (child_height + 2).min(MAX_HEIGHT)
        } else {
            child_height.min(MAX_HEIGHT)
        };
        if render_borders {
            width += 2;
        }
        if viewport.width <= rel_x + width + 2 {
            rel_x = viewport.width.saturating_sub(width + 2);
            width = viewport.width.saturating_sub(rel_x + 2)
        }

        let area = match final_pos {
            Open::Above => {
                rel_y = rel_y.saturating_sub(height);
                Rect::new(rel_x, rel_y, width, position.row as u16 - rel_y)
            }
            Open::Below => {
                rel_y += 1;
                let y_max = viewport.bottom().min(height + rel_y);
                Rect::new(rel_x, rel_y, width, y_max - rel_y)
            }
        };
        RenderInfo {
            area,
            child_height,
            render_borders,
            is_menu,
        }
    }

    fn handle_mouse_event(
        &mut self,
        &MouseEvent {
            kind,
            column: x,
            row: y,
            ..
        }: &MouseEvent,
    ) -> EventResult {
        if self.auto_close && matches!(kind, MouseEventKind::Down(_)) {
            let close_fn: Callback = Box::new(|compositor, _| {
                // remove the layer
                compositor.remove(self.id.as_ref());
            });

            return EventResult::Ignored(Some(close_fn));
        }

        let mouse_is_within_popup = x >= self.area.left()
            && x < self.area.right()
            && y >= self.area.top()
            && y < self.area.bottom();

        if !mouse_is_within_popup {
            return EventResult::Ignored(None);
        }

        match kind {
            MouseEventKind::ScrollDown if self.has_scrollbar => {
                self.scroll_half_page_down();
                EventResult::Consumed(None)
            }
            MouseEventKind::ScrollUp if self.has_scrollbar => {
                self.scroll_half_page_up();
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored(None),
        }
    }
}

impl<T: Component> Component for Popup<T> {
    fn handle_event(&mut self, event: &Event, cx: &mut Context) -> EventResult {
        let key = match event {
            Event::Key(event) => *event,
            Event::Mouse(event) => return self.handle_mouse_event(event),
            Event::Resize(_, _) => {
                // TODO: calculate inner area, call component's handle_event with that area
                return EventResult::Ignored(None);
            }
            _ => return EventResult::Ignored(None),
        };

        if key!(Esc) == key && self.ignore_escape_key {
            return EventResult::Ignored(None);
        }

        let close_fn: Callback = Box::new(|compositor, _| {
            // remove the layer
            compositor.remove(self.id.as_ref());
        });

        match key {
            // esc or ctrl-c aborts the completion and closes the menu
            key!(Esc) | ctrl!('c') => {
                let _ = self.contents.handle_event(event, cx);
                EventResult::Consumed(Some(close_fn))
            }
            ctrl!('d') => {
                self.scroll_half_page_down();
                EventResult::Consumed(None)
            }
            ctrl!('u') => {
                self.scroll_half_page_up();
                EventResult::Consumed(None)
            }
            _ => {
                let contents_event_result = self.contents.handle_event(event, cx);

                if self.auto_close {
                    if let EventResult::Ignored(None) = contents_event_result {
                        return EventResult::Ignored(Some(close_fn));
                    }
                }

                contents_event_result
            }
        }
        // for some events, we want to process them but send ignore, specifically all input except
        // tab/enter/ctrl-k or whatever will confirm the selection/ ctrl-n/ctrl-p for scroll.
    }

    fn render(&mut self, viewport: Rect, surface: &mut Surface, cx: &mut Context) {
        let RenderInfo {
            area,
            child_height,
            render_borders,
            is_menu,
        } = self.render_info(viewport, cx.editor);
        self.area = area;

        // clear area
        let background = if is_menu {
            // TODO: consistently style menu
            cx.editor
                .theme
                .try_get("ui.menu")
                .unwrap_or_else(|| cx.editor.theme.get("ui.text"))
        } else {
            cx.editor.theme.get("ui.popup")
        };
        surface.clear_with(area, background);

        let mut inner = area;
        if render_borders {
            let border_type = BorderType::new(cx.editor.config().rounded_corners);
            inner = area.inner(Margin::all(1));
            Widget::render(Block::bordered().border_type(border_type), area, surface);
        }
        let border = usize::from(render_borders);

        let max_offset = child_height.saturating_sub(inner.height) as usize;
        let half_page_size = (inner.height / 2) as usize;
        let scroll = max_offset.min(self.scroll_half_pages * half_page_size);
        if half_page_size > 0 {
            self.scroll_half_pages = scroll / half_page_size;
        }
        cx.scroll = Some(scroll);
        self.contents.render(inner, surface, cx);

        // render scrollbar if contents do not fit
        if self.has_scrollbar {
            let win_height = inner.height as usize;
            let len = child_height as usize;
            let fits = len <= win_height;
            let scroll_style = cx.editor.theme.get("ui.menu.scroll");

            if !fits {
                let scroll_height = win_height.pow(2).div_ceil(len).min(win_height);
                let scroll_line = (win_height - scroll_height) * scroll
                    / std::cmp::max(1, len.saturating_sub(win_height));

                let mut cell;
                for i in 0..win_height {
                    cell =
                        &mut surface[(inner.right() - 1 + border as u16, inner.top() + i as u16)];

                    let half_block = if render_borders { "▌" } else { "▐" };

                    if scroll_line <= i && i < scroll_line + scroll_height {
                        // Draw scroll thumb
                        cell.set_symbol(half_block);
                        cell.set_fg(scroll_style.fg.unwrap_or(helix_view::theme::Color::Reset));
                    } else if !render_borders {
                        // Draw scroll track
                        cell.set_symbol(half_block);
                        cell.set_fg(scroll_style.bg.unwrap_or(helix_view::theme::Color::Reset));
                    }
                }
            }
        }
    }

    fn id(&self) -> Option<&'static str> {
        Some(self.id)
    }
}
