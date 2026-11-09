//! Built-in prose spell checking.
//!
//! Spelling is delegated to the system `hunspell` dictionary through its
//! ispell-compatible pipe interface. Markdown/code/URL syntax is blanked out
//! before checking so only prose words produce diagnostics and every reported
//! offset still maps back to the original document character positions.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use anyhow::{bail, Context as _, Result};
use helix_core::diagnostic::{Diagnostic, DiagnosticProvider, Range, Severity};
use helix_core::Rope;
use helix_view::Document;

const MAX_SUGGESTIONS: usize = 20;

const PROSE_EXTENSIONS: &[&str] = &[
    "md", "markdown", "mdx", "mkd", "mkdn", "mdwn", "mdown", "mdtxt", "mdtext", "txt", "text",
    "rst", "rest", "org", "typst", "tex", "latex", "dj", "djot", "adoc", "asciidoc",
];

const PROSE_LANGUAGES: &[&str] = &[
    "markdown",
    "text",
    "txt",
    "plaintext",
    "rst",
    "org",
    "typst",
    "latex",
    "djot",
    "asciidoc",
    "mail",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellIssue {
    /// Zero-based document line.
    pub line: usize,
    /// Character offset within the line.
    pub start: usize,
    /// Exclusive character offset within the line.
    pub end: usize,
    pub word: String,
    pub suggestions: Vec<String>,
}

/// Whether a document should be spell checked automatically. This covers
/// Markdown, plain text and common prose formats without affecting source
/// files or configuration.
pub fn is_prose(doc: &Document) -> bool {
    is_prose_for(doc.language_name(), doc.path())
}

fn is_prose_for(language: Option<&str>, path: Option<&Path>) -> bool {
    if let Some(language) = language {
        if PROSE_LANGUAGES
            .iter()
            .any(|name| language.eq_ignore_ascii_case(name))
        {
            return true;
        }
    }

    let Some(path) = path else {
        return false;
    };
    let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    PROSE_EXTENSIONS
        .iter()
        .any(|known| extension.eq_ignore_ascii_case(known))
}

pub fn available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("hunspell")
            .arg("-v")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    })
}

/// Check an entire document. Returns misspelled words in document order.
pub fn check_text(text: &Rope, language: &str) -> Result<Vec<SpellIssue>> {
    let lines = sanitized_lines(text);
    if lines.iter().all(|line| line.iter().all(|ch| *ch == ' ')) {
        return Ok(Vec::new());
    }
    let output = run_hunspell(&lines, language)?;
    Ok(parse_hunspell_output(&output, &lines))
}

/// Spell suggestions for one word. Used by `z=` when the diagnostic does not
/// already carry suggestions (or the cursor is on a word without one).
pub fn suggestions(word: &str, language: &str) -> Result<Vec<String>> {
    if word.trim().is_empty() {
        return Ok(Vec::new());
    }
    let line = format!(" {word}\n");
    let output = run_hunspell_text(&line, language)?;
    let mut suggestions = Vec::new();
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("& ") {
            if let Some(suggestions_text) = rest.split_once(':').map(|(_, text)| text) {
                suggestions.extend(parse_suggestions(suggestions_text));
            }
        }
    }
    suggestions.truncate(MAX_SUGGESTIONS);
    Ok(suggestions)
}

/// Find the word under a character position for `z=` fallback.
pub fn word_at(text: &Rope, pos: usize) -> Option<(usize, usize, String)> {
    let len = text.len_chars();
    if len == 0 {
        return None;
    }
    let mut cursor = pos.min(len.saturating_sub(1));
    if !is_word_char(text.char(cursor)) && cursor > 0 && is_word_char(text.char(cursor - 1)) {
        cursor -= 1;
    }
    if !is_word_char(text.char(cursor)) {
        return None;
    }
    let mut start = cursor;
    while start > 0 && is_word_char(text.char(start - 1)) {
        start -= 1;
    }
    let mut end = cursor + 1;
    while end < len && is_word_char(text.char(end)) {
        end += 1;
    }
    let word: String = text.slice(start..end).chars().collect();
    Some((start, end, word))
}

