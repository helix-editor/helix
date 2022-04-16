use ropey::RopeSlice;

/// Given a slice of text, return the text re-wrapped to fit it
/// within the given width.
pub fn reflow_hard_wrap(text: RopeSlice, max_line_len: usize) -> String {
    // TODO: We should handle CRLF line endings.

    let text = String::from(text);
    let prefix = detect_prefix(&text);

    // The current algorithm eats a single trailing newline character.
    // Thisis a hacky way to put it back at the end. Ideally there would be
    // a cleaner way to do this. It also happens to matter a lot because
    // Helix currently selects lines *including* the trailing newline.
    let ends_with_newline = text.chars().rev().next() == Some('\n');

    let flow_pieces = separate_flow_pieces(&text, &prefix).into_iter().peekable();

    let mut new_flow = String::new();
    let mut current_line_len: usize = 0;

    for piece in flow_pieces {
        match piece {
            Piece::NoReflow(piece) => {
                // Get rid of spaces at the ends of lines.
                match new_flow.pop() {
                    Some(c) if c == ' ' => (),
                    Some(c) => new_flow.push(c),
                    None => unreachable!(),
                }
                new_flow.push('\n');
                new_flow.push_str(piece);
                current_line_len = 0;
                continue;
            }
            Piece::Reflow(piece) => {
                use unicode_segmentation::UnicodeSegmentation;
                let piece_len = UnicodeSegmentation::graphemes(piece, true).count();

                if piece.is_empty() {
                    continue;
                }

                let piece_will_fit = current_line_len + piece_len <= max_line_len;
                if !piece_will_fit && !new_flow.is_empty() {
                    // Get rid of spaces at the ends of lines.
                    match new_flow.pop() {
                        Some(c) if c == ' ' => (),
                        Some(c) => new_flow.push(c),
                        None => unreachable!(),
                    }

                    new_flow.push('\n');
                    current_line_len = 0;
                }

                if current_line_len == 0 {
                    new_flow.push_str(&prefix);
                }

                new_flow.push_str(piece);

                if !piece.chars().rev().next().unwrap().is_whitespace() {
                    new_flow.push(' ');
                }

                current_line_len += piece_len;
            }
        }
    }

    // Get rid of the space at the very end.
    match new_flow.pop() {
        Some(c) if c == ' ' => (),
        Some(c) => new_flow.push(c),
        None => (),
    }

    if ends_with_newline {
        new_flow.push('\n');
    }

    new_flow
}

#[cfg_attr(test, derive(Debug, PartialEq))]
enum Piece<'a> {
    Reflow(&'a str),
    NoReflow(&'a str),
}

fn detect_prefix(text: &str) -> String {
    // TODO: We should detect the configured comment prefixes here.

    if text.is_empty() {
        return String::new();
    }

    // For now, we'll only detect the prefix by the string (possibly empty)
    // that is present at the beginning of every selected line, excluding
    // lines that are entirely blank.

    // UNWRAP: This is OK because we already checked to see if the string is empty.
    let first_line = text.lines().next().unwrap();
    text.lines()
        // TODO(decide): Should `is_whitespace` be `is_ascii_whitespace` in this next line?
        .filter(|line| !line.chars().all(|ch| ch.is_whitespace()))
        .fold(first_line, |prefix, line| {
            // If the prefix is already empty, we won't bother checking what portion
            // of it matches with the current line.
            if prefix.is_empty() {
                return prefix;
            }

            let matched_until_excl: usize = prefix
                .chars()
                .enumerate()
                .zip(line.chars())
                .take_while(|((_prefix_char_idx, prefix_char), line_char)| {
                    // TODO(decide): Do we want to relax the restriction that the
                    // "prefix" for these lines must be entirely non-ascii-alphanumeric?
                    prefix_char == line_char && !prefix_char.is_ascii_alphanumeric()
                })
                // add 1 here to be "one past the end" of the prefix
                .map(|((prefix_char_idx, _), _)| prefix_char_idx + 1)
                .last()
                .unwrap_or(0);

            prefix.get(..matched_until_excl).unwrap()
        })
        .into()
}

// PANIC: This function may panic if the `prefix` does not begin every
// non-blank line, where "blank" here means a line that has
// non-whitespace characters *after* the prefix. A line with only the
// prefix and whitespace characters is considered blank.
fn separate_flow_pieces<'a>(text: &'a str, prefix: &str) -> Vec<Piece<'a>> {
    use separated::Separated;

    let mut flow_pieces = Vec::<Piece>::new();
    let lines = Separated::new(text, "\n");
    for line in lines {
        match line.get(..prefix.len()) {
            Some(possible_prefix) if possible_prefix == prefix => {
                // If the rest of the line is whitespace, we count it as an effectively
                // "blank" line, and we don't want to reflow it.
                if line[prefix.len()..].chars().all(char::is_whitespace) {
                    flow_pieces.push(Piece::NoReflow(line));
                } else {
                    // UNWRAP: The `detect_prefix` function should ensure that
                    // this unwrap is valid. The "prefix" should begin every line
                    // except for entirely blank lines.
                    let line_no_prefix = line.get(prefix.len()..).unwrap();
                    let new_pieces = Separated::new(line_no_prefix, " ")
                        .map(|s| Piece::Reflow(s.trim_end_matches('\n').trim_end_matches('\r')));
                    flow_pieces.extend(new_pieces);
                }
            }
            _ => {
                assert!(
                    line.chars().all(char::is_whitespace),
                    "lines not matching the prefix should be blank"
                );

                flow_pieces.push(Piece::NoReflow(line));
            }
        }
    }

    flow_pieces
}

