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

#[derive(Default, Clone)]
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
    error: Option<String>,
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
            self.error = Some(format!("dashboard: cannot read {}", path.display()));
            self.path = Some(path);
            self.modified = modified;
            self.text.clear();
            return config.header.clone();
        };
        self.path = Some(path);
        self.modified = modified;
        self.text = text.clone();
        self.error = None;
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

    /// Returns a header file read error once, if one occurred.
    pub fn take_error(&mut self) -> Option<String> {
        self.header.error.take()
    }

    /// Draws the dashboard in `area` and returns the size of the chosen
    /// layout, so the caller can warn when it does not fit.
    pub fn render(
        &mut self,
        theme: &Theme,
        config: &DashboardConfig,
        area: Rect,
        surface: &mut Surface,
    ) -> (usize, usize) {
        if area.width == 0 || area.height == 0 {
            return (0, 0);
        }

        let header_text = self.header.resolve(config);
        let (header_vertical, header_horizontal) =
            if config.default_logo && header_text.trim().is_empty() {
                (
                    default_logo(theme, DashboardLayout::Vertical),
                    default_logo(theme, DashboardLayout::Horizontal),
                )
            } else {
                let header = text_block(&header_text, dashboard_style(theme, "header", "ui.text"));
                (header.clone(), header)
            };
        let subtitle = text_block(
            &config.subtitle,
            dashboard_style(theme, "subtitle", "ui.text.inactive"),
        );
        let items_min = items_block(config, theme, 0);
        let items_min_width = items_min.width();
        let items = items_block(config, theme, config.menu_width);
        let footer = text_block(
            &config.footer,
            dashboard_style(theme, "footer", "ui.text.inactive"),
        );

        let right_min = stack(
            [subtitle.clone(), items_min, footer.clone()],
            config.section_gap,
        );
        let gap = if header_horizontal.is_empty() || right_min.is_empty() {
            0
        } else {
            config.column_gap as usize
        };

        let vertical = stack(
            [
                header_vertical.clone(),
                subtitle.clone(),
                items.clone(),
                footer.clone(),
            ],
            config.section_gap,
        );
        let vertical_size = (vertical.width(), vertical.height());
        let horizontal_min_size = (
            header_horizontal.width() + gap + right_min.width(),
            header_horizontal.height().max(right_min.height()),
        );
        let layout = choose_layout(config.layout, area, vertical_size, horizontal_min_size);

        match layout {
            DashboardLayout::Vertical | DashboardLayout::Auto => {
                let x = match config.align {
                    DashboardAlign::Left => area.x,
                    DashboardAlign::Center => {
                        area.x + area.width.saturating_sub(vertical.width() as u16) / 2
                    }
                };
                let y = vertical_y(area, vertical.height(), config.vertical_align);
                draw(&vertical, x, y, area, surface);
                vertical_size
            }
            DashboardLayout::Horizontal => {
                let available =
                    (area.width as usize).saturating_sub(header_horizontal.width() + gap);
                let menu_width = (config.menu_width as usize).min(available.max(items_min_width));
                let menu_width = menu_width.min(u16::MAX as usize) as u16;
                let items = if menu_width == config.menu_width {
                    items
                } else {
                    items_block(config, theme, menu_width)
                };
                let right = stack([subtitle, items, footer], config.section_gap);
                let horizontal_size = (
                    header_horizontal.width() + gap + right.width(),
                    header_horizontal.height().max(right.height()),
                );
                let x = area.x + area.width.saturating_sub(horizontal_size.0 as u16) / 2;

                let y = vertical_y(
                    area,
                    header_horizontal.height().max(right.height()),
                    config.vertical_align,
                );
                draw(&header_horizontal, x, y, area, surface);

                let right_x = x + header_horizontal.width() as u16 + gap as u16;
                draw(&right, right_x, y, area, surface);
                horizontal_size
            }
        }
    }
}