pub fn issue_diagnostics(issues: &[SpellIssue], text: &Rope) -> Vec<Diagnostic> {
    issues
        .iter()
        .filter_map(|issue| {
            if issue.line >= text.len_lines() || issue.end <= issue.start {
                return None;
            }
            let line_start = text.line_to_char(issue.line);
            Some(Diagnostic {
                range: Range {
                    start: line_start + issue.start,
                    end: line_start + issue.end,
                },
                ends_at_word: true,
                starts_at_word: true,
                zero_width: false,
                line: issue.line,
                message: format!("Misspelled: {}", issue.word),
                severity: Some(Severity::Hint),
                code: None,
                provider: DiagnosticProvider::Spell,
                tags: Vec::new(),
                source: Some("spell".to_string()),
                data: Some(serde_json::json!({
                    "word": issue.word,
                    "suggestions": issue.suggestions,
                })),
            })
        })
        .collect()
}

/// Replace syntax that should not be spell checked with spaces while keeping
/// character offsets identical to the source line.
fn sanitized_lines(text: &Rope) -> Vec<Vec<char>> {
    let mut lines = Vec::with_capacity(text.len_lines());
    let mut start = 0;

    // Skip YAML frontmatter, but only when it has a closing delimiter. A
    // leading `---` without one is a Markdown horizontal rule.
    if text.len_lines() > 0 && line_without_newline(text, 0).trim() == "---" {
        let limit = text.len_lines().min(100);
        if let Some(close) =
            (1..limit).find(|line| line_without_newline(text, *line).trim() == "---")
        {
            for line in 0..=close {
                lines.push(vec![' '; line_without_newline(text, line).chars().count()]);
            }
            start = close + 1;
        }
    }

    let mut fence: Option<(char, usize)> = None;
    for index in start..text.len_lines() {
        let line = line_without_newline(text, index);
        let chars: Vec<char> = line.chars().collect();

        if let Some((marker, len)) = fence {
            lines.push(vec![' '; chars.len()]);
            if is_closing_fence(&line, marker, len) {
                fence = None;
            }
            continue;
        }

        if let Some((marker, len)) = fence_start(&line) {
            fence = Some((marker, len));
            lines.push(vec![' '; chars.len()]);
            continue;
        }

        lines.push(sanitize_inline(&chars));
    }
    lines
}

fn line_without_newline(text: &Rope, line: usize) -> String {
    text.line(line)
        .to_string()
        .trim_end_matches(['\r', '\n'])
        .to_string()
}

fn fence_start(line: &str) -> Option<(char, usize)> {
    let trimmed = line.trim_start();
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let len = trimmed.chars().take_while(|ch| *ch == marker).count();
    if len < 3 {
        return None;
    }
    let info = trimmed.chars().skip(len).collect::<String>();
    if marker == '`' && info.contains('`') {
        return None;
    }
    Some((marker, len))
}

fn is_closing_fence(line: &str, marker: char, len: usize) -> bool {
    let trimmed = line.trim();
    let run = trimmed.chars().take_while(|ch| *ch == marker).count();
    run >= len && trimmed.chars().skip(run).all(char::is_whitespace)
}

fn sanitize_inline(chars: &[char]) -> Vec<char> {
    let mut out = chars.to_vec();
    let mut index = 0;
    while index < out.len() {
        // Inline code: `code`.
        if out[index] == '`' {
            let run = out[index..].iter().take_while(|ch| **ch == '`').count();
            if let Some(close) = find_tick_run(&out, index + run, run) {
                blank(&mut out, index, close + run);
                index = close + run;
                continue;
            }
            index += run.max(1);
            continue;
        }

        // Markdown link destination: `[label](destination)`.
        if out[index] == ']' && out.get(index + 1) == Some(&'(') {
            if let Some(close) = out[index + 2..].iter().position(|ch| *ch == ')') {
                let end = index + 2 + close;
                blank(&mut out, index + 1, end + 1);
                index = end + 1;
                continue;
            }
        }

        // Obsidian embeds and wiki links are file references, not prose.
        if out[index] == '!' && out.get(index + 1) == Some(&'[') && out.get(index + 2) == Some(&'[')
        {
            if let Some(close) = find_chars(&out, index + 3, "]]") {
                blank(&mut out, index, close + 2);
                index = close + 2;
                continue;
            }
        }
        if out[index] == '[' && out.get(index + 1) == Some(&'[') {
            if let Some(close) = find_chars(&out, index + 2, "]]") {
                blank(&mut out, index, close + 2);
                index = close + 2;
                continue;
            }
        }

        // Callout marker: `[!NOTE]`. The custom title after it stays checked.
        if out[index] == '[' && out.get(index + 1) == Some(&'!') {
            if let Some(close) = out[index + 2..].iter().position(|ch| *ch == ']') {
                let end = index + 2 + close;
                blank(&mut out, index, end + 1);
                index = end + 1;
                continue;
            }
        }

        // Autolinks/HTML tags and inline math are not prose.
        if out[index] == '<' {
            if let Some(close) = out[index + 1..].iter().position(|ch| *ch == '>') {
                let end = index + 1 + close;
                let inner: String = out[index + 1..end].iter().collect();
                if inner.contains("://") || inner.contains('@') || !inner.contains(' ') {
                    blank(&mut out, index, end + 1);
                    index = end + 1;
                    continue;
                }
            }
        }
        if out[index] == '$' {
            if let Some(close) = out[index + 1..].iter().position(|ch| *ch == '$') {
                let end = index + 1 + close;
                blank(&mut out, index, end + 1);
                index = end + 1;
                continue;
            }
        }

        // Bare URLs.
        if starts_with(&out, index, "http://") || starts_with(&out, index, "https://") {
            let mut end = index;
            while end < out.len()
                && !out[end].is_whitespace()
                && !matches!(out[end], ')' | ']' | '}' | '>' | '"' | '\'' | '<')
            {
                end += 1;
            }
            blank(&mut out, index, end);
            index = end;
            continue;
        }

        index += 1;
    }
    out
}

