use ropey::Rope;

use crate::{Change, Transaction};

pub fn diff_ropes(old: &Rope, new: &Rope) -> Transaction {
    let old_into_string = old.to_string();
    let new_into_string = new.to_string();

    let mut config = similar::TextDiff::configure();
    config.timeout(std::time::Duration::new(2, 0));

    let diff = config.diff_chars(&old_into_string, &new_into_string);
    let changes: Vec<Change> = diff
        .ops()
        .iter()
        .filter_map(|op| {
            let (tag, old_range, new_range) = op.as_tag_tuple();
            match tag {
                similar::DiffTag::Insert | similar::DiffTag::Replace => {
                    let text: &str = {
                        let start = new.char_to_byte(new_range.start);
                        let end = new.char_to_byte(new_range.end);
                        &new_into_string[start..end]
                    };
                    Some((old_range.start, old_range.end, Some(text.into())))
                }
                similar::DiffTag::Delete => Some((old_range.start, old_range.end, None)),
                similar::DiffTag::Equal => None,
            }
        })
        .collect();

    Transaction::change(old, changes.into_iter())
}
