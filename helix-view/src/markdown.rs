//! Markdown image layout shared by the view and the terminal renderer.
//!
//! Images need virtual lines reserved after their anchor line so following
//! text is reflowed instead of covered. This module computes that layout in
//! `helix-view`, where both cursor positioning and rendering can use it.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::Hash;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use helix_core::doc_formatter::FormattedGrapheme;
use helix_core::text_annotations::LineAnnotation;
use helix_core::Position;

use crate::{Document, DocumentId, View, ViewId};

const MAX_HEIGHT_PERCENT: f32 = 0.5;

#[derive(Debug, Clone)]
pub struct ImageLayout {
    pub path: PathBuf,
    pub char_idx: usize,
    pub line: usize,
    pub rows: u16,
    pub cols: u16,
    pub offset_rows: u16,
    /// Pixel dimensions of `path` (post-conversion). Used for source-rect
    /// cropping when an image is partially scrolled out of view.
    pub width_px: u32,
    pub height_px: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ImageLayoutKey {
    doc: DocumentId,
    view: ViewId,
    version: i32,
    anchor: usize,
    cursor_line: usize,
    width: u16,
    height: u16,
}

/// `visible_images` is called by both the text annotation pass and the image
/// renderer in the same frame. Cache the short-lived layout so the filesystem
/// and remote/conversion checks are not repeated. Keep a small map instead of
/// a single entry so alternating split views don't invalidate each other.
type ImageLayoutCache = Mutex<HashMap<ImageLayoutKey, (Instant, Vec<ImageLayout>)>>;

fn image_layout_cache() -> &'static ImageLayoutCache {
    static CACHE: OnceLock<ImageLayoutCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

const IMAGE_LAYOUT_TTL: Duration = Duration::from_millis(100);
const IMAGE_LAYOUT_CACHE_ENTRIES: usize = 8;

/// Whether the current buffer is an image file shown full-view instead of
/// as text. Shared by layout, placement, and viewport blanking.
pub fn is_hijacked_image(doc: &Document) -> bool {
    let config = doc.config.load();
    if !config.markdown.images || !kitty_supported() {
        return false;
    }
    if doc.language_name() == Some("markdown") {
        return false;
    }
    doc.path()
        .and_then(|path| path.extension())
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(ext))
        })
}
/// Compute inline image layouts for the visible section of a markdown buffer.
/// Image files opened directly get a single full-view layout (hijack).
pub fn visible_images(doc: &Document, view: &View) -> Vec<ImageLayout> {
    let config = doc.config.load();
    if !config.markdown.images || !kitty_supported() {
        return Vec::new();
    }
    if doc.language_name() != Some("markdown") {
        return hijacked_image(doc, view);
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

    let cursor = doc.selection(view.id).primary().cursor(text);
    let cursor_line = text.char_to_line(cursor);

    let view_offset = doc.view_offset(view.id);
    let key = ImageLayoutKey {
        doc: doc.id(),
        view: view.id,
        version: doc.version(),
        anchor: view_offset.anchor,
        cursor_line,
        width: inner.width,
        height: inner.height,
    };
    if let Some((cached_at, images)) = image_layout_cache().lock().unwrap().get(&key) {
        if cached_at.elapsed() < IMAGE_LAYOUT_TTL {
            return images.clone();
        }
    }
    let start_line = text.char_to_line(view_offset.anchor.min(text.len_chars()));
    let end_line = (start_line + inner.height as usize + 2).min(text.len_lines());
    let (cell_width, cell_height) = cell_size();
    let max_rows = ((inner.height as f32 * MAX_HEIGHT_PERCENT).floor() as u16).max(2);
    let max_cols = inner.width;

    let mut images = Vec::new();
    let mut line_rows: HashMap<usize, u16> = HashMap::new();

    for line_idx in start_line..end_line {
        // Anti-conceal: the cursor line shows its raw Markdown, so do not
        // reserve virtual rows or place the image it references.
        if line_idx == cursor_line {
            continue;
        }
        let line_start = text.line_to_char(line_idx);
        let line = text.line(line_idx).to_string();
        for (column, target) in image_links(&line) {
            let Some(path) = resolve_image(
                &source,
                vault.as_deref(),
                &target,
                config.markdown.download_remote_images,
            ) else {
                continue;
            };
            let Some((path, width_px, height_px)) = dimensions_or_converted(&path) else {
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
                width_px,
                height_px,
            });
        }
    }

    {
        let mut cache = image_layout_cache().lock().unwrap();
        cache.retain(|_, (cached_at, _)| cached_at.elapsed() < IMAGE_LAYOUT_TTL);
        if cache.len() >= IMAGE_LAYOUT_CACHE_ENTRIES && !cache.contains_key(&key) {
            if let Some(oldest) = cache
                .iter()
                .min_by_key(|(_, (cached_at, _))| *cached_at)
                .map(|(key, _)| *key)
            {
                cache.remove(&oldest);
            }
        }
        cache.insert(key, (Instant::now(), images.clone()));
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
                // Obsidian embeds can carry a width/alias after `|`:
                // `![[image.png|300]]`.
                line[start + 3..end]
                    .split('|')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
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
        let target = line[target_start..end].trim();
        // `![alt](path "title")` / `![alt](<path with spaces>)`: keep only
        // the destination, not the optional title or angle brackets.
        let target = if let Some(inner) = target.strip_prefix('<') {
            inner.split('>').next().unwrap_or(inner).trim()
        } else {
            target.split_whitespace().next().unwrap_or(target)
        };
        links.push((line[..start].chars().count(), target.to_string()));
        offset = end + 1;
    }
    links
}

