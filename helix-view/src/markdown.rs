//! Markdown image layout shared by the view and the terminal renderer.
//!
//! Images need virtual lines reserved after their anchor line so following
//! text is reflowed instead of covered. This module computes that layout in
//! `helix-view`, where both cursor positioning and rendering can use it.

use std::cell::Cell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use helix_core::doc_formatter::FormattedGrapheme;
use helix_core::text_annotations::LineAnnotation;
use helix_core::Position;

use crate::{Document, View};

const MAX_HEIGHT_PERCENT: f32 = 0.5;

#[derive(Debug, Clone)]
pub struct ImageLayout {
    pub path: PathBuf,
    pub char_idx: usize,
    pub line: usize,
    pub rows: u16,
    pub cols: u16,
    pub offset_rows: u16,
}

/// Compute inline image layouts for the visible section of a markdown buffer.
pub fn visible_images(doc: &Document, view: &View) -> Vec<ImageLayout> {
    let config = doc.config.load();
    if !config.markdown.images || doc.language_name() != Some("markdown") || !kitty_supported() {
        return Vec::new();
    }
    let Some(source) = doc.path().map(Path::to_path_buf) else {
        return Vec::new();
    };

    let vault = config
        .obsidian
        .enable
        .then(|| config.obsidian.vault.as_deref())
        .flatten()
        .map(|path| helix_stdx::path::expand_tilde(Path::new(path)).into_owned());

    let text = doc.text().slice(..);
    if text.len_lines() == 0 {
        return Vec::new();
    }
    let inner = view.inner_area(doc);
    if inner.width == 0 || inner.height == 0 {
        return Vec::new();
    }

    let view_offset = doc.view_offset(view.id);
    let start_line = text.char_to_line(view_offset.anchor.min(text.len_chars()));
    let end_line = (start_line + inner.height as usize + 2).min(text.len_lines());
    let (cell_width, cell_height) = cell_size();
    let max_rows = ((inner.height as f32 * MAX_HEIGHT_PERCENT).floor() as u16).max(2);
    let max_cols = inner.width;

    let mut images = Vec::new();
    let mut line_rows: HashMap<usize, u16> = HashMap::new();

    for line_idx in start_line..end_line {
        let line_start = text.line_to_char(line_idx);
        let line = text.line(line_idx).to_string();
        for (column, target) in image_links(&line) {
            let Some(path) = resolve_image(&source, vault.as_deref(), &target) else {
                continue;
            };
            let Some((width_px, height_px)) = image_dimensions(&path) else {
                continue;
            };
            let (cols, rows) = fit_cells(
                width_px,
                height_px,
                cell_width,
                cell_height,
                max_cols,
                max_rows,
            );
            let offset_rows = *line_rows.get(&line_idx).unwrap_or(&0);
            line_rows.insert(line_idx, offset_rows.saturating_add(rows));
            images.push(ImageLayout {
                path,
                char_idx: line_start + column,
                line: line_idx,
                rows,
                cols,
                offset_rows,
            });
        }
    }

    images
}

/// Build a virtual-line annotation that reserves space for inline images.
pub fn image_annotation(doc: &Document, view: &View) -> Option<Box<dyn LineAnnotation>> {
    let images = visible_images(doc, view);
    if images.is_empty() {
        return None;
    }
    Some(Box::new(ImageLineAnnotation::new(images)))
}

struct ImageLineAnnotation {
    anchors: Vec<(usize, usize, u16)>,
    next: Cell<usize>,
    pending: Cell<Option<(usize, u16)>>,
}

impl ImageLineAnnotation {
    fn new(images: Vec<ImageLayout>) -> Self {
        Self {
            anchors: images
                .into_iter()
                .map(|image| (image.char_idx, image.line, image.rows))
                .collect(),
            next: Cell::new(0),
            pending: Cell::new(None),
        }
    }
}

impl LineAnnotation for ImageLineAnnotation {
    fn reset_pos(&mut self, char_idx: usize) -> usize {
        let index = self
            .anchors
            .partition_point(|(anchor, _, _)| *anchor < char_idx);
        self.next.set(index);
        self.pending.set(None);
        self.anchors
            .get(index)
            .map(|(anchor, _, _)| *anchor)
            .unwrap_or(usize::MAX)
    }

