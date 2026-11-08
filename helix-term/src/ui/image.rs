//! Kitty graphics protocol image rendering for markdown buffers.
//!
//! Images are placed over the editor surface after each frame. The protocol
//! allows placing an image at a terminal cell without changing the TUI buffer,
//! so the editor keeps showing ordinary text underneath.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use helix_view::Editor;

const CELL_ASPECT: f32 = 2.0;
const MAX_IMAGE_HEIGHT: f32 = 0.5;

struct Placement {
    path: PathBuf,
    x: u16,
    y: u16,
    cols: u16,
    rows: u16,
}

struct CachedImage {
    modified: Option<SystemTime>,
    id: u32,
}

#[derive(Default)]
struct ImageState {
    next_id: u32,
    cache: HashMap<PathBuf, CachedImage>,
}

fn state() -> &'static Mutex<ImageState> {
    static STATE: OnceLock<Mutex<ImageState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(ImageState::default()))
}

pub fn render(editor: &Editor) {
    let config = editor.config();
    if !config.markdown.images || !kitty_supported() {
        return;
    }

    let placements = compute_placements(editor);
    let mut out = String::new();

    // Drop placements from the previous frame before drawing new ones.
    out.push_str("\x1b_Ga=d,d=A\x1b\\");
    if placements.is_empty() {
        flush_output(&out);
        return;
    }

    // Save the cursor the TUI positioned, then restore it after placing images.
    out.push_str("\x1b7");
    let mut state = state().lock().unwrap();
    for placement in placements {
        let id = transmit(&mut state, &placement.path, &mut out);
        out.push_str(&format!(
            "\x1b[{};{}H\x1b_Ga=p,i={id},p={id},c={},r={},C=1\x1b\\",
            placement.y + 1,
            placement.x + 1,
            placement.cols,
            placement.rows,
        ));
    }
    out.push_str("\x1b8");
    flush_output(&out);
}

fn flush_output(output: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(output.as_bytes());
    let _ = stdout.flush();
}

fn kitty_supported() -> bool {
    std::env::var_os("KITTY_WINDOW_ID").is_some()
        || std::env::var("TERM").is_ok_and(|term| term.contains("kitty"))
}

fn transmit(state: &mut ImageState, path: &Path, out: &mut String) -> u32 {
    let modified = fs::metadata(path).and_then(|meta| meta.modified()).ok();
    if let Some(cached) = state.cache.get(path) {
        if cached.modified == modified {
            return cached.id;
        }
    }

    state.next_id = state.next_id.wrapping_add(1).max(1);
    let id = state.next_id;
    state
        .cache
        .insert(path.to_path_buf(), CachedImage { modified, id });

    let encoded_path = base64_encode(path.to_string_lossy().as_bytes());
    out.push_str(&format!(
        "\x1b_Ga=t,f=100,t=f,i={id},q=2;{encoded_path}\x1b\\"
    ));
    id
}

fn compute_placements(editor: &Editor) -> Vec<Placement> {
    let view = editor.tree.get(editor.tree.focus);
    let Some(doc) = editor.documents.get(&view.doc) else {
        return Vec::new();
    };
    if doc.language_name() != Some("markdown") {
        return Vec::new();
    }
    let Some(source) = doc.path().map(Path::to_path_buf) else {
        return Vec::new();
    };

    let config = editor.config();
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

    let view_offset = doc.view_offset(view.id);
    let inner = view.inner_area(doc);
    if inner.width == 0 || inner.height == 0 {
        return Vec::new();
    }

    let start_line = text.char_to_line(view_offset.anchor.min(text.len_chars()));
    let end_line = (start_line + inner.height as usize).min(text.len_lines());
    let max_rows =
        ((inner.height as f32 * MAX_IMAGE_HEIGHT).floor() as u16).clamp(2, inner.height.max(2));

    let mut placements = Vec::new();
    for line_idx in start_line..end_line {
        let line_start = text.line_to_char(line_idx);
        let line = text.line(line_idx).to_string();
        for (column, target) in image_links(&line) {
            let Some(path) = resolve_image(&source, vault.as_deref(), &target) else {
                continue;
            };
            let Some(pos) = view.screen_coords_at_pos(doc, text, line_start + column) else {
                continue;
            };
            let (width_px, height_px) = image_size(&path).unwrap_or((320, 200));
            let (cols, rows) = fit_cells(
                width_px,
                height_px,
                inner.width.saturating_sub(pos.col as u16).max(1),
                pos.row as u16,
                inner.height,
                max_rows,
            );
            placements.push(Placement {
                path,
                x: inner.x,
                y: inner.y.saturating_add(pos.row as u16),
                cols,
                rows,
            });
        }
    }
    placements
}

#[allow(clippy::too_many_arguments)]
fn fit_cells(
    width_px: u32,
    height_px: u32,
    max_cols: u16,
    pos_row: u16,
    inner_height: u16,
    max_rows: u16,
) -> (u16, u16) {
    let available_rows = inner_height.saturating_sub(pos_row);
    let max_rows = max_rows.min(available_rows.max(1));
    let natural_rows = (height_px as f32 / 20.0).ceil().max(2.0) as u16;
    let mut rows = natural_rows.min(max_rows).max(1);
    let mut cols = (rows as f32 * width_px as f32 / height_px.max(1) as f32 / CELL_ASPECT)
        .ceil()
        .max(1.0) as u16;

    if cols > max_cols {
        cols = max_cols.max(1);
        rows = (cols as f32 * height_px.max(1) as f32 / width_px.max(1) as f32 * CELL_ASPECT)
            .ceil()
            .clamp(1.0, max_rows.max(1) as f32) as u16;
    }
    (cols, rows)
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
            let target = line[start + 3..end].trim().to_string();
            links.push((line[..start].chars().count(), target));
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
        let target = line[target_start..end].trim().to_string();
        links.push((line[..start].chars().count(), target));
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

fn image_size(path: &Path) -> Option<(u32, u32)> {
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
    jpeg_size(&data)
}

fn jpeg_size(data: &[u8]) -> Option<(u32, u32)> {
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

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        output.push(TABLE[(triple >> 18) as usize & 0x3F] as char);
        output.push(TABLE[(triple >> 12) as usize & 0x3F] as char);
        output.push(if chunk.len() > 1 {
            TABLE[(triple >> 6) as usize & 0x3F] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[triple as usize & 0x3F] as char
        } else {
            '='
        });
    }
    output
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
    fn encodes_base64() {
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
    }
}
