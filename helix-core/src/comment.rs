use crate::{
    find_first_non_whitespace_char, Change, Rope, RopeSlice, Selection, Tendril, Transaction,
};
use std::borrow::Cow;

fn find_line_comment(
    token: &str,
    text: RopeSlice,
    lines: impl IntoIterator<Item = usize>,
) -> (bool, Vec<usize>, usize, usize) {
    let mut commented = true;
    let mut to_change = Vec::new();
    let mut min = usize::MAX; // minimum col for find_first_non_whitespace_char
    let mut margin = 1;
    let token_len = token.chars().count();
    for line in lines {
        let line_slice = text.line(line);
        if let Some(pos) = find_first_non_whitespace_char(line_slice) {
            let len = line_slice.len_chars();

            if pos < min {
                min = pos;
            }

            // line can be shorter than pos + token len
            let fragment = Cow::from(line_slice.slice(pos..std::cmp::min(pos + token.len(), len)));

            if fragment != token {
                // as soon as one of the non-blank lines doesn't have a comment, the whole block is
                // considered uncommented.
                commented = false;
            }

            // determine margin of 0 or 1 for uncommenting; if any comment token is not followed by a space,
            // a margin of 0 is used for all lines.
            if matches!(line_slice.get_char(pos + token_len), Some(c) if c != ' ') {
                margin = 0;
            }

            // blank lines don't get pushed.
            to_change.push(line);
        }
    }
    (commented, to_change, min, margin)
}

#[must_use]
pub fn toggle_line_comments(doc: &Rope, selection: &Selection, token: Option<&str>) -> Transaction {
    let text = doc.slice(..);

    let token = token.unwrap_or("//");
    let comment = Tendril::from(format!("{} ", token));

    let mut lines: Vec<usize> = Vec::new();

    let mut min_next_line = 0;
    for selection in selection {
        let start = text.char_to_line(selection.from()).max(min_next_line);
        let end = text.char_to_line(selection.to()) + 1;
        lines.extend(start..end);
        min_next_line = end + 1;
    }

    let (commented, to_change, min, margin) = find_line_comment(&token, text, lines);

    let mut changes: Vec<Change> = Vec::with_capacity(to_change.len());

    for line in to_change {
        let pos = text.line_to_char(line) + min;

        if !commented {
            // comment line
            changes.push((pos, pos, Some(comment.clone())));
        } else {
            // uncomment line
            changes.push((pos, pos + token.len() + margin, None));
        }
    }

    Transaction::change(doc, changes.into_iter())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_line_comment() {
        use crate::State;

        // four lines, two space indented, except for line 1 which is blank.
        let doc = Rope::from("  1\n\n  2\n  3");

        let mut state = State::new(doc);
        // select whole document
        state.selection = Selection::single(0, state.doc.len_chars() - 1);

        let text = state.doc.slice(..);

        let res = find_line_comment("//", text, 0..3);
        // (commented = true, to_change = [line 0, line 2], min = col 2, margin = 1)
        assert_eq!(res, (false, vec![0, 2], 2, 1));

        // comment
        let transaction = toggle_line_comments(&state.doc, &state.selection, None);
        transaction.apply(&mut state.doc);
        state.selection = state.selection.map(transaction.changes());

        assert_eq!(state.doc, "  // 1\n\n  // 2\n  // 3");

        // uncomment
        let transaction = toggle_line_comments(&state.doc, &state.selection, None);
        transaction.apply(&mut state.doc);
        state.selection = state.selection.map(transaction.changes());
        assert_eq!(state.doc, "  1\n\n  2\n  3");

        // 0 margin comments
        state.doc = Rope::from("  //1\n\n  //2\n  //3");
        // reset the selection.
        state.selection = Selection::single(0, state.doc.len_chars() - 1);

        let transaction = toggle_line_comments(&state.doc, &state.selection, None);
        transaction.apply(&mut state.doc);
        state.selection = state.selection.map(transaction.changes());
        assert_eq!(state.doc, "  1\n\n  2\n  3");

        // TODO: account for uncommenting with uneven comment indentation
    }
}