    fn process_anchor(&mut self, grapheme: &FormattedGrapheme) -> usize {
        let index = self.next.get();
        if let Some((anchor, line, rows)) = self.anchors.get(index) {
            if *anchor == grapheme.char_idx {
                let (pending_line, pending_rows) = self.pending.get().unwrap_or((*line, 0));
                self.pending
                    .set(Some((pending_line, pending_rows.saturating_add(*rows))));
                self.next.set(index + 1);
            }
        }
        self.anchors
            .get(self.next.get())
            .map(|(anchor, _, _)| *anchor)
            .unwrap_or(usize::MAX)
    }

    fn insert_virtual_lines(
        &mut self,
        _line_end_char_idx: usize,
        _line_end_visual_pos: Position,
        doc_line: usize,
    ) -> Position {
        if let Some((line, rows)) = self.pending.get() {
            if line == doc_line {
                self.pending.set(None);
                return Position::new(rows.saturating_sub(1) as usize, 0);
            }
        }
        Position::new(0, 0)
    }
}

fn kitty_supported() -> bool {
    std::env::var_os("KITTY_WINDOW_ID").is_some()
        || std::env::var("TERM").is_ok_and(|term| term.contains("kitty"))
}

fn fit_cells(
    width_px: u32,
    height_px: u32,
    cell_width: f32,
    cell_height: f32,
    max_cols: u16,
    max_rows: u16,
) -> (u16, u16) {
    let mut rows = (height_px as f32 / cell_height.max(1.0)).ceil().max(1.0);
    let mut cols = (width_px as f32 / cell_width.max(1.0)).ceil().max(1.0);
    let scale = (max_rows as f32 / rows)
        .min(max_cols as f32 / cols)
        .min(1.0);
    rows = (rows * scale).round().max(1.0);
    cols = (cols * scale).round().max(1.0);
    (cols as u16, rows as u16)
}

fn image_links(line: &str) -> Vec<(usize, String)> {
    let mut links = Vec::new();
    let mut offset = 0;
    while let Some(found) = line[offset..].find("![") {
        let start = offset + found;
        if line[start..].starts_with("![[") {
            let Some(close) = line[start + 3..].find("]]") else {
                break;
            };
            let end = start + 3 + close;
            links.push((
                line[..start].chars().count(),
                line[start + 3..end].trim().to_string(),
            ));
            offset = end + 2;
            continue;
        }

        let Some(mid) = line[start..].find("](") else {
            break;
        };
        let target_start = start + mid + 2;
        let Some(close) = line[target_start..].find(')') else {
            break;
        };
        let end = target_start + close;
        links.push((
            line[..start].chars().count(),
            line[target_start..end].trim().to_string(),
        ));
        offset = end + 1;
    }
    links
}

fn resolve_image(source: &Path, vault: Option<&Path>, target: &str) -> Option<PathBuf> {
    let target = target
        .trim()
        .trim_matches(|ch| ch == '<' || ch == '>')
        .split('#')
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default();
    if target.is_empty()
        || target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("data:")
    {
        return None;
    }
    let target = percent_decode(target);
    let target = Path::new(&target);

    let mut candidates = Vec::new();
    if target.is_absolute() {
        if let Some(vault) = vault {
            candidates.push(vault.join(target.to_string_lossy().trim_start_matches('/')));
        }
        candidates.push(target.to_path_buf());
    } else {
        if let Some(parent) = source.parent() {
            candidates.push(parent.join(target));
        }
        if let Some(vault) = vault {
            candidates.push(vault.join(target));
        }
    }
    candidates.into_iter().find(|path| path.is_file())
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2])) {
                output.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&output).into_owned()
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
struct CachedDimensions {
    modified: Option<SystemTime>,
    dimensions: Option<(u32, u32)>,
}

fn dimension_cache() -> &'static Mutex<HashMap<PathBuf, CachedDimensions>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedDimensions>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn image_dimensions(path: &Path) -> Option<(u32, u32)> {
    let modified = fs::metadata(path).and_then(|meta| meta.modified()).ok();
    if let Some(cached) = dimension_cache().lock().unwrap().get(path) {
        if cached.modified == modified {
            return cached.dimensions;
        }
    }

    let dimensions = read_image_dimensions(path);
    dimension_cache().lock().unwrap().insert(
        path.to_path_buf(),
        CachedDimensions {
            modified,
            dimensions,
        },
    );
    dimensions
}

