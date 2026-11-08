//! Kitty graphics protocol image rendering for markdown buffers.
//!
//! Layout (including virtual lines reserved for each image) lives in
//! [`helix_view::markdown`]. This module transmits images once and moves
//! placements after each editor frame.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use helix_view::Editor;

struct Placement {
    path: PathBuf,
    x: u16,
    y: u16,
    cols: u16,
    rows: u16,
    /// Source-rectangle crop in image pixels. `None` shows the full image.
    crop: Option<(u32, u32, u32, u32)>,
}

struct CachedImage {
    modified: Option<SystemTime>,
    id: u32,
}

#[derive(Default)]
struct ImageState {
    next_id: u32,
    cache: HashMap<PathBuf, CachedImage>,
    placed: HashMap<u32, ()>,
    rendered: bool,
}

fn state() -> &'static Mutex<ImageState> {
    static STATE: OnceLock<Mutex<ImageState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(ImageState::default()))
}

pub fn render(editor: &Editor) {
    let config = editor.config();
    if !kitty_supported() {
        return;
    }
    if !config.markdown.images {
        clear();
        return;
    }

    let placements = compute_placements(editor);
    let mut out = String::new();
    let mut state = state().lock().unwrap();

    // Save the cursor the TUI positioned, then restore it after placing images.
    out.push_str("\x1b7");

    let mut current = HashMap::new();
    for placement in placements {
        let id = transmit(&mut state, &placement.path, &mut out);
        current.insert(id, ());
        // Re-placing the same image/placement id moves it without flicker.
        // A source rect (x,y,w,h) crops partially scrolled images instead
        // of squishing them.
        let crop = placement
            .crop
            .map(|(x, y, w, h)| format!(",x={x},y={y},w={w},h={h}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "\x1b[{};{}H\x1b_Ga=p,i={id},p={id},c={},r={}{crop},q=2,C=1\x1b\\",
            placement.y + 1,
            placement.x + 1,
            placement.cols,
            placement.rows,
        ));
    }

    // Delete images that scrolled out of view.
    for id in state.placed.keys() {
        if !current.contains_key(id) {
            out.push_str(&format!("\x1b_Ga=d,i={id},q=2\x1b\\"));
        }
    }
    state.placed = current;
    state.rendered = !state.placed.is_empty();

    out.push_str("\x1b8");
    drop(state);
    flush_output(&out);
}

/// Remove all image placements. Called on exit.
pub fn clear() {
    if !kitty_supported() {
        return;
    }
    let mut state = state().lock().unwrap();
    if state.rendered || !state.placed.is_empty() {
        flush_output("\x1b_Ga=d,d=A,q=2\x1b\\");
        state.placed.clear();
        state.rendered = false;
    }
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

    let text = doc.text().slice(..);
    if text.len_lines() == 0 {
        return Vec::new();
    }
    let inner = view.inner_area(doc);
    if inner.width == 0 || inner.height == 0 {
        return Vec::new();
    }
    let anchor = doc.view_offset(view.id).anchor.min(text.len_chars());
    let topline = text.char_to_line(anchor);
    let mut placements = Vec::new();

    for image in helix_view::markdown::visible_images(doc, &view) {
        let Some(path) = helix_view::markdown::ensure_png(&image.path) else {
            continue;
        };
        let rows_total = image.rows.max(1) as usize;

        if let Some(pos) = view.screen_coords_at_pos(doc, text, image.char_idx) {
            // Anchor on screen. Crop the bottom if the image runs past the
            // viewport instead of squishing it.
            let eff_row = pos.row as usize + image.offset_rows as usize;
            if eff_row >= inner.height as usize {
                continue;
            }
            let avail_rows = (inner.height as usize - eff_row).max(1);
            let rows_vis = (image.rows as usize).min(avail_rows).max(1);
            let avail_cols = inner.width.saturating_sub(pos.col as u16).max(1);
            let cols = image.cols.min(avail_cols).max(1);
            let crop = if rows_vis != image.rows as usize {
                // Show the top rows_vis/rows of the source image.
                let h = ((rows_vis as u64 * image.height_px as u64) / rows_total as u64)
                    .clamp(1, image.height_px as u64) as u32;
                Some((0, 0, image.width_px, h))
            } else {
                None
            };
            placements.push(Placement {
                path,
                x: inner.x.saturating_add(pos.col as u16),
                y: inner.y.saturating_add(eff_row as u16),
                cols,
                rows: rows_vis as u16,
                crop,
            });
        } else if image.line < topline {
            // Anchor scrolled above the viewport: keep the visible tail
            // on screen with a cropped source rect (image.nvim overlap).
            let scrolled = topline - image.line;
            let top_cut = scrolled.saturating_sub(image.offset_rows as usize);
            if top_cut >= rows_total {
                continue;
            }
            let y_off = (image.offset_rows as usize).saturating_sub(scrolled);
            if y_off >= inner.height as usize {
                continue;
            }
            let avail_rows = (inner.height as usize - y_off).max(1);
            let rows_vis = (rows_total - top_cut).min(avail_rows).max(1);
            let col = image.char_idx.saturating_sub(text.line_to_char(image.line)) as u16;
            let cols = image
                .cols
                .min(inner.width.saturating_sub(col).max(1))
                .max(1);
            let y_src = ((top_cut as u64 * image.height_px as u64) / rows_total as u64)
                .min(image.height_px as u64) as u32;
            let h_max = (image.height_px as u64).saturating_sub(y_src as u64).max(1);
            let h_src = ((rows_vis as u64 * image.height_px as u64) / rows_total as u64)
                .clamp(1, h_max) as u32;
            placements.push(Placement {
                path,
                x: inner.x.saturating_add(col),
                y: inner.y.saturating_add(y_off as u16),
                cols,
                rows: rows_vis as u16,
                crop: Some((0, y_src, image.width_px, h_src)),
            });
        }
        // Anchor below the viewport: skip until scrolled into view.
    }

    placements
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
    fn encodes_base64() {
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
    }
}