fn find_tick_run(chars: &[char], start: usize, run: usize) -> Option<usize> {
    let mut index = start;
    while index < chars.len() {
        if chars[index] == '`' {
            let found = chars[index..].iter().take_while(|ch| **ch == '`').count();
            if found == run {
                return Some(index);
            }
            index += found;
        } else {
            index += 1;
        }
    }
    None
}

fn find_chars(chars: &[char], start: usize, needle: &str) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    (start..chars.len()).find(|index| {
        needle
            .iter()
            .enumerate()
            .all(|(offset, ch)| chars.get(index + offset) == Some(ch))
    })
}

fn starts_with(chars: &[char], start: usize, needle: &str) -> bool {
    needle
        .chars()
        .enumerate()
        .all(|(offset, ch)| chars.get(start + offset) == Some(&ch))
}

fn blank(chars: &mut [char], start: usize, end: usize) {
    let len = chars.len();
    for ch in &mut chars[start.min(len)..end.min(len)] {
        *ch = ' ';
    }
}

fn run_hunspell(lines: &[Vec<char>], language: &str) -> Result<String> {
    let mut input = String::new();
    for line in lines {
        // A leading space prevents ispell pipe commands (`#`, `*`, `+`, ...)
        // from being interpreted when a Markdown line starts with them.
        input.push(' ');
        input.extend(line);
        input.push('\n');
    }
    run_hunspell_text(&input, language)
}

