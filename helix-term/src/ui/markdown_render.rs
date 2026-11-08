//! Lightweight markdown rendering for the editor.
//!
//! This replaces self-describing markdown markers with icons or frame
//! characters. It intentionally uses single-grapheme overlays so the buffer
//! text (and the cursor) is unchanged.

use helix_core::text_annotations::Overlay;
use helix_view::Document;

pub type Layer = (&'static str, Vec<Overlay>);

/// Build overlay layers for the visible section of a markdown document.
pub fn overlays(doc: &Document, anchor: usize, height: u16) -> Vec<Layer> {
    if doc.language_name() != Some("markdown") {
        return Vec::new();
    }

    let text = doc.text().slice(..);
    if text.len_lines() == 0 {
        return Vec::new();
    }

    let anchor = anchor.min(text.len_chars());
    let start = text.char_to_line(anchor);
    let end = (start + height as usize + 2).min(text.len_lines());

    let mut heading = Vec::new();
    let mut list = Vec::new();
    let mut checkbox = Vec::new();
    let mut quote = Vec::new();
    let mut rule = Vec::new();

    for line_idx in start..end {
        let line_start = text.line_to_char(line_idx);
        let raw = text.line(line_idx).to_string();
        let line = raw.trim_end_matches(['\r', '\n']);
        let chars: Vec<char> = line.chars().collect();
        let indent = chars.iter().take_while(|ch| **ch == ' ').count();
        if indent >= chars.len() {
            continue;
        }

        // Headings: `### Title`.
        if chars[indent] == '#' {
            let level = chars[indent..].iter().take_while(|ch| **ch == '#').count();
            let after = chars.get(indent + level);
            if (1..=6).contains(&level) && matches!(after, None | Some(' ')) {
                heading.push(Overlay::new(line_start + indent, heading_icon(level)));
                for offset in (indent + 1)..(indent + level) {
                    heading.push(Overlay::new(line_start + offset, " "));
                }
                continue;
            }
        }

        // Task lists: `- [ ] todo` / `- [x] done`.
        if indent + 4 < chars.len()
            && matches!(chars[indent], '-' | '*' | '+')
            && chars[indent + 1] == ' '
            && chars[indent + 2] == '['
            && matches!(chars[indent + 3], ' ' | 'x' | 'X')
            && chars[indent + 4] == ']'
        {
            let checked = chars[indent + 3] != ' ';
            checkbox.push(Overlay::new(
                line_start + indent,
                if checked { "󰱒" } else { "󰄱" },
            ));
            for offset in (indent + 1)..=(indent + 4) {
                checkbox.push(Overlay::new(line_start + offset, " "));
            }
            continue;
        }

        // Bullet lists.
        if matches!(chars[indent], '-' | '*' | '+') && chars.get(indent + 1) == Some(&' ') {
            list.push(Overlay::new(line_start + indent, "•"));
            continue;
        }

        // Block quotes, including nested `>>`.
        let mut cursor = indent;
        let mut quoted = false;
        while chars.get(cursor) == Some(&'>') {
            quote.push(Overlay::new(line_start + cursor, "│"));
            quoted = true;
            cursor += 1;
            if chars.get(cursor) == Some(&' ') {
                cursor += 1;
            }
        }
        if quoted {
            continue;
        }

        // Horizontal rules.
        let trimmed = &chars[indent..];
        if trimmed.len() >= 3
            && matches!(trimmed[0], '-' | '*' | '_')
            && trimmed.iter().all(|ch| *ch == trimmed[0])
        {
            for offset in indent..chars.len() {
                rule.push(Overlay::new(line_start + offset, "─"));
            }
        }
    }

    [
        ("markup.heading", heading),
        ("markup.list", list),
        ("markup.list", checkbox),
        ("markup.quote", quote),
        ("markup.raw", rule),
    ]
    .into_iter()
    .filter(|(_, overlays)| !overlays.is_empty())
    .collect()
}

fn heading_icon(level: usize) -> &'static str {
    match level {
        1 => "󰲡",
        2 => "󰲣",
        3 => "󰲥",
        4 => "󰲧",
        5 => "󰲩",
        _ => "󰲫",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_icons_cover_all_levels() {
        for level in 1..=6 {
            assert_eq!(heading_icon(level).chars().count(), 1);
        }
    }
}
