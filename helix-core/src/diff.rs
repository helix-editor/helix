use ropey::Rope;

use crate::{Change, Transaction};

/// Compares `old` and `new` to generate a [`Transaction`] describing
/// the steps required to get from `old` to `new`.
pub fn compare_ropes(old: &Rope, new: &Rope) -> Transaction {
    // `similar` only works on contiguous data, so a `Rope` has
    // to be temporarily converted into a `String` until the diff
    // is created.
    let old_as_chars: Vec<_> = old.chars().collect();
    let new_as_chars: Vec<_> = new.chars().collect();

    // A timeout is set so after 2 seconds, the algorithm will start
    // approximating. This is especially important for big `Rope`s or
    // `Rope`s that are extremely dissimilar so the diff will be
    // created in a reasonable amount of time.
    // let mut config = similar::TextDiff::configure();
    // config.timeout(std::time::Duration::new(2, 0));

    // Note: Ignore the clippy warning, as the trait bounds of
    // `Transaction::change()` require an iterator implementing
    // `ExactIterator`.

    let time = std::time::Instant::now() + std::time::Duration::new(10, 0);
    let diff = similar::capture_diff_slices_deadline(
        similar::Algorithm::Myers,
        &old_as_chars,
        &new_as_chars,
        Some(time),
    );
    let changes: Vec<Change> = diff
        .iter()
        .filter_map(|op| {
            let (tag, old_range, new_range) = op.as_tag_tuple();
            match tag {
                // Semantically, inserts and replacements are the same thing.
                similar::DiffTag::Insert | similar::DiffTag::Replace => {
                    // This is the text from the `new` rope that should be
                    // inserted into `old`.
                    let text: String = new_as_chars[new_range].iter().collect();
                    Some((old_range.start, old_range.end, Some(text.into())))
                }
                similar::DiffTag::Delete => Some((old_range.start, old_range.end, None)),
                similar::DiffTag::Equal => None,
            }
        })
        .collect();
    std::fs::write("derp.txt", format!("{:#?}", diff)).unwrap();
    Transaction::change(old, changes.into_iter())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 10000, .. ProptestConfig::default()
        })]
        #[test]
        fn test_compare_ropes(a: String, b: String) {
            let mut old = Rope::from(a);
            let new = Rope::from(b);
            compare_ropes(&old, &new).apply(&mut old);
            prop_assert_eq!(old.to_string(), new.to_string());
        }
    }
}