fn resolve_image(
    source: &Path,
    vault: Option<&Path>,
    target: &str,
    download_remote: bool,
) -> Option<PathBuf> {
    let target = target
        .trim()
        .trim_matches(|ch| ch == '<' || ch == '>')
        .split('|')
        .next()
        .unwrap_or_default()
        .split('#')
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default();
    if target.is_empty() || target.starts_with("data:") {
        return None;
    }
    if target.starts_with("http://") || target.starts_with("https://") {
        if !download_remote || !remote_url_allowed(target) {
            return None;
        }
        return resolve_remote(target);
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

fn remote_url_host(url: &str) -> Option<&str> {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    if let Some(rest) = authority.strip_prefix('[') {
        return rest.split(']').next();
    }
    authority.split(':').next()
}

/// Reject localhost/private-literal destinations. This cannot stop DNS
/// rebinding, but prevents the common accidental/localhost SSRF cases while
/// keeping remote image rendering useful.
fn remote_url_allowed(url: &str) -> bool {
    let Some(host) = remote_url_host(url) else {
        return false;
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
    {
        return false;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => ipv4_is_public(ip),
        Ok(IpAddr::V6(ip)) => ipv6_is_public(ip),
        Err(_) => host_resolves_public(&host),
    }
}

/// Resolve a hostname and require every address to be public. DNS can change
/// between this check and curl's own resolution, so this is not a complete
/// defense against rebinding; it does stop stable names that point at a
/// private/loopback address. Results are cached briefly because layout runs
/// on every rendered frame.
fn host_resolves_public(host: &str) -> bool {
    const DNS_CACHE_TTL: Duration = Duration::from_secs(300);
    {
        let cache = dns_allowed().lock().unwrap();
        if let Some((checked_at, allowed)) = cache.get(host) {
            if checked_at.elapsed() < DNS_CACHE_TTL {
                return *allowed;
            }
        }
    }

    use std::net::ToSocketAddrs;
    let allowed = format!("{host}:443")
        .to_socket_addrs()
        .map(|addrs| {
            let mut any = false;
            for addr in addrs {
                any = true;
                if !match addr.ip() {
                    IpAddr::V4(ip) => ipv4_is_public(ip),
                    IpAddr::V6(ip) => ipv6_is_public(ip),
                } {
                    return false;
                }
            }
            any
        })
        .unwrap_or(false);

    dns_allowed()
        .lock()
        .unwrap()
        .insert(host.to_string(), (Instant::now(), allowed));
    allowed
}

fn dns_allowed() -> &'static Mutex<HashMap<String, (Instant, bool)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (Instant, bool)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn ipv4_is_public(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
        || octets[0] == 0
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        || (octets[0] == 198 && (18..=19).contains(&octets[1]))
        || octets[0] >= 240)
}

fn ipv6_is_public(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    // IPv4-mapped addresses can still encode a private IPv4 destination.
    if segments[..5] == [0, 0, 0, 0, 0] && segments[5] == 0xffff {
        let v4 = Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            segments[6] as u8,
            (segments[7] >> 8) as u8,
            segments[7] as u8,
        );
        return ipv4_is_public(v4);
    }
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00 // unique local
        || (segments[0] & 0xffc0) == 0xfe80) // link local
}

