//! Test helpers.
use crate::{Range, Selection};
use smallvec::SmallVec;
use std::cmp::Reverse;

/// Convert annotated test string to test string and selection.
///
/// `#[|` for primary selection with head before anchor followed by `]#`.
/// `#(|` for secondary selection with head before anchor followed by `)#`.
/// `#[` for primary selection with head after anchor followed by `|]#`.
/// `#(` for secondary selection with head after anchor followed by `|)#`.
///
/// # Examples
///
/// ```
/// use helix_core::{Range, Selection, test::print};
/// use smallvec::smallvec;
///
/// assert_eq!(
///     print("#[a|]#b#(|c)#"),
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
    let mut primary = None;
    let mut ranges = SmallVec::new();
    let mut iter = s.chars().peekable();
    let mut left = String::with_capacity(s.len());
    'outer: while let Some(c) = iter.next() {
        let start = left.len();
        if c == '#' {
            if iter.next_if_eq(&'[').is_some() {
                if primary.is_some() {
                    panic!("primary `#[` already appeared {left:?} {s:?}");
                }
                if iter.next_if_eq(&'|').is_some() {
                    while let Some(c) = iter.next() {
                        if c == ']' && iter.next_if_eq(&'#').is_some() {
                            primary = Some(ranges.len());
                            ranges.push(Range::new(left.len(), start));
                            continue 'outer;
                        } else {
                            left.push(c);
                        }
                    }
                    panic!("missing primary end `]#` {left:?} {s:?}");
                } else {
                    while let Some(c) = iter.next() {
                        if c == '|' {
                            if let Some(cc) = iter.next_if_eq(&']') {
                                if iter.next_if_eq(&'#').is_some() {
                                    primary = Some(ranges.len());
                                    ranges.push(Range::new(start, left.len()));
                                    continue 'outer;
                                } else {
                                    left.push(c);
                                    left.push(cc);
                                }
                            } else {
                                left.push(c);
                            }
                        } else {
                            left.push(c);
                        }
                    }
                    panic!("missing primary end `|]#` {left:?} {s:?}");
                }
            } else if iter.next_if_eq(&'(').is_some() {
                if iter.next_if_eq(&'|').is_some() {
                    while let Some(c) = iter.next() {
                        if c == ')' && iter.next_if_eq(&'#').is_some() {
                            ranges.push(Range::new(left.len(), start));
                            continue 'outer;
                        } else {
                            left.push(c);
                        }
                    }
                    panic!("missing end `)#` {left:?} {s:?}");
                } else {
                    while let Some(c) = iter.next() {
                        if c == '|' {
                            if let Some(cc) = iter.next_if_eq(&')') {
                                if iter.next_if_eq(&'#').is_some() {
                                    ranges.push(Range::new(start, left.len()));
                                    continue 'outer;
                                } else {
                                    left.push(c);
                                    left.push(cc);
                                }
                            } else {
                                left.push(c);
                            }
                        } else {
                            left.push(c);
                        }
                    }
                    panic!("missing end `|)#` {left:?} {s:?}");
                }
            } else {
                left.push(c);
            }
        } else {
            left.push(c);
        }
    }
    let primary = match primary {
        Some(i) => i,
        None => panic!("missing primary `#[|]#` {s:?}"),
    };
    let selection = Selection::new(ranges, primary);
    (left, selection)
}

/// Convert test string and selection to annotated test string.
///
/// `#[|` for primary selection with head before anchor followed by `]#`.
/// `#(|` for secondary selection with head before anchor followed by `)#`.
/// `#[` for primary selection with head after anchor followed by `|]#`.
/// `#(` for secondary selection with head after anchor followed by `|)#`.
///
/// # Examples
///
/// ```
/// use helix_core::{Range, Selection, test::plain};
/// use smallvec::smallvec;
///
/// assert_eq!(
///     plain("abc", Selection::new(smallvec![Range::new(0, 1), Range::new(3, 2)], 0)),
///     "#[a|]#b#(|c)#".to_owned()
/// );
/// ```
pub fn plain(s: &str, selection: Selection) -> String {
    let primary = selection.primary_index();
    let mut out = String::with_capacity(s.len() + 5 * selection.len());
    out.push_str(s);
    let mut insertion: Vec<_> = selection
        .iter()
        .enumerate()
        .flat_map(|(i, range)| {
            // sort like this before reversed so anchor < head later
            match (range.anchor < range.head, i == primary) {
                (true, true) => [(range.anchor, "#["), (range.head, "|]#")],
                (true, false) => [(range.anchor, "#("), (range.head, "|)#")],
                (false, true) => [(range.anchor, "]#"), (range.head, "#[|")],
                (false, false) => [(range.anchor, ")#"), (range.head, "#(|")],
            }
        })
        .collect();
    // insert in reverse order
    insertion.sort_unstable_by_key(|k| Reverse(k.0));
    for (i, s) in insertion {
        out.insert_str(i, s);
    }
    out
}
