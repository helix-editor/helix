use crate::{
    find_first_non_whitespace_char, Change, Rope, RopeSlice, Selection, Tendril, Transaction,
};
use core::ops::Range;
use std::borrow::Cow;

fn find_line_comment(
    token: &str,
    text: RopeSlice,
    lines: Range<usize>,
) -> (bool, Vec<usize>, usize) {
    let mut commented = true;
    let mut skipped = Vec::new();
    let mut min = usize::MAX; // minimum col for find_first_non_whitespace_char
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
        } else {
            // blank line
            skipped.push(line);
        }
    }
    (commented, skipped, min)
}

#[must_use]
pub fn toggle_line_comments(doc: &Rope, selection: &Selection, token: Option<&str>) -> Transaction {
    let text = doc.slice(..);
    let mut changes: Vec<Change> = Vec::new();

    let token = token.unwrap_or("//");
    let comment = Tendril::from(format!("{} ", token));

    for selection in selection {
        let (start, end) = selection.line_range(text);
        let lines = start..end + 1;
        let (commented, skipped, min) = find_line_comment(&token, text, lines.clone());

        changes.reserve((end - start).saturating_sub(skipped.len()));

        for line in lines {
            if skipped.contains(&line) {
                continue;
            }

            let pos = text.line_to_char(line) + min;

            if !commented {
                // comment line
                changes.push((pos, pos, Some(comment.clone())))
            } else {
                // uncomment line
                let margin = 1; // TODO: margin is hardcoded 1 but could easily be 0
                changes.push((pos, pos + token.len() + margin, None))
            }
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
        // (commented = true, skipped = [line 1], min = col 2)
        assert_eq!(res, (false, vec![1], 2));

        // comment
        let transaction = toggle_line_comments(&state.doc, &state.selection, None);
        transaction.apply(&mut state.doc);
        state.selection = state.selection.clone().map(transaction.changes());

        assert_eq!(state.doc, "  // 1\n\n  // 2\n  // 3");

        // uncomment
        let transaction = toggle_line_comments(&state.doc, &state.selection, None);
        transaction.apply(&mut state.doc);
        state.selection = state.selection.clone().map(transaction.changes());
        assert_eq!(state.doc, "  1\n\n  2\n  3");

        // TODO: account for no margin after comment
        // TODO: account for uncommenting with uneven comment indentation
    }
}
