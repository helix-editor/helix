use crate::{Rope, Transaction};

/// Compares `old` and `new` to generate a [`Transaction`] describing
/// the steps required to get from `old` to `new`.
pub fn compare_ropes(old: &Rope, new: &Rope) -> Transaction {
    // `similar` only works on contiguous data, so a `Rope` has
    // to be temporarily converted into a `String`.
    let old_converted = old.to_string();
    let new_converted = new.to_string();

    // A timeout is set so after 1 seconds, the algorithm will start
    // approximating. This is especially important for big `Rope`s or
    // `Rope`s that are extremely dissimilar to each other.
    let mut config = similar::TextDiff::configure();
    config.timeout(std::time::Duration::from_secs(1));

    let diff = config.diff_chars(&old_converted, &new_converted);

    // The current position of the change needs to be tracked to
    // construct the `Change`s.
    let mut pos = 0;
    Transaction::change(
        old,
        diff.ops()
            .iter()
            .map(|op| op.as_tag_tuple())
            .filter_map(|(tag, old_range, new_range)| {
                // `old_pos..pos` is equivalent to `start..end` for where
                // the change should be applied.
                let old_pos = pos;
                pos += old_range.end - old_range.start;

                match tag {
                    // Semantically, inserts and replacements are the same thing.
                    similar::DiffTag::Insert | similar::DiffTag::Replace => {
                        // This is the text from the `new` rope that should be
                        // inserted into `old`.
                        let text: &str = {
                            let start = new.char_to_byte(new_range.start);
                            let end = new.char_to_byte(new_range.end);
                            &new_converted[start..end]
                        };
                        Some((old_pos, pos, Some(text.into())))
                    }
                    similar::DiffTag::Delete => Some((old_pos, pos, None)),
                    similar::DiffTag::Equal => None,
                }
            }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    quickcheck::quickcheck! {
        fn test_compare_ropes(a: String, b: String) -> bool {
            let mut old = Rope::from(a);
            let new = Rope::from(b);
            compare_ropes(&old, &new).apply(&mut old);
            old == new
        }
    }
}
