//! Test helpers.
use crate::{Range, Selection};
use smallvec::SmallVec;
use std::cmp::Reverse;

/// Convert annotated test string to test string and selection.
///
/// `^` for `anchor` and `|` for head (`@` for primary), both must appear
/// or otherwise it will panic.
///
/// # Examples
///
/// ```
/// use helix_core::{Range, Selection, test::print};
/// use smallvec::smallvec;
///
/// assert_eq!(
///     print("^a@b|c^"),
///     ("abc".to_owned(), Selection::new(smallvec![Range::new(0, 1), Range::new(3, 2)], 0))
/// );
/// ```
///
/// # Panics
///
/// Panics when missing primary or appeared more than once.
/// Panics when missing head or anchor.
/// Panics when head come after head or anchor come after anchor.
pub fn print(s: &str) -> (String, Selection) {
    let mut anchor = None;
    let mut head = None;
    let mut primary = None;
    let mut ranges = SmallVec::new();
    let mut i = 0;
    let s = s
        .chars()
        .filter(|c| {
            match c {
                '^' if anchor != None => panic!("anchor without head {s:?}"),
                '^' if head == None => anchor = Some(i),
                '^' => ranges.push(Range::new(i, head.take().unwrap())),
                '|' if head != None => panic!("head without anchor {s:?}"),
                '|' if anchor == None => head = Some(i),
                '|' => ranges.push(Range::new(anchor.take().unwrap(), i)),
                '@' if primary != None => panic!("head (primary) already appeared {s:?}"),
                '@' if head != None => panic!("head (primary) without anchor {s:?}"),
                '@' if anchor == None => {
                    primary = Some(ranges.len());
                    head = Some(i);
                }
                '@' => {
                    primary = Some(ranges.len());
                    ranges.push(Range::new(anchor.take().unwrap(), i));
                }
                _ => {
                    i += 1;
                    return true;
                }
            };
            false
        })
        .collect();
    if head.is_some() {
        panic!("missing anchor (|) {s:?}");
    }
    if anchor.is_some() {
        panic!("missing head (^) {s:?}");
    }
    let primary = match primary {
        Some(i) => i,
        None => panic!("missing primary (@) {s:?}"),
    };
    let selection = Selection::new(ranges, primary);
    (s, selection)
}

/// Convert test string and selection to annotated test string.
///
/// `^` for `anchor` and `|` for head (`@` for primary).
///
/// # Examples
///
/// ```
/// use helix_core::{Range, Selection, test::plain};
/// use smallvec::smallvec;
///
/// assert_eq!(
///     plain("abc", Selection::new(smallvec![Range::new(0, 1), Range::new(3, 2)], 0)),
///     "^a@b|c^".to_owned()
/// );
/// ```
pub fn plain(s: &str, selection: Selection) -> String {
    let primary = selection.primary_index();
    let mut out = String::with_capacity(s.len() + 2 * selection.len());
    out.push_str(s);
    let mut insertion: Vec<_> = selection
        .iter()
        .enumerate()
        .flat_map(|(i, range)| {
            [
                // sort like this before reversed so anchor < head later
                (range.head, if i == primary { '@' } else { '|' }),
                (range.anchor, '^'),
            ]
        })
        .collect();
    // insert in reverse order
    insertion.sort_unstable_by_key(|k| Reverse(k.0));
    for (i, c) in insertion {
        out.insert(i, c);
    }
    out
}
