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

struct PlacedImage {
    image_id: u32,
    path: PathBuf,
}

#[derive(Default)]
struct ImageState {
    next_id: u32,
    /// Paths with image data currently transmitted to Kitty, and their ids.
    /// Entries are dropped whenever Kitty is told to delete the image so
    /// the next render re-transmits instead of referencing a dead id.
    cache: HashMap<PathBuf, CachedImage>,
    /// Placements in render order. Placement ids are derived from the index,
    /// so a change in order/count invalidates all placements.
    placed: Vec<PlacedImage>,
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

    // Nothing to draw and no existing placement to remove: avoid writing a
    // save/restore cursor sequence on every frame for ordinary buffers.
    if placements.is_empty() && state.placed.is_empty() {
        return;
    }

    // Save the cursor the TUI positioned, then restore it after placing images.
    out.push_str("\x1b7");

    // Placement ids are positional. When the image set or its order changes,
    // delete all old placements first so duplicate links cannot leave a stale
    // placement behind. Transmitted data stays cached.
    let layout_changed = state.placed.len() != placements.len()
        || state
            .placed
            .iter()
            .zip(&placements)
            .any(|(old, new)| old.path != new.path);
    if layout_changed && !state.placed.is_empty() {
        out.push_str("\x1b_Ga=d,d=a,q=2\x1b\\");
    }

    let old_ids: Vec<u32> = state.placed.iter().map(|placed| placed.image_id).collect();
    let mut current = Vec::with_capacity(placements.len());
    for (placement_index, placement) in placements.into_iter().enumerate() {
        let id = transmit(&mut state, &placement.path, &mut out);
        // Re-placing the same placement id moves it without flicker. Use a
        // distinct placement id per occurrence so the same image linked twice
        // in one note gets two placements (Kitty treats `p` as the placement
        // key, separate from the transmitted `i` image key).
        let placement_id = placement_index as u32 + 1;
        current.push(PlacedImage {
            image_id: id,
            path: placement.path.clone(),
        });
        // A source rect (x,y,w,h) crops partially scrolled images instead
        // of squishing them.
        let crop = placement
            .crop
            .map(|(x, y, w, h)| format!(",x={x},y={y},w={w},h={h}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "\x1b[{};{}H\x1b_Ga=p,i={id},p={placement_id},c={},r={}{crop},q=2,C=1\x1b\\",
            placement.y + 1,
            placement.x + 1,
            placement.cols,
            placement.rows,
        ));
    }

    // Free transmitted image data only when no placement still references it.
    let used: std::collections::HashSet<u32> =
        current.iter().map(|placed| placed.image_id).collect();
    let mut freed = std::collections::HashSet::new();
    for image_id in old_ids {
        if !used.contains(&image_id) && freed.insert(image_id) {
            out.push_str(&format!("\x1b_Ga=d,d=I,i={image_id},q=2\x1b\\"));
            state.cache.retain(|_, cached| cached.id != image_id);
        }
    }
    state.placed = current;

    out.push_str("\x1b8");
    drop(state);
    flush_output(&out);
}

/// Remove placements while keeping transmitted data. Used when an overlay
/// temporarily covers the text, so closing it does not retransmit every
/// image.
pub fn clear_placements() {
    if !kitty_supported() {
        return;
    }
    let mut state = state().lock().unwrap();
    if state.placed.is_empty() {
        return;
    }
    flush_output("\x1b_Ga=d,d=a,q=2\x1b\\");
    state.placed.clear();
}

/// Remove all image placements. Called on exit.
pub fn clear() {
    if !kitty_supported() {
        return;
    }
    let mut state = state().lock().unwrap();
    if !state.cache.is_empty() || !state.placed.is_empty() {
        flush_output("\x1b_Ga=d,d=A,q=2\x1b\\");
        state.placed.clear();
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

    for image in helix_view::markdown::visible_images(doc, view) {
        let Some(path) = helix_view::markdown::ensure_png_async(&image.path) else {
            continue;
        };
        let rows_total = image.rows.max(1) as usize;

        if let Some(pos) = view.screen_coords_at_pos(doc, text, image.char_idx) {
            // Anchor on screen. Crop the bottom if the image runs past the
            // viewport instead of squishing it.
            let eff_row = pos.row + image.offset_rows as usize;
            if eff_row >= inner.height as usize {
                continue;
            }
            let avail_rows = (inner.height as usize - eff_row).max(1);
            let rows_vis = (image.rows as usize).min(avail_rows).max(1);
            let avail_cols = inner.width.saturating_sub(pos.col as u16).max(1);
            let cols = image.cols.min(avail_cols).max(1);
            let rows_cropped = rows_vis != image.rows as usize;
            let cols_cropped = cols != image.cols;
            let crop = if rows_cropped || cols_cropped {
                // Crop source pixels instead of squishing when either axis
                // runs past the viewport.
                let h = if rows_cropped {
                    ((rows_vis as u64 * image.height_px as u64) / rows_total as u64)
                        .clamp(1, image.height_px as u64) as u32
                } else {
                    image.height_px
                };
                let w = if cols_cropped {
                    ((cols as u64 * image.width_px as u64) / image.cols.max(1) as u64)
                        .clamp(1, image.width_px as u64) as u32
                } else {
                    image.width_px
                };
                Some((0, 0, w, h))
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