/// Download a remote image in the background and return its cached path once
/// available. The first call spawns `curl` and returns `None`; later frames
/// pick the file up once it exists.
fn resolve_remote(url: &str) -> Option<PathBuf> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    let digest = {
        use std::hash::Hasher;
        hasher.finish()
    };
    let ext = url
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('.')
        .next()
        .filter(|ext| ext.len() <= 5 && ext.chars().all(|ch| ch.is_ascii_alphanumeric()))
        .map(|ext| format!(".{ext}"))
        .unwrap_or_default();
    let path = cache_dir()?
        .join("rx-helix")
        .join("images")
        .join(format!("remote-{digest:016x}{ext}"));
    if path.is_file() {
        return Some(path);
    }
    {
        let mut downloads = remote_downloads().lock().unwrap();
        if downloads.in_progress.contains(url) {
            return None;
        }
        if downloads
            .failed
            .get(url)
            .is_some_and(|failed_at| failed_at.elapsed() < REMOTE_RETRY_BACKOFF)
        {
            return None;
        }
        downloads.failed.remove(url);
        downloads.in_progress.insert(url.to_string());
    }
    let url = url.to_string();
    std::thread::spawn(move || {
        let part = path.with_extension("part");
        let ok = std::process::Command::new("curl")
            // Only web URLs, including redirects. Markdown can be untrusted
            // and must not be able to make curl read `file://` or other
            // protocols.
            .args([
                "-L",
                "-s",
                "--proto",
                "=http,https",
                "--proto-redir",
                "=http,https",
                "--max-filesize",
                "52428800",
                "--max-time",
                "30",
                "-o",
            ])
            .arg(&part)
            .arg(&url)
            .status()
            .is_ok_and(|status| status.success())
            && part.is_file();
        if ok {
            let _ = std::fs::rename(&part, &path);
        } else {
            let _ = std::fs::remove_file(&part);
        }
        let mut downloads = remote_downloads().lock().unwrap();
        downloads.in_progress.remove(&url);
        if ok {
            downloads.failed.remove(&url);
        } else {
            // Back off instead of respawning curl on every rendered frame.
            downloads.failed.insert(url, Instant::now());
        }
    });
    None
}

const REMOTE_RETRY_BACKOFF: Duration = Duration::from_secs(60);

#[derive(Default)]
struct RemoteDownloads {
    in_progress: HashSet<String>,
    failed: HashMap<String, Instant>,
}

fn remote_downloads() -> &'static Mutex<RemoteDownloads> {
    static DOWNLOADS: OnceLock<Mutex<RemoteDownloads>> = OnceLock::new();
    DOWNLOADS.get_or_init(|| Mutex::new(RemoteDownloads::default()))
}

/// Image extensions recognized when opening a file directly.
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp"];

/// Single full-view layout for an image file opened directly.
fn hijacked_image(doc: &Document, view: &View) -> Vec<ImageLayout> {
    if !is_hijacked_image(doc) {
        return Vec::new();
    }
    let Some(path) = doc.path() else {
        return Vec::new();
    };
    let inner = view.inner_area(doc);
    if inner.width == 0 || inner.height == 0 {
        return Vec::new();
    }
    let Some((path, width_px, height_px)) = dimensions_or_converted(path) else {
        return Vec::new();
    };
    let (cell_width, cell_height) = cell_size();
    let (cols, rows) = fit_cells(
        width_px,
        height_px,
        cell_width,
        cell_height,
        inner.width,
        inner.height,
    );
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    vec![ImageLayout {
        path,
        char_idx: 0,
        line: 0,
        rows,
        cols,
        offset_rows: 0,
        width_px,
        height_px,
    }]
}

