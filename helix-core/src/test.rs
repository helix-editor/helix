//! Test helpers.
use crate::{Range, Selection};
use ropey::Rope;
use smallvec::SmallVec;
use std::cmp::Reverse;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub enum ParseSelectionError {
    MoreThanOnePrimary(String),
    MissingClosingPair(String),
    MissingPrimary(String),
}

/// Convert string annotated with selections to string and selection.
///
/// `#[|` for primary selection with head before anchor followed by `]#`.
/// `#(|` for secondary selection with head before anchor followed by `)#`.
/// `#[` for primary selection with head after anchor followed by `|]#`.
/// `#(` for secondary selection with head after anchor followed by `|)#`.
///
/// If the selection contains any LF or CRLF sequences, which are immediately
/// followed by the same grapheme, then the subsequent one is removed. This is
/// to allow representing having the cursor over the end of the line.
///
/// # Examples
///
/// ```
/// use helix_core::{Range, Selection, test::parse_selection_string};
/// use smallvec::smallvec;
///
/// assert_eq!(
///     parse_selection_string("#[a|]#b#(|c)#").unwrap(),
///     ("abc".to_owned(), Selection::new(smallvec![Range::new(0, 1), Range::new(3, 2)], 0))
/// );
/// ```
pub fn parse_selection_string(s: &str) -> Result<(String, Selection), ParseSelectionError> {
    let mut primary_idx = None;
    let mut ranges = SmallVec::new();
    let mut iter = UnicodeSegmentation::graphemes(s, true).peekable();
    let mut left = String::with_capacity(s.len());

    'outer: while let Some(c) = iter.next() {
        let start = left.chars().count();

        if c != "#" {
            left.push_str(c);
            continue;
        }

        let (is_primary, close_pair) = match iter.next() {
            Some("[") => (true, "]"),
            Some("(") => (false, ")"),
            Some(ch) => {
                left.push('#');
                left.push_str(ch);
                continue;
            }
            None => break,
        };

        if is_primary && primary_idx.is_some() {
            return Err(ParseSelectionError::MoreThanOnePrimary(format!(
                "{:?} {:?}",
                left, s
            )));
        }

        let head_at_beg = iter.next_if_eq(&"|").is_some();
        let last_grapheme = |s: &str| {
            UnicodeSegmentation::graphemes(s, true)
                .last()
                .map(String::from)
        };

        while let Some(c) = iter.next() {
            let next = iter.peek();
            let mut prev = last_grapheme(left.as_str());

            if !(c == close_pair && next == Some(&"#")) {
                left.push_str(c);
                continue;
            }

            if !head_at_beg {
                match &prev {
                    Some(p) if p != "|" => {
                        left.push_str(c);
                        continue;
                    }
                    Some(p) if p == "|" => {
                        left.pop().unwrap(); // pop the |
                        prev = last_grapheme(left.as_str());
                    }
                    _ => (),
                }
            }

            iter.next(); // skip "#"
            let next = iter.peek();

            // skip explicit line end inside selection
            if (prev == Some(String::from("\r\n")) || prev == Some(String::from("\n")))
                && next.map(|n| String::from(*n)) == prev
            {
                iter.next();
            }

            if is_primary {
                primary_idx = Some(ranges.len());
            }

            let (anchor, head) = match head_at_beg {
                true => (left.chars().count(), start),
                false => (start, left.chars().count()),
            };

            ranges.push(Range::new(anchor, head));
            continue 'outer;
        }

        if head_at_beg {
            return Err(ParseSelectionError::MissingClosingPair(format!(
                "Missing end `{}#` {:?} {:?}",
                close_pair, left, s
            )));
        } else {
            return Err(ParseSelectionError::MissingClosingPair(format!(
                "Missing end `|{}#` {:?} {:?}",
                close_pair, left, s
            )));
        }
    }

    let primary = match primary_idx {
        Some(i) => i,
        None => {
            return Err(ParseSelectionError::MissingPrimary(format!(
                "Missing primary `#[|]#` {:?}",
                s
            )));
        }
    };

    let selection = Selection::new(ranges, primary);
    Ok((left, selection))
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
///     plain("abc", &Selection::new(smallvec![Range::new(0, 1), Range::new(3, 2)], 0)),
///     "#[a|]#b#(|c)#".to_owned()
/// );
/// ```
pub fn plain<R: Into<Rope>>(s: R, selection: &Selection) -> String {
    let s = s.into();
    let primary = selection.primary_index();
    let mut out = String::with_capacity(s.len_bytes() + 5 * selection.len());
    out.push_str(&s.to_string());

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
        .map(|(char_idx, marker)| (s.char_to_byte(char_idx), marker))
        .collect();

    // insert in reverse order
    insertion.sort_unstable_by_key(|k| Reverse(k.0));
    for (i, s) in insertion {
        out.insert_str(i, s);
    }
    out
}

#[cfg(test)]
#[allow(clippy::module_inception)]
mod test {
    use super::*;

    #[test]
    fn print_single() {
        assert_eq!(
            (String::from("hello"), Selection::single(1, 0)),
            parse_selection_string("#[|h]#ello").unwrap()
        );
        assert_eq!(
            (String::from("hello"), Selection::single(0, 1)),
            parse_selection_string("#[h|]#ello").unwrap()
        );
        assert_eq!(
            (String::from("hello"), Selection::single(4, 0)),
            parse_selection_string("#[|hell]#o").unwrap()
        );
        assert_eq!(
            (String::from("hello"), Selection::single(0, 4)),
            parse_selection_string("#[hell|]#o").unwrap()
        );
        assert_eq!(
            (String::from("hello"), Selection::single(5, 0)),
            parse_selection_string("#[|hello]#").unwrap()
        );
        assert_eq!(
            (String::from("hello"), Selection::single(0, 5)),
            parse_selection_string("#[hello|]#").unwrap()
        );
    }