fn run_hunspell_text(input: &str, language: &str) -> Result<String> {
    let mut child = Command::new("hunspell")
        .args(["-a", "-d", language])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| "failed to start hunspell (is it installed?)")?;

    // Write stdin from a separate thread so a large document cannot fill
    // hunspell's stdout pipe while we are still blocked writing to stdin.
    let mut stdin = child
        .stdin
        .take()
        .context("hunspell stdin was not available")?;
    let input = input.to_owned();
    let writer = std::thread::spawn(move || -> std::io::Result<()> {
        stdin.write_all(input.as_bytes())?;
        drop(stdin);
        Ok(())
    });

    let output = child
        .wait_with_output()
        .context("failed to read hunspell output")?;
    writer
        .join()
        .map_err(|_| anyhow::anyhow!("hunspell writer thread panicked"))?
        .context("failed to write text to hunspell")?;
    if !output.status.success() {
        bail!("hunspell exited with status {}", output.status);
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_hunspell_output(output: &str, lines: &[Vec<char>]) -> Vec<SpellIssue> {
    let mut issues = Vec::new();
    let mut line_index = 0;
    for raw in output.lines() {
        if raw.starts_with("@(#)") {
            continue;
        }
        if raw.is_empty() {
            line_index += 1;
            continue;
        }
        if let Some(rest) = raw.strip_prefix("& ") {
            if let Some((meta, suggestion_text)) = rest.split_once(':') {
                if let Some((word, offset)) = parse_word_offset(meta) {
                    push_issue(
                        &mut issues,
                        lines,
                        line_index,
                        word,
                        offset,
                        parse_suggestions(suggestion_text),
                    );
                }
            }
        } else if let Some(rest) = raw.strip_prefix("# ") {
            if let Some((word, offset)) = parse_word_offset(rest) {
                push_issue(&mut issues, lines, line_index, word, offset, Vec::new());
            }
        }
    }
    issues
}

fn parse_word_offset(meta: &str) -> Option<(&str, usize)> {
    let mut fields = meta.split_whitespace();
    let word = fields.next()?;
    // `& word count offset` or `# word offset`.
    let offset = fields.last()?.parse::<usize>().ok()?;
    Some((word, offset))
}

fn push_issue(
    issues: &mut Vec<SpellIssue>,
    lines: &[Vec<char>],
    line_index: usize,
    word: &str,
    offset: usize,
    suggestions: Vec<String>,
) {
    let Some(line) = lines.get(line_index) else {
        return;
    };
    // Strip the sentinel space added to every input line.
    let start = offset.saturating_sub(1).min(line.len());
    let mut end = start;
    while end < line.len() && is_word_char(line[end]) {
        end += 1;
    }
    if end == start {
        end = (start + word.chars().count()).min(line.len());
    }
    if end <= start {
        return;
    }
    issues.push(SpellIssue {
        line: line_index,
        start,
        end,
        word: word.to_string(),
        suggestions,
    });
}

fn parse_suggestions(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|suggestion| !suggestion.is_empty())
        .take(MAX_SUGGESTIONS)
        .map(str::to_string)
        .collect()
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '\'' || ch == '\u{2019}'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(line: &str) -> Vec<char> {
        line.chars().collect()
    }

    #[test]
    fn parses_hunspell_offsets_and_suggestions() {
        let lines = vec![chars("hello wrld"), chars("another wrld")];
        // Offsets include the sentinel space `run_hunspell` adds to every
        // line, hence 7/9 instead of the raw 6/8.
        let output =
            "@(#) banner\n*\n& wrld 6 7: weld, world, wild\n\n*\n& wrld 6 9: weld, world\n\n";
        let issues = parse_hunspell_output(output, &lines);
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].line, 0);
        assert_eq!((issues[0].start, issues[0].end), (6, 10));
        assert_eq!(issues[0].word, "wrld");
        assert_eq!(issues[0].suggestions, vec!["weld", "world", "wild"]);
        assert_eq!(issues[1].line, 1);
        assert_eq!((issues[1].start, issues[1].end), (8, 12));
    }

    #[test]
    fn sanitizes_code_urls_and_callout_markers() {
        let line = chars("`let x = 1` ![alt](https://example.com/a.png) [!NOTE] title");
        let sanitized: String = sanitize_inline(&line).iter().collect();
        assert!(sanitized.contains("alt"));
        assert!(sanitized.contains("title"));
        assert!(!sanitized.contains("let x"));
        assert!(!sanitized.contains("example.com"));
        assert!(!sanitized.contains("[!NOTE]"));
        assert_eq!(sanitized.chars().count(), line.len());
    }

    #[test]
    fn sanitized_lines_keep_offsets_and_skip_fences() {
        let text = Rope::from("# Heading\n\n```rust\nlet code = 1;\n```\n\nplain wrld\n");
        let lines = sanitized_lines(&text);
        assert_eq!(lines.len(), text.len_lines());
        assert!(lines[0].iter().collect::<String>().contains("Heading"));
        assert!(lines[3].iter().all(|ch| *ch == ' '));
        assert!(lines[6].iter().collect::<String>().contains("wrld"));
    }

    #[test]
    fn finds_word_at_cursor() {
        let text = Rope::from("hello wrld\n");
        assert_eq!(word_at(&text, 7), Some((6, 10, "wrld".to_string())));
        // On whitespace, fall back to the preceding word.
        assert_eq!(word_at(&text, 5), Some((0, 5, "hello".to_string())));
        assert_eq!(word_at(&Rope::from(""), 0), None);
    }

    #[test]
    fn prose_detection_uses_extension_without_a_language() {
        assert!(is_prose_for(None, Some(Path::new("/tmp/note.txt"))));
        assert!(is_prose_for(None, Some(Path::new("/tmp/note.md"))));
        assert!(!is_prose_for(None, Some(Path::new("/tmp/main.rs"))));
        assert!(is_prose_for(Some("markdown"), None));
        assert!(!is_prose_for(Some("rust"), None));
    }
}