/// Measure an image, converting through PNG first when the native reader
/// cannot sniff the format (e.g. SVG). Returns the path Kitty should read.
fn dimensions_or_converted(path: &Path) -> Option<(PathBuf, u32, u32)> {
    // Kitty's graphics protocol accepts PNG (or raw pixels), not JPEG/GIF/
    // WebP/SVG. Detect dimensions alone is not enough: always route non-PNG
    // files through the converter, even when we can read their dimensions.
    if is_png(path) {
        let dimensions = image_dimensions(path)?;
        return Some((path.to_path_buf(), dimensions.0, dimensions.1));
    }
    let converted = ensure_png_async(path)?;
    let dimensions = image_dimensions(&converted)?;
    Some((converted, dimensions.0, dimensions.1))
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

fn png_output_path(path: &Path) -> Option<PathBuf> {
    if is_png(path) {
        return Some(path.to_path_buf());
    }

    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    modified.hash(&mut hasher);
    Some(
        cache_dir()?
            .join("rx-helix")
            .join("images")
            .join(format!("{:016x}.png", {
                use std::hash::Hasher;
                hasher.finish()
            })),
    )
}

/// Convert a non-PNG image in the background. Returns the cached PNG path
/// once it exists; while a conversion is running, returns `None` so the
/// renderer can try again on a later frame without blocking the UI.
pub fn ensure_png_async(path: &Path) -> Option<PathBuf> {
    let out = png_output_path(path)?;
    if out.is_file() {
        return Some(out);
    }
    if conversion_failed_recently(&out) {
        return None;
    }
    {
        let mut in_progress = conversion_in_progress().lock().unwrap();
        if !in_progress.insert(out.clone()) {
            return None;
        }
    }

    let input = path.to_path_buf();
    std::thread::spawn(move || {
        let ok = ensure_png(&input).is_some();
        let mut in_progress = conversion_in_progress().lock().unwrap();
        in_progress.remove(&out);
        drop(in_progress);
        if !ok {
            conversion_failures()
                .lock()
                .unwrap()
                .insert(out, Instant::now());
        }
    });
    None
}

/// Kitty only understands PNG (or raw pixels). Convert other formats to a
/// cached PNG with ffmpeg (SVG uses rsvg-convert when available).
///
/// This is the blocking implementation used by the background conversion
/// worker; UI paths should use [`ensure_png_async`].
pub fn ensure_png(path: &Path) -> Option<PathBuf> {
    let out = png_output_path(path)?;
    if out.is_file() {
        return Some(out);
    }
    // A malformed image (or a missing ffmpeg/rsvg-convert) would otherwise
    // spawn a converter process on every frame. Back off briefly so
    // installing a converter later is picked up without a restart.
    if conversion_failed_recently(&out) {
        return None;
    }
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).ok()?;
    }

    let svg = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"));
    let converted = if svg {
        std::process::Command::new("rsvg-convert")
            .arg("-o")
            .arg(&out)
            .arg(path)
            .status()
            .ok()
    } else {
        std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(path)
            .args(["-frames:v", "1"])
            .arg(&out)
            .status()
            .ok()
    };
    if converted.is_some_and(|status| status.success()) && out.is_file() {
        conversion_failures().lock().unwrap().remove(&out);
        Some(out)
    } else {
        conversion_failures()
            .lock()
            .unwrap()
            .insert(out, Instant::now());
        None
    }
}

const CONVERSION_RETRY_BACKOFF: Duration = Duration::from_secs(30);

fn conversion_failures() -> &'static Mutex<HashMap<PathBuf, Instant>> {
    static FAILURES: OnceLock<Mutex<HashMap<PathBuf, Instant>>> = OnceLock::new();
    FAILURES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn conversion_failed_recently(out: &Path) -> bool {
    let mut failures = conversion_failures().lock().unwrap();
    match failures.get(out) {
        Some(failed_at) if failed_at.elapsed() < CONVERSION_RETRY_BACKOFF => true,
        Some(_) => {
            failures.remove(out);
            false
        }
        None => false,
    }
}

fn conversion_in_progress() -> &'static Mutex<HashSet<PathBuf>> {
    static IN_PROGRESS: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    IN_PROGRESS.get_or_init(|| Mutex::new(HashSet::new()))
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
    use std::io::{Read, Seek, SeekFrom};

    let mut file = fs::File::open(path).ok()?;
    let mut header = [0u8; 32];
    let read = file.read(&mut header).ok()?;
    let header = &header[..read];
    if header.len() >= 24 && header.starts_with(b"\x89PNG\r\n\x1a\n") {
        let width = u32::from_be_bytes(header[16..20].try_into().ok()?);
        let height = u32::from_be_bytes(header[20..24].try_into().ok()?);
        return Some((width, height));
    }
    if header.len() >= 10 && (header.starts_with(b"GIF87a") || header.starts_with(b"GIF89a")) {
        let width = u16::from_le_bytes(header[6..8].try_into().ok()?) as u32;
        let height = u16::from_le_bytes(header[8..10].try_into().ok()?) as u32;
        return Some((width, height));
    }
    if header.len() >= 4 && header[0] == 0xFF && header[1] == 0xD8 {
        file.seek(SeekFrom::Start(0)).ok()?;
        return jpeg_dimensions_stream(&mut file);
    }
    webp_dimensions(header)
}

