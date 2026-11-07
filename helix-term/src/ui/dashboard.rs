use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use helix_core::unicode::width::UnicodeWidthStr;
use helix_view::editor::{
    DashboardAlign, DashboardConfig, DashboardLayout, DashboardMenuStyle, DashboardVerticalAlign,
};
use helix_view::graphics::{Rect, Style};
use helix_view::Theme;
use tui::buffer::Buffer as Surface;
use tui::text::Span;

type Line = Vec<Span<'static>>;

#[derive(Default)]
struct Block {
    lines: Vec<Line>,
}

impl Block {
    fn width(&self) -> usize {
        self.lines
            .iter()
            .map(|line| line_width(line))
            .max()
            .unwrap_or(0)
    }

    fn height(&self) -> usize {
        self.lines.len()
    }

    fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

#[derive(Default)]
struct HeaderCache {
    path: Option<PathBuf>,
    modified: Option<SystemTime>,
    text: String,
}

impl HeaderCache {
    fn resolve(&mut self, config: &DashboardConfig) -> String {
        let Some(path) = config.header_file.as_deref() else {
            return config.header.clone();
        };
        let path = helix_stdx::path::expand_tilde(Path::new(path)).into_owned();
        let modified = fs::metadata(&path).and_then(|meta| meta.modified()).ok();

        if self.path.as_ref() == Some(&path) && self.modified == modified {
            return self.text.clone();
        }

        let Some(text) = fs::read_to_string(&path).ok() else {
            return config.header.clone();
        };
        self.path = Some(path);
        self.modified = modified;
        self.text = text.clone();
        text
    }
}

#[derive(Default)]
pub struct Renderer {
    header: HeaderCache,
}

impl Renderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws the dashboard in `area` and returns the layout width if it does
    /// not fit, so the caller can warn.
    pub fn render(
        &mut self,
        theme: &Theme,
        config: &DashboardConfig,
        area: Rect,
        surface: &mut Surface,
    ) -> Option<u16> {
        if area.width == 0 || area.height == 0 {
            return None;
        }

        let header = text_block(
            &self.header.resolve(config),
            dashboard_style(theme, "header", "ui.text"),
        );
        let subtitle = text_block(
            &config.subtitle,
            dashboard_style(theme, "subtitle", "ui.text.inactive"),
        );
        let items = items_block(config, theme);
        let footer = text_block(
            &config.footer,
            dashboard_style(theme, "footer", "ui.text.inactive"),
        );

        let layout_width = match config.layout {
            DashboardLayout::Vertical => {
                let block = stack([header, subtitle, items, footer], config.section_gap);
                let x = match config.align {
                    DashboardAlign::Left => area.x,
                    DashboardAlign::Center => {
                        area.x + area.width.saturating_sub(block.width() as u16) / 2
                    }
                };
                let y = vertical_y(area, block.height(), config.vertical_align);
                draw(&block, x, y, area, surface);
                block.width()
            }
            DashboardLayout::Horizontal => {
                let right = stack([subtitle, items, footer], config.section_gap);
                let gap = if header.is_empty() || right.is_empty() {
                    0
                } else {
                    config.column_gap as usize
                };
                let total = header.width() + gap + right.width();
                let x = area.x + area.width.saturating_sub(total as u16) / 2;

                let header_y = vertical_y(area, header.height(), config.vertical_align);
                draw(&header, x, header_y, area, surface);

                let right_x = x + header.width() as u16 + gap as u16;
                let right_y = vertical_y(area, right.height(), config.vertical_align);
                draw(&right, right_x, right_y, area, surface);
                total
            }
        };

        (layout_width > area.width as usize).then_some(layout_width as u16)
    }
}

