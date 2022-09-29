use crate::syntax::{overlapping_overlay, Highlight, HighlightEvent::*};
use std::iter;

/// this tests checks that merging two overlapping ranges onto each other
/// correctly preveres the order of merges.
/// that is the highlight that is merged in last, gets applied last and overwrites the other layers
/// In this test a range of lower priority (like a hint) starts at 2
/// and another range of a high priority range (like an error) starts earlier
/// with the old span implementation the hint would always overwrite the error.
/// The new implementation (tested here) forces the
#[test]
fn overlay_long_hint() {
    let base = iter::once(Source { start: 0, end: 31 });
    let highlights = overlapping_overlay(base, [2..10].into_iter(), Highlight(1));
    let highlights = overlapping_overlay(highlights, [0..4].into_iter(), Highlight(2));
    let res: Vec<_> = highlights.collect();
    assert_eq!(
        &*res,
        &[
            HighlightStart(Highlight(2)),
            Source { start: 0, end: 2 },
            HighlightEnd,
            HighlightStart(Highlight(1)),
            HighlightStart(Highlight(2)),
            Source { start: 2, end: 4 },
            HighlightEnd,
            Source { start: 4, end: 10 },
            HighlightEnd,
            Source { start: 10, end: 31 },
        ]
    );
}