/// Scan JPEG markers from the file without loading pixel data. APP/EXIF
/// segments can be much larger than any fixed header window.
fn jpeg_dimensions_stream(file: &mut fs::File) -> Option<(u32, u32)> {
    use std::io::{Read, Seek, SeekFrom};

    let mut soi = [0u8; 2];
    file.read_exact(&mut soi).ok()?;
    if soi != [0xFF, 0xD8] {
        return None;
    }
    let mut pair = [0u8; 2];
    loop {
        file.read_exact(&mut pair).ok()?;
        if pair[0] != 0xFF {
            return None;
        }
        let mut marker = pair[1];
        while marker == 0xFF {
            file.read_exact(&mut pair).ok()?;
            marker = pair[1];
        }
        if marker == 0xD8 || marker == 0xD9 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }
        let mut length_bytes = [0u8; 2];
        file.read_exact(&mut length_bytes).ok()?;
        let length = u16::from_be_bytes(length_bytes) as u64;
        if length < 2 {
            return None;
        }
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let mut frame = [0u8; 5];
            file.read_exact(&mut frame).ok()?;
            let height = u16::from_be_bytes([frame[1], frame[2]]) as u32;
            let width = u16::from_be_bytes([frame[3], frame[4]]) as u32;
            return Some((width, height));
        }
        file.seek(SeekFrom::Current(length as i64 - 2)).ok()?;
    }
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
    fn strips_image_titles_and_obsidian_widths() {
        let links =
            image_links(r#"![alt](assets/a.png "title") ![alt](<assets/b c.png>) ![[d.png|300]]"#);
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].1, "assets/a.png");
        assert_eq!(links[1].1, "assets/b c.png");
        assert_eq!(links[2].1, "d.png");
    }

    #[test]
    fn decodes_percent_escapes() {
        assert_eq!(percent_decode("a%20b.png"), "a b.png");
    }

    #[test]
    fn rejects_private_remote_image_hosts() {
        // Use a literal public address so the test does not depend on DNS.
        assert!(remote_url_allowed("https://93.184.216.34/a.png"));
        assert!(remote_url_allowed("http://user@93.184.216.34:8080/a.png"));
        for url in [
            "http://localhost/a.png",
            "http://127.0.0.1/a.png",
            "http://10.0.0.1/a.png",
            "http://192.168.1.1/a.png",
            "http://169.254.169.254/latest/meta-data",
            "http://[::1]/a.png",
            "http://[fe80::1]/a.png",
            "http://printer.local/a.png",
            "http://metadata.google.internal/a.png",
        ] {
            assert!(!remote_url_allowed(url), "allowed private URL {url}");
        }
    }

    #[test]
    fn fits_images_inside_the_viewport() {
        let (cols, rows) = fit_cells(4000, 4000, 8.0, 16.0, 100, 30);
        assert!(cols <= 100);
        assert!(rows <= 30);
        assert!(cols > 0 && rows > 0);
    }

    #[test]
    fn reads_dimensions_from_a_png_header() {
        let path = std::env::temp_dir().join(format!(
            "rx-helix-png-dims-{}-{:?}.png",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut data = b"\x89PNG\r\n\x1a\n".to_vec();
        data.extend_from_slice(&[0; 8]);
        data.extend_from_slice(&123u32.to_be_bytes());
        data.extend_from_slice(&456u32.to_be_bytes());
        // Pixel data beyond the header must not be required.
        data.extend_from_slice(&[0; 4096]);
        std::fs::write(&path, data).unwrap();
        assert_eq!(read_image_dimensions(&path), Some((123, 456)));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reads_dimensions_by_seeking_through_jpeg_segments() {
        let path = std::env::temp_dir().join(format!(
            "rx-helix-jpeg-dims-{}-{:?}.jpg",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut data = vec![0xFF, 0xD8];
        // A large APP0 segment the old whole-file reader would still load.
        data.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x12]);
        data.extend_from_slice(&[0; 16]);
        // SOF0: length, precision, height=300, width=600.
        data.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08, 0x01, 0x2C, 0x02, 0x58]);
        data.extend_from_slice(&[0; 10]);
        data.extend_from_slice(&[0; 4096]);
        std::fs::write(&path, data).unwrap();
        assert_eq!(read_image_dimensions(&path), Some((600, 300)));
        let _ = std::fs::remove_file(path);
    }
}