    #[test]
    fn print_multi() {
        assert_eq!(
            (
                String::from("hello"),
                Selection::new(
                    SmallVec::from_slice(&[Range::new(1, 0), Range::new(5, 4)]),
                    0
                )
            ),
            parse_selection_string("#[|h]#ell#(|o)#").unwrap()
        );
        assert_eq!(
            (
                String::from("hello"),
                Selection::new(
                    SmallVec::from_slice(&[Range::new(0, 1), Range::new(4, 5)]),
                    0
                )
            ),
            parse_selection_string("#[h|]#ell#(o|)#").unwrap()
        );
        assert_eq!(
            (
                String::from("hello"),
                Selection::new(
                    SmallVec::from_slice(&[Range::new(2, 0), Range::new(5, 3)]),
                    0
                )
            ),
            parse_selection_string("#[|he]#l#(|lo)#").unwrap()
        );
        assert_eq!(
            (
                String::from("hello\r\nhello\r\nhello\r\n"),
                Selection::new(
                    SmallVec::from_slice(&[
                        Range::new(7, 5),
                        Range::new(21, 19),
                        Range::new(14, 12)
                    ]),
                    0
                )
            ),
            parse_selection_string("hello#[|\r\n]#hello#(|\r\n)#hello#(|\r\n)#").unwrap()
        );
    }

    #[test]
    fn print_multi_byte_code_point() {
        assert_eq!(
            (String::from("„“"), Selection::single(1, 0)),
            parse_selection_string("#[|„]#“").unwrap()
        );
        assert_eq!(
            (String::from("„“"), Selection::single(2, 1)),
            parse_selection_string("„#[|“]#").unwrap()
        );
        assert_eq!(
            (String::from("„“"), Selection::single(0, 1)),
            parse_selection_string("#[„|]#“").unwrap()
        );
        assert_eq!(
            (String::from("„“"), Selection::single(1, 2)),
            parse_selection_string("„#[“|]#").unwrap()
        );
        assert_eq!(
            (String::from("they said „hello“"), Selection::single(11, 10)),
            parse_selection_string("they said #[|„]#hello“").unwrap()
        );
    }

    #[test]
    fn print_multi_code_point_grapheme() {
        assert_eq!(
            (
                String::from("hello 👨‍👩‍👧‍👦 goodbye"),
                Selection::single(13, 6)
            ),
            parse_selection_string("hello #[|👨‍👩‍👧‍👦]# goodbye").unwrap()
        );
    }

    #[test]
    fn plain_single() {
        assert_eq!("#[|h]#ello", plain("hello", &Selection::single(1, 0)));
        assert_eq!("#[h|]#ello", plain("hello", &Selection::single(0, 1)));
        assert_eq!("#[|hell]#o", plain("hello", &Selection::single(4, 0)));
        assert_eq!("#[hell|]#o", plain("hello", &Selection::single(0, 4)));
        assert_eq!("#[|hello]#", plain("hello", &Selection::single(5, 0)));
        assert_eq!("#[hello|]#", plain("hello", &Selection::single(0, 5)));
    }

    #[test]
    fn plain_multi() {
        assert_eq!(
            plain(
                "hello",
                &Selection::new(
                    SmallVec::from_slice(&[Range::new(1, 0), Range::new(5, 4)]),
                    0
                )
            ),
            String::from("#[|h]#ell#(|o)#")
        );
        assert_eq!(
            plain(
                "hello",
                &Selection::new(
                    SmallVec::from_slice(&[Range::new(0, 1), Range::new(4, 5)]),
                    0
                )
            ),
            String::from("#[h|]#ell#(o|)#")
        );
        assert_eq!(
            plain(
                "hello",
                &Selection::new(
                    SmallVec::from_slice(&[Range::new(2, 0), Range::new(5, 3)]),
                    0
                )
            ),
            String::from("#[|he]#l#(|lo)#")
        );
        assert_eq!(
            plain(
                "hello\r\nhello\r\nhello\r\n",
                &Selection::new(
                    SmallVec::from_slice(&[
                        Range::new(7, 5),
                        Range::new(21, 19),
                        Range::new(14, 12)
                    ]),
                    0
                )
            ),
            String::from("hello#[|\r\n]#hello#(|\r\n)#hello#(|\r\n)#")
        );
    }

    #[test]
    fn plain_multi_byte_code_point() {
        assert_eq!(
            plain("„“", &Selection::single(1, 0)),
            String::from("#[|„]#“")
        );
        assert_eq!(
            plain("„“", &Selection::single(2, 1)),
            String::from("„#[|“]#")
        );
        assert_eq!(
            plain("„“", &Selection::single(0, 1)),
            String::from("#[„|]#“")
        );
        assert_eq!(
            plain("„“", &Selection::single(1, 2)),
            String::from("„#[“|]#")
        );
        assert_eq!(
            plain("they said „hello“", &Selection::single(11, 10)),
            String::from("they said #[|„]#hello“")
        );
    }

    #[test]
    fn plain_multi_code_point_grapheme() {
        assert_eq!(
            plain("hello 👨‍👩‍👧‍👦 goodbye", &Selection::single(13, 6)),
            String::from("hello #[|👨‍👩‍👧‍👦]# goodbye")
        );
    }
}
