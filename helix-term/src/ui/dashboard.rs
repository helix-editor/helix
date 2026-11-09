use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use helix_core::unicode::width::UnicodeWidthStr;
use helix_view::editor::{
    DashboardAlign, DashboardConfig, DashboardLayout, DashboardMenuStyle, DashboardVerticalAlign,
};
use helix_view::graphics::{Modifier, Rect, Style};
use helix_view::input::KeyEvent;
use helix_view::{Editor, Theme};
use tui::buffer::Buffer as Surface;
use tui::text::Span;

type Line = Vec<Span<'static>>;

const HEADER_CHECK_INTERVAL: Duration = Duration::from_millis(500);

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
    checked_at: Option<Instant>,
    text: String,
    error: Option<String>,
}

impl HeaderCache {
    fn resolve(&mut self, config: &DashboardConfig) -> String {
        let Some(path) = config.header_file.as_deref() else {
            return config.header.clone();
        };
        let path = helix_stdx::path::expand_tilde(Path::new(path)).into_owned();
        let now = Instant::now();
        if self.path.as_ref() == Some(&path)
            && self
                .checked_at
                .is_some_and(|checked_at| now.duration_since(checked_at) < HEADER_CHECK_INTERVAL)
        {
            return self.text.clone();
        }
        self.checked_at = Some(now);
        let modified = fs::metadata(&path).and_then(|meta| meta.modified()).ok();

        if self.path.as_ref() == Some(&path) && self.modified == modified {
            return self.text.clone();
        }

        let Some(text) = fs::read_to_string(&path).ok() else {
            self.error = Some(format!("dashboard: cannot read {}", path.display()));
            self.path = Some(path);
            self.modified = modified;
            // Keep the inline fallback in the cache: the next frame hits the
            // `path == self.path && modified == self.modified` branch and must
            // keep showing `header` instead of an empty cache entry.
            self.text = config.header.clone();
            return self.text.clone();
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
    key_signature: Vec<(Option<String>, Option<String>)>,
    keymap: Vec<(KeyEvent, String)>,
}

impl Renderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve a dashboard key without reparsing every configured key on each
    /// keypress.
    pub fn action(&mut self, editor: &Editor, event: KeyEvent) -> Option<String> {
        let items = &editor.config().dashboard.items;
        let same = items.len() == self.key_signature.len()
            && items
                .iter()
                .zip(&self.key_signature)
                .all(|(item, (key, action))| item.key == *key && item.action == *action);
        if !same {
            self.key_signature = items
                .iter()
                .map(|item| (item.key.clone(), item.action.clone()))
                .collect();
            self.keymap.clear();
            for item in items {
                let (Some(key), Some(action)) = (item.key.as_deref(), item.action.as_ref()) else {
                    continue;
                };
                let Ok(mut keys) = helix_view::input::parse_macro(key) else {
                    continue;
                };
                if keys.len() == 1 {
                    if let Some(key) = keys.pop() {
                        self.keymap.push((key, action.clone()));
                    }
                }
            }
        }
        self.keymap
            .iter()
            .find(|(key, _)| *key == event)
            .map(|(_, action)| action.clone())
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
        let vertical_gap = config.menu_gap.for_layout(DashboardLayout::Vertical);
        let horizontal_gap = config.menu_gap.for_layout(DashboardLayout::Horizontal);
        let items_vertical = items_block(config, theme, config.menu_width, vertical_gap);
        let items_horizontal_min = items_block(config, theme, 0, horizontal_gap);
        let items_min_width = items_horizontal_min.width();
        let footer = text_block(
            &config.footer,
            dashboard_style(theme, "footer", "ui.text.inactive"),
        );

        let right_min = stack(
            [subtitle.clone(), items_horizontal_min, footer.clone()],
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
                items_vertical,
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
                let items = items_block(config, theme, menu_width, horizontal_gap);
                let right = stack([subtitle, items, footer], config.section_gap);
                let horizontal_size = (
                    header_horizontal.width() + gap + right.width(),
                    header_horizontal.height().max(right.height()),
                );
                let x = area.x + area.width.saturating_sub(horizontal_size.0 as u16) / 2;

                let height = header_horizontal.height().max(right.height());
                let y = vertical_y(area, height, config.vertical_align);
                let header_y = y + (height - header_horizontal.height()).div_ceil(2) as u16;
                let right_y = y + (height - right.height()).div_ceil(2) as u16;
                draw(&header_horizontal, x, header_y, area, surface);

                let right_x = x + header_horizontal.width() as u16 + gap as u16;
                draw(&right, right_x, right_y, area, surface);
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

fn items_block(config: &DashboardConfig, theme: &Theme, menu_width: u16, menu_gap: u16) -> Block {
    // Icons and keys carry the accent colors; descriptions stay readable in
    // the normal text color. Themes can override each role with
    // `ui.dashboard.*` scopes.
    let icon_style = dashboard_style(theme, "icon", "ui.text");
    let item_style = dashboard_style(theme, "item", "ui.text");
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

    let space_between = matches!(config.menu_style, DashboardMenuStyle::SpaceBetween);
    let (max_left, max_key) = if space_between {
        config
            .items
            .iter()
            .fold((0, 0), |(max_left, max_key), item| {
                let icon_column = if icon_width > 0 { icon_width + 1 } else { 0 };
                (
                    max_left.max(icon_column + item.desc.width()),
                    max_key.max(item.key.as_deref().unwrap_or("").width()),
                )
            })
    } else {
        (0, 0)
    };
    let menu_width = if space_between {
        let dynamic = max_left + max_key + menu_gap as usize;
        let requested = menu_width as usize;
        let width = if requested > 0 { requested } else { dynamic };
        width.max(max_left + max_key + 1)
    } else {
        menu_width as usize
    };

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
                    let filler = menu_width.saturating_sub(desc_width + key_width).max(1);
                    spans.push(Span::raw(" ".repeat(filler)));
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

const LOGO_H: [&str; 6] = [
    "██╗  ██╗",
    "██║  ██║",
    "███████║",
    "██╔══██║",
    "██║  ██║",
    "╚═╝  ╚═╝",
];

const LOGO_E: [&str; 6] = [
    "███████╗",
    "██╔════╝",
    "█████╗  ",
    "██╔══╝  ",
    "███████╗",
    "╚══════╝",
];

const LOGO_L: [&str; 6] = [
    "██╗     ",
    "██║     ",
    "██║     ",
    "██║     ",
    "███████╗",
    "╚══════╝",
];

const LOGO_I: [&str; 6] = ["██╗", "██║", "██║", "██║", "██║", "╚═╝"];

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
        horizontal_logo(theme)
    } else {
        vertical_logo(theme)
    }
}

fn horizontal_logo(theme: &Theme) -> Block {
    let mut block = dots_logo(theme);
    let width = block.width();
    let caption = "· ʜᴇʟɪx ·";
    let caption_width = caption.width();
    let pad = width.saturating_sub(caption_width) / 2;
    block.lines.push(vec![
        Span::raw(" ".repeat(pad)),
        Span::styled(
            caption,
            dashboard_style(theme, "subtitle", "ui.text.inactive"),
        ),
        Span::raw(" ".repeat(width.saturating_sub(pad + caption_width))),
    ]);
    block
}

fn dots_logo(theme: &Theme) -> Block {
    // Reuse the mode-chip accent colors instead of custom dashboard scopes:
    // generated themes (matugen/Noctalia) keep the statusline scopes but may
    // drop anything dashboard-specific on the next wallpaper change.
    let primary = accent_style(theme, "ui.statusline.normal", "ui.text");
    let secondary = accent_style(theme, "ui.statusline.select", "ui.text.inactive");
    let tertiary = accent_style(theme, "ui.statusline.insert", "ui.text.inactive");
    let muted = theme.get("ui.text.inactive");
    let styles = [primary, secondary, tertiary, muted];

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
    let dots = dots_logo(theme);
    let helix = letters_block(&[&LOGO_H, &LOGO_E, &LOGO_L, &LOGO_I, &LOGO_X], header);

    let dots_width = dots.width();
    let helix_width = helix.width();
    let gap = 4;
    let height = dots.height().max(helix.height());
    let dots_offset = (height - dots.height()) / 2;
    let helix_offset = (height - helix.height()) / 2;

    let mut lines = Vec::new();
    for row in 0..height {
        let mut spans = Vec::new();
        if let Some(line) = row.checked_sub(dots_offset).and_then(|i| dots.lines.get(i)) {
            let mut width = 0;
            for span in line {
                width += span.content.width();
                spans.push(span.clone());
            }
            if width < dots_width {
                spans.push(Span::raw(" ".repeat(dots_width - width)));
            }
        } else {
            spans.push(Span::raw(" ".repeat(dots_width)));
        }
        spans.push(Span::raw(" ".repeat(gap)));
        if let Some(line) = row
            .checked_sub(helix_offset)
            .and_then(|i| helix.lines.get(i))
        {
            spans.extend(line.iter().cloned());
        } else {
            spans.push(Span::raw(" ".repeat(helix_width)));
        }
        lines.push(spans);
    }

    Block { lines }
}

fn letters_block(letters: &[&[&str; 6]], header: Style) -> Block {
    let mut lines = Vec::new();
    for row in 0..6 {
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
    let blocks: Vec<Block> = blocks
        .into_iter()
        .filter(|block| !block.is_empty())
        .collect();
    let width = blocks.iter().map(Block::width).max().unwrap_or(0);

    let mut lines = Vec::new();
    for (index, block) in blocks.into_iter().enumerate() {
        if index > 0 {
            for _ in 0..gap {
                lines.push(Vec::new());
            }
        }
        let pad = width.saturating_sub(block.width()) / 2;
        for line in block.lines {
            let line_width = line_width(&line);
            let mut new_line = Vec::new();
            if pad > 0 {
                new_line.push(Span::raw(" ".repeat(pad)));
            }
            new_line.extend(line);
            let trailing = width.saturating_sub(pad + line_width);
            if trailing > 0 {
                new_line.push(Span::raw(" ".repeat(trailing)));
            }
            lines.push(new_line);
        }
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

/// Use a theme's existing accent color, without any dashboard-specific scope.
///
/// Mode chips (`ui.statusline.normal`/`.select`/`.insert`) are defined by every
/// theme and carry the primary/secondary/tertiary colors in their background.
fn accent_style(theme: &Theme, scope: &str, fallback: &str) -> Style {
    theme
        .try_get(scope)
        .and_then(|style| style.bg.or(style.fg))
        .map(|color| Style::default().fg(color))
        .unwrap_or_else(|| theme.get(fallback))
}

fn dashboard_style(theme: &Theme, name: &str, fallback: &str) -> Style {
    if let Some(style) = theme.try_get_exact(&format!("ui.dashboard.{name}")) {
        return style;
    }
    match name {
        "header" => {
            accent_style(theme, "ui.statusline.normal", fallback).add_modifier(Modifier::BOLD)
        }
        "icon" => accent_style(theme, "ui.statusline.select", fallback),
        _ => theme.get(fallback),
    }
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
    fn missing_header_file_keeps_inline_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let config = DashboardConfig {
            header: "fallback".to_string(),
            header_file: Some(dir.path().join("missing.txt").display().to_string()),
            ..Default::default()
        };
        let mut cache = HeaderCache::default();

        assert_eq!(cache.resolve(&config), "fallback");
        // A cached miss must keep returning the inline header, not an empty
        // cached string.
        assert_eq!(cache.resolve(&config), "fallback");
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
    fn helix_wordmark_has_expected_size() {
        let block = letters_block(
            &[&LOGO_H, &LOGO_E, &LOGO_L, &LOGO_I, &LOGO_X],
            Style::default(),
        );
        assert_eq!(block.height(), 6);
        assert_eq!(block.width(), 39);
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