fn read_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    let data = fs::read(path).ok()?;
    if data.len() >= 24 && data.starts_with(b"\x89PNG\r\n\x1a\n") {
        let width = u32::from_be_bytes(data[16..20].try_into().ok()?);
        let height = u32::from_be_bytes(data[20..24].try_into().ok()?);
        return Some((width, height));
    }
    if data.len() >= 10 && (data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a")) {
        let width = u16::from_le_bytes(data[6..8].try_into().ok()?) as u32;
        let height = u16::from_le_bytes(data[8..10].try_into().ok()?) as u32;
        return Some((width, height));
    }
    if let Some(dimensions) = jpeg_dimensions(&data) {
        return Some(dimensions);
    }
    webp_dimensions(&data)
}

fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }
    let mut index = 2;
    while index + 9 < data.len() {
        if data[index] != 0xFF {
            index += 1;
            continue;
        }
        let marker = data[index + 1];
        index += 2;
        if marker == 0xD8 || marker == 0xD9 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }
        let length = u16::from_be_bytes([data[index], data[index + 1]]) as usize;
        if (0xC0..=0xCF).contains(&marker)
            && !matches!(marker, 0xC4 | 0xC8 | 0xCC)
            && index + 7 < data.len()
        {
            let height = u16::from_be_bytes([data[index + 3], data[index + 4]]) as u32;
            let width = u16::from_be_bytes([data[index + 5], data[index + 6]]) as u32;
            return Some((width, height));
        }
        index = index.checked_add(length)?;
    }
    None
}

fn webp_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 30 || !data.starts_with(b"RIFF") || &data[8..12] != b"WEBP" {
        return None;
    }
    match &data[12..16] {
        b"VP8X" => {
            let width = 1 + u32::from_le_bytes([data[24], data[25], data[26], 0]);
            let height = 1 + u32::from_le_bytes([data[27], data[28], data[29], 0]);
            Some((width, height))
        }
        b"VP8 " => {
            let payload = &data[20..];
            if payload.len() < 10 {
                return None;
            }
            let width = u16::from_le_bytes([payload[6], payload[7]]) as u32 & 0x3FFF;
            let height = u16::from_le_bytes([payload[8], payload[9]]) as u32 & 0x3FFF;
            Some((width, height))
        }
        b"VP8L" => {
            let payload = &data[20..];
            if payload.len() < 5 {
                return None;
            }
            let bits = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
            let width = (bits & 0x3FFF) + 1;
            let height = ((bits >> 14) & 0x3FFF) + 1;
            Some((width, height))
        }
        _ => None,
    }
}

#[cfg(unix)]
fn cell_size() -> (f32, f32) {
    let mut size = std::mem::MaybeUninit::<libc::winsize>::zeroed();
    let ok = unsafe { libc::ioctl(1, libc::TIOCGWINSZ, size.as_mut_ptr()) } == 0;
    let size = unsafe { size.assume_init() };
    if ok && size.ws_col > 0 && size.ws_row > 0 {
        let width = if size.ws_xpixel > 0 {
            size.ws_xpixel as f32 / size.ws_col as f32
        } else {
            8.0
        };
        let height = if size.ws_ypixel > 0 {
            size.ws_ypixel as f32 / size.ws_row as f32
        } else {
            16.0
        };
        (width.max(1.0), height.max(1.0))
    } else {
        (8.0, 16.0)
    }
}

#[cfg(not(unix))]
fn cell_size() -> (f32, f32) {
    (8.0, 16.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_markdown_images() {
        let links = image_links("text ![alt](assets/a.png) and ![[b.png]]");
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].1, "assets/a.png");
        assert_eq!(links[1].1, "b.png");
    }

    #[test]
    fn decodes_percent_escapes() {
        assert_eq!(percent_decode("a%20b.png"), "a b.png");
    }

    #[test]
    fn fits_images_inside_the_viewport() {
        let (cols, rows) = fit_cells(4000, 4000, 8.0, 16.0, 100, 30);
        assert!(cols <= 100);
        assert!(rows <= 30);
        assert!(cols > 0 && rows > 0);
    }
}