mod separated {
    // Like the std::str::Lines iterator except it doesn't eat the newline characters.
    pub(crate) struct Separated<'a, 'b> {
        s: &'a str,
        pattern: &'b str,
        beg: usize,
    }

    impl<'a, 'b> Separated<'a, 'b> {
        pub(crate) fn new(s: &'a str, pattern: &'b str) -> Self {
            Self { s, pattern, beg: 0 }
        }
    }

    impl<'a, 'b> Iterator for Separated<'a, 'b> {
        type Item = &'a str;

        fn next(&mut self) -> Option<Self::Item> {
            let rest = self.s.get(self.beg..)?;

            if rest.is_empty() {
                return None;
            }

            let end = rest
                .find(self.pattern)
                .unwrap_or_else(|| rest.len().saturating_sub(1));

            self.beg += end + 1;
            Some(&rest[..=end])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    #[test]
    fn reflow_basic_to_one_line() {
        let text = "hello my name\nis helix";
        let text = Rope::from(text);
        let reflow = reflow_hard_wrap(text.slice(..), 100);
        assert_eq!(reflow, "hello my name is helix");
    }

    #[test]
    fn reflow_basic_to_one_line_with_trailing_space() {
        let text = "hello my name \nis helix";
        let text = Rope::from(text);
        let reflow = reflow_hard_wrap(text.slice(..), 100);
        assert_eq!(reflow, "hello my name is helix");
    }

    #[test]
    fn reflow_basic_to_many_lines() {
        let text = "hello my name is helix";
        let text = Rope::from(text);
        let reflow = reflow_hard_wrap(text.slice(..), 10);
        assert_eq!(reflow, "hello my\nname is\nhelix");
    }

    #[test]
    fn reflow_with_blank_empty_line() {
        let text = "hello\n\nmy name is helix";
        let text = Rope::from(text);
        let reflow = reflow_hard_wrap(text.slice(..), 10);
        assert_eq!(reflow, "hello\n\nmy name\nis helix");
    }

    #[test]
    fn reflow_with_blank_whitespace_line() {
        let text = "hello\n  \nmy name is helix";
        let text = Rope::from(text);
        let reflow = reflow_hard_wrap(text.slice(..), 10);
        assert_eq!(reflow, "hello\n  \nmy name\nis helix");
    }

    #[test]
    fn reflow_end_with_blank_line() {
        let text = "hello my name is helix\n";
        let text = Rope::from(text);
        let reflow = reflow_hard_wrap(text.slice(..), 10);
        assert_eq!(reflow, "hello my\nname is\nhelix\n");
    }

    #[test]
    fn reflow_with_blank_lines_and_prefix() {
        let text = "  hello\n\nmy name is helix";
        let text = Rope::from(text);
        let reflow = reflow_hard_wrap(text.slice(..), 10);
        assert_eq!(reflow, "  hello\n\nmy name\nis helix");
    }

    #[test]
    fn reflow_to_many_lines_with_whitespace_prefix() {
        let text = Rope::from("\t Text indented. \n\t Still indented.");
        let expected_reflow = "\t Text indented. Still indented.";
        let reflow = reflow_hard_wrap(text.slice(..), 80);
        assert_eq!(reflow, expected_reflow);
    }

    #[test]
    fn reflow_to_many_lines_with_whitespace_and_comment_prefix() {
        let text = Rope::from("// Text indented. \n// Still indented.");
        let expected_reflow = "// Text indented. Still indented.";
        let text = Rope::from(text);
        let reflow = reflow_hard_wrap(text.slice(..), 80);
        assert_eq!(reflow, expected_reflow);
    }

    #[test]
    fn reflow_empty() {
        let text = Rope::from("");
        let reflow = reflow_hard_wrap(text.slice(..), 10);
        assert_eq!(reflow, "");
    }

    #[test]
    fn reflow_max_line_length_zero() {
        let text = "hello my name is helix";
        let text = Rope::from(text);
        let reflow = reflow_hard_wrap(text.slice(..), 0);
        assert_eq!(reflow, "hello\nmy\nname\nis\nhelix");
    }

    #[test]
    fn reflow_comment_after_blank_line() {
        let text = Rope::from("// Text indented. \n\n// Still indented.");
        let expected_reflow = "// Text indented.\n\n// Still indented.";
        let text = Rope::from(text);
        let reflow = reflow_hard_wrap(text.slice(..), 80);
        assert_eq!(reflow, expected_reflow);
    }

    #[test]
    fn detect_prefix_no_indent() {
        let text = Rope::from("hello my name is helix");
        let prefix = detect_prefix(text.to_string().as_str());
        assert_eq!(prefix, "");
    }

    #[test]
    fn detect_prefix_spaces_indent() {
        let text = Rope::from("   hello my name is helix");
        let prefix = detect_prefix(text.to_string().as_str());
        assert_eq!(prefix, "   ");
    }

    #[test]
    fn detect_prefix_tabs_indent() {
        let text = Rope::from("\t\t\thello my name is helix");
        let prefix = detect_prefix(text.to_string().as_str());
        assert_eq!(prefix, "\t\t\t");
    }

    #[test]
    fn detect_prefix_spaces_with_tabs() {
        let text = Rope::from("  \t\thello my name is helix");
        let prefix = detect_prefix(text.to_string().as_str());
        assert_eq!(prefix, "  \t\t");
    }

    #[test]
    fn detect_prefix_tabs_with_spaces_indent() {
        let text = Rope::from("\t\t  hello my name is helix");
        let prefix = detect_prefix(text.to_string().as_str());
        assert_eq!(prefix, "\t\t  ");
    }

    #[test]
    fn detect_prefix_many_lines_with_comment_then_space() {
        let text = Rope::from("// Text indented.\n// Still indented.");
        let prefix = detect_prefix(text.to_string().as_str());
        assert_eq!(prefix, "// ");
    }

    #[test]
    fn detect_prefix_unfinished_final_line() {
        let text = Rope::from("// Text indented.\n// Still indented.\n");
        let prefix = detect_prefix(text.to_string().as_str());
        assert_eq!(prefix, "// ");
    }

    #[test]
    fn flow_pieces_basic() {
        let text = "one two three";
        let pieces = separate_flow_pieces(text, "");
        assert_eq!(
            pieces,
            Vec::from([
                Piece::Reflow("one "),
                Piece::Reflow("two "),
                Piece::Reflow("three")
            ])
        );
    }

    #[test]
    fn flow_pieces_with_blank_line() {
        let text = "one\n\ntwo";
        let pieces = separate_flow_pieces(text, "");
        assert_eq!(
            pieces,
            Vec::from([
                Piece::Reflow("one"),
                Piece::NoReflow("\n"),
                Piece::Reflow("two"),
            ])
        );
    }
}