fn items_block(config: &DashboardConfig, theme: &Theme) -> Block {
    let item_style = dashboard_style(theme, "item", "ui.text");
    let icon_style = dashboard_style(theme, "icon", "ui.text.inactive");
    let key_style = dashboard_style(theme, "key", "ui.text.inactive");

    let icon_width = config
        .items
        .iter()
        .filter_map(|item| item.icon.as_deref())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let has_keys = config
        .items
        .iter()
        .any(|item| item.key.as_deref().is_some_and(|key| !key.is_empty()));

    let mut lines = Vec::new();
    for (index, item) in config.items.iter().enumerate() {
        if index > 0 {
            for _ in 0..config.item_gap {
                lines.push(Vec::new());
            }
        }

        let key = item.key.as_deref().unwrap_or("");
        let icon = item.icon.as_deref().unwrap_or("");
        let mut spans = Vec::new();

        match config.menu_style {
            DashboardMenuStyle::KeyLeft => {
                if has_keys {
                    spans.push(Span::styled(pad(key, config.key_width as usize), key_style));
                    spans.push(Span::raw(" "));
                }
                spans.extend(icon_spans(icon, icon_width, icon_style));
                spans.push(Span::styled(item.desc.clone(), item_style));
            }
            DashboardMenuStyle::SpaceBetween => {
                spans.extend(icon_spans(icon, icon_width, icon_style));
                spans.push(Span::styled(item.desc.clone(), item_style));

                if !key.is_empty() {
                    let desc_width = line_width(&spans);
                    let key_width = key.width();
                    let width = (config.menu_width as usize).max(desc_width + key_width + 1);
                    spans.push(Span::raw(" ".repeat(width - desc_width - key_width)));
                    spans.push(Span::styled(key.to_string(), key_style));
                }
            }
        }
        lines.push(spans);
    }

    Block { lines }
}

fn icon_spans(icon: &str, icon_width: usize, style: Style) -> Vec<Span<'static>> {
    if icon_width == 0 {
        return Vec::new();
    }
    vec![
        Span::styled(icon.to_string(), style),
        Span::raw(" ".repeat(icon_width - icon.width() + 1)),
    ]
}

fn text_block(text: &str, style: Style) -> Block {
    let text = text.replace("%{version}", env!("CARGO_PKG_VERSION"));
    Block {
        lines: text
            .lines()
            .map(|line| vec![Span::styled(line.to_string(), style)])
            .collect(),
    }
}

fn stack(blocks: impl IntoIterator<Item = Block>, gap: u16) -> Block {
    let mut lines = Vec::new();
    let mut empty = true;
    for block in blocks {
        if block.is_empty() {
            continue;
        }
        if !empty {
            for _ in 0..gap {
                lines.push(Vec::new());
            }
        }
        lines.extend(block.lines);
        empty = false;
    }
    Block { lines }
}

fn vertical_y(area: Rect, height: usize, align: DashboardVerticalAlign) -> u16 {
    match align {
        DashboardVerticalAlign::Top => area.y,
        DashboardVerticalAlign::Center => area.y + area.height.saturating_sub(height as u16) / 2,
    }
}

fn draw(block: &Block, x: u16, y: u16, area: Rect, surface: &mut Surface) {
    let bottom = area.y.saturating_add(area.height);
    let right = area.right();
    for (row, line) in block.lines.iter().enumerate() {
        let row_y = y.saturating_add(row as u16);
        if row_y >= bottom {
            break;
        }
        let mut col = x;
        for span in line {
            if col >= right {
                break;
            }
            let remaining = right.saturating_sub(col) as usize;
            let (next, _) = surface.set_stringn(col, row_y, &span.content, remaining, span.style);
            col = next;
        }
    }
}

fn line_width(line: &[Span]) -> usize {
    line.iter().map(|span| span.content.width()).sum()
}

fn pad(text: &str, width: usize) -> String {
    let text_width = text.width();
    if text_width >= width {
        text.to_string()
    } else {
        format!("{text}{}", " ".repeat(width - text_width))
    }
}

fn dashboard_style(theme: &Theme, name: &str, fallback: &str) -> Style {
    theme
        .try_get(&format!("ui.dashboard.{name}"))
        .unwrap_or_else(|| theme.get(fallback))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> Line {
        vec![Span::raw(text.to_string())]
    }

    #[test]
    fn pad_uses_display_width() {
        assert_eq!(pad("ab", 4), "ab  ");
        assert_eq!(pad("日本", 5), "日本 ");
        assert_eq!(pad("toolong", 3), "toolong");
    }

    #[test]
    fn stack_skips_empty_blocks_and_inserts_gaps() {
        let blocks = [
            Block {
                lines: vec![line("a")],
            },
            Block::default(),
            Block {
                lines: vec![line("b"), line("c")],
            },
        ];
        let block = stack(blocks, 1);
        assert_eq!(block.height(), 4);
        assert_eq!(block.width(), 1);
    }

    #[test]
    fn text_block_expands_version() {
        let block = text_block("helix %{version}", Style::default());
        assert_eq!(
            block.lines[0][0].content.as_ref(),
            format!("helix {}", env!("CARGO_PKG_VERSION"))
        );
    }
}
