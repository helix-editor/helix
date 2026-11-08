//! Kitty graphics protocol image rendering for markdown buffers.
//!
//! Layout (including virtual lines reserved for each image) lives in
//! [`helix_view::markdown`]. This module converts images to PNG, transmits them
//! once, and moves placements after each editor frame.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use helix_view::Editor;

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
        out.push_str(&format!(
            "\x1b[{};{}H\x1b_Ga=p,i={id},p={id},c={},r={},q=2,C=1\x1b\\",
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
    let inner = view.inner_area(doc);
    let mut placements = Vec::new();

    for image in helix_view::markdown::visible_images(doc, &view) {
        let Some(path) = ensure_png(&image.path) else {
            continue;
        };
        let Some(pos) = view.screen_coords_at_pos(doc, text, image.char_idx) else {
            continue;
        };
        placements.push(Placement {
            path,
            x: inner.x.saturating_add(pos.col as u16),
            y: inner
                .y
                .saturating_add(pos.row as u16)
                .saturating_add(image.offset_rows),
            cols: image.cols,
            rows: image.rows,
        });
    }

    placements
}

fn is_png(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut signature = [0u8; 8];
    std::io::Read::read_exact(&mut file, &mut signature).is_ok()
        && signature == *b"\x89PNG\r\n\x1a\n"
}

fn cache_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_CACHE_HOME") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache"))
}

/// Kitty only understands PNG (or raw pixels). Convert other formats to a
/// cached PNG with ffmpeg, which is commonly available where ImageMagick is not.
fn ensure_png(path: &Path) -> Option<PathBuf> {
    if is_png(path) {
        return Some(path.to_path_buf());
    }

    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    modified.hash(&mut hasher);
    let out = cache_dir()?
        .join("rx-helix")
        .join("images")
        .join(format!("{:016x}.png", hasher.finish()));
    if out.is_file() {
        return Some(out);
    }
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).ok()?;
    }

    let svg = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"));
    let converted = if svg {
        Command::new("rsvg-convert")
            .arg("-o")
            .arg(&out)
            .arg(path)
            .status()
            .ok()?
    } else {
        Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(path)
            .args(["-frames:v", "1"])
            .arg(&out)
            .status()
            .ok()?
    };
    converted.success().then_some(out)
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
