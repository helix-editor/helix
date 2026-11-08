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
    /// Paths with image data currently transmitted to Kitty, and their ids.
    /// Entries are dropped whenever Kitty is told to delete the image so
    /// the next render re-transmits instead of referencing a dead id.
    cache: HashMap<PathBuf, CachedImage>,
    placed: HashMap<u32, PathBuf>,
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

    let mut current: HashMap<u32, PathBuf> = HashMap::new();
    for placement in placements {
        let id = transmit(&mut state, &placement.path, &mut out);
        current.insert(id, placement.path.clone());
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

    // Delete images that scrolled out of view, and forget their ids so a
    // later render re-transmits the data (a placement referencing a deleted
    // image id is silently dropped by Kitty).
    let gone: Vec<(u32, PathBuf)> = state
        .placed
        .iter()
        .filter(|(id, _)| !current.contains_key(id))
        .map(|(id, path)| (*id, path.clone()))
        .collect();
    for (id, path) in gone {
        out.push_str(&format!("\x1b_Ga=d,d=i,i={id},q=2\x1b\\"));
        state.cache.remove(&path);
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
        // d=A drops transmitted data too: forget every id so the next
        // render re-transmits instead of placing dead ids.
        state.cache.clear();
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
        }
        // Anchor at or above the viewport top, or below it: skip. The line
        // annotation drops virtual rows for off-screen anchors, so text
        // reflows with no gap and nothing needs covering. (Showing the tail
        // of a scrolled-off image would require virtual rows *before* the
        // first rendered line, which the annotation model can't express.)
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