fn choose_layout(
    layout: DashboardLayout,
    area: Rect,
    vertical: (usize, usize),
    horizontal: (usize, usize),
) -> DashboardLayout {
    if layout != DashboardLayout::Auto {
        return layout;
    }

    let fits = |(width, height): (usize, usize)| {
        width <= area.width as usize && height <= area.height as usize
    };

    match (fits(vertical), fits(horizontal)) {
        (true, false) => DashboardLayout::Vertical,
        (false, true) => DashboardLayout::Horizontal,
        (true, true) => {
            if area.width as usize >= area.height as usize * 3 {
                DashboardLayout::Horizontal
            } else {
                DashboardLayout::Vertical
            }
        }
        (false, false) => {
            let overflow = |(width, height): (usize, usize)| {
                width.saturating_sub(area.width as usize)
                    + height.saturating_sub(area.height as usize)
            };
            if overflow(vertical) <= overflow(horizontal) {
                DashboardLayout::Vertical
            } else {
                DashboardLayout::Horizontal
            }
        }
    }
}

fn items_block(config: &DashboardConfig, theme: &Theme, menu_width: u16) -> Block {
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
                    let width = (menu_width as usize).max(desc_width + key_width + 1);
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

const LOGO_R: [&str; 6] = [
    "██████╗ ",
    "██╔══██╗",
    "██████╔╝",
    "██╔══██╗",
    "██║  ██║",
    "╚═╝  ╚═╝",
];

const LOGO_H: [&str; 6] = [
    "██╗  ██╗",
    "██║  ██║",
    "███████║",
    "██╔══██║",
    "██║  ██║",
    "╚═╝  ╚═╝",
];

const LOGO_X: [&str; 6] = [
    "██╗  ██╗",
    "╚██╗██╔╝",
    " ╚███╔╝ ",
    " ██╔██╗ ",
    "██╔╝ ██╗",
    "╚═╝  ╚═╝",
];

const LOGO_DOTS: &str = "\
==            ,
xxxx,       .::
`xxxxxxxx,:::::
    `xx::::::`
   ,::::::^xxx;
.:::::`    ,;;;
:::`    ,;;;;;`
:eee,;;;;;;`
 ,;;;;;;ee,
;;;;;^eeeeeeee,
;;^       `eeee
`            --";

fn default_logo(theme: &Theme, layout: DashboardLayout) -> Block {
    if layout == DashboardLayout::Horizontal {
        dots_logo(theme)
    } else {
        vertical_logo(theme)
    }
}

fn dots_logo(theme: &Theme) -> Block {
    let header = dashboard_style(theme, "header", "ui.text");
    let icon = dashboard_style(theme, "icon", "ui.text.inactive");
    let subtitle = dashboard_style(theme, "subtitle", "ui.text.inactive");
    let key = dashboard_style(theme, "key", "ui.text.inactive");
    let styles = [
        theme
            .try_get_exact("ui.dashboard.logo.purple")
            .unwrap_or(header),
        theme
            .try_get_exact("ui.dashboard.logo.purple-dark")
            .unwrap_or(subtitle),
        theme
            .try_get_exact("ui.dashboard.logo.blue")
            .unwrap_or(icon),
        theme
            .try_get_exact("ui.dashboard.logo.blue-dark")
            .unwrap_or(key),
    ];

    Block {
        lines: LOGO_DOTS
            .lines()
            .map(|line| {
                let mut spans = Vec::new();
                push_ribbon(&mut spans, line.trim_end(), styles);
                spans
            })
            .collect(),
    }
}

fn vertical_logo(theme: &Theme) -> Block {
    let header = dashboard_style(theme, "header", "ui.text");
    let icon = dashboard_style(theme, "icon", "ui.text.inactive");
    let letters = letters_block(&[&LOGO_R, &LOGO_X, &LOGO_H, &LOGO_X], header);

    let mut lines = vec![vec![Span::styled(
        format!("{}  ランサム", "━".repeat(35)),
        header,
    )]];
    for line in letters.lines {
        let mut spans = vec![Span::raw(" ".repeat(5))];
        spans.extend(line);
        spans.push(Span::raw(" ".repeat(5)));
        lines.push(spans);
    }
    lines.push(vec![
        Span::raw(" ".repeat(4)),
        Span::styled("󰘬 ", icon),
        Span::styled("━".repeat(35), header),
        Span::raw(" ".repeat(4)),
    ]);

    Block { lines }
}

fn letters_block(letters: &[&[&str; 6]], header: Style) -> Block {
    let mut lines = Vec::new();
    for row in 0..LOGO_R.len() {
        let mut spans = Vec::new();
        for (index, letter) in letters.iter().enumerate() {
            if index > 0 {
                spans.push(Span::raw(" "));
            }
            push_chars(&mut spans, letter[row], header);
        }
        lines.push(spans);
    }
    Block { lines }
}

fn push_chars(line: &mut Line, text: &str, style: Style) {
    for ch in text.chars() {
        if ch == ' ' {
            line.push(Span::raw(" "));
        } else {
            line.push(Span::styled(ch.to_string(), style));
        }
    }
}

fn push_ribbon(line: &mut Line, text: &str, styles: [Style; 4]) {
    for ch in text.chars() {
        let style = match ch {
            'x' | 'e' => Some(styles[0]),
            '=' | '`' | '^' => Some(styles[1]),
            ':' | '.' => Some(styles[2]),
            ';' | ',' | '-' => Some(styles[3]),
            ' ' => None,
            _ => Some(styles[0]),
        };
        match style {
            Some(style) => line.push(Span::styled(ch.to_string(), style)),
            None => line.push(Span::raw(" ")),
        }
    }
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

    #[test]
    fn default_logo_uses_known_characters() {
        for ch in LOGO_DOTS.chars() {
            assert!(
                matches!(
                    ch,
                    'x' | 'e' | '=' | '`' | '^' | ':' | '.' | ';' | ',' | '-' | ' ' | '\n'
                ),
                "unexpected character in default logo: {ch:?}"
            );
        }
    }

    #[test]
    fn wordmark_has_expected_size() {
        let block = letters_block(&[&LOGO_R, &LOGO_X, &LOGO_H, &LOGO_X], Style::default());
        assert_eq!(block.height(), 6);
        assert_eq!(block.width(), 35);
    }

    #[test]
    fn dot_logo_has_expected_size() {
        assert_eq!(LOGO_DOTS.lines().count(), 12);
        assert!(
            LOGO_DOTS
                .lines()
                .map(|line| line.trim_end().len())
                .max()
                .unwrap()
                >= 15
        );
    }

    #[test]
    fn auto_layout_prefers_vertical_unless_clearly_wide() {
        let area = Rect::new(0, 0, 120, 60);
        assert_eq!(
            choose_layout(DashboardLayout::Auto, area, (45, 20), (74, 12)),
            DashboardLayout::Vertical
        );
    }

    #[test]
    fn auto_layout_prefers_horizontal_when_wide() {
        let area = Rect::new(0, 0, 120, 24);
        assert_eq!(
            choose_layout(DashboardLayout::Auto, area, (40, 20), (80, 12)),
            DashboardLayout::Horizontal
        );
    }

    #[test]
    fn auto_layout_prefers_vertical_when_tall() {
        let area = Rect::new(0, 0, 50, 40);
        assert_eq!(
            choose_layout(DashboardLayout::Auto, area, (40, 20), (45, 10)),
            DashboardLayout::Vertical
        );
    }

    #[test]
    fn auto_layout_uses_the_fitting_orientation() {
        let area = Rect::new(0, 0, 60, 20);
        assert_eq!(
            choose_layout(DashboardLayout::Auto, area, (50, 18), (80, 10)),
            DashboardLayout::Vertical
        );
        assert_eq!(
            choose_layout(DashboardLayout::Auto, area, (80, 18), (50, 10)),
            DashboardLayout::Horizontal
        );
    }

    #[test]
    fn auto_layout_picks_the_smallest_overflow() {
        let area = Rect::new(0, 0, 50, 12);
        assert_eq!(
            choose_layout(DashboardLayout::Auto, area, (30, 17), (51, 10)),
            DashboardLayout::Horizontal
        );
    }
}
