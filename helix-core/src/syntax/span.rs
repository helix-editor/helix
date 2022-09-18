use std::collections::VecDeque;

use crate::syntax::Highlight;

use super::HighlightEvent;

/// A range highlighted with a given scope.
///
/// Spans are a simplifer data structure for describing a highlight range
/// than [super::HighlightEvent]s.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Span {
    pub scope: usize,
    pub start: usize,
    pub end: usize,
}

impl Ord for Span {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by range: ascending by start and then ascending by end for ties.
        if self.start == other.start {
            self.end.cmp(&other.end)
        } else {
            self.start.cmp(&other.start)
        }
    }
}

impl PartialOrd for Span {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

struct SpanIter {
    spans: Vec<Span>,
    index: usize,
    event_queue: VecDeque<HighlightEvent>,
    range_ends: Vec<usize>,
    cursor: usize,
}

/// Creates an iterator of [HighlightEvent]s from a [Vec] of [Span]s.
///
/// Spans may overlap. In the produced [HighlightEvent] iterator, all
/// `HighlightEvent::Source` events will be sorted by `start` and will not
/// overlap. The iterator produced by this function satisfies all invariants
/// and assumptions for [super::merge]
///
/// `spans` is assumed to be sorted by `range.start` ascending and then by
/// `range.end` descending for any ties.
///
/// # Panics
///
/// Panics on debug builds when the input spans overlap or are not sorted.
pub fn span_iter(spans: Vec<Span>) -> impl Iterator<Item = HighlightEvent> {
    // Assert that `spans` is sorted by `range.start` ascending and
    // `range.end` descending.
    debug_assert!(spans.windows(2).all(|window| window[0] <= window[1]));

    SpanIter {
        spans,
        index: 0,
        event_queue: VecDeque::new(),
        range_ends: Vec::new(),
        cursor: 0,
    }
}

impl Iterator for SpanIter {
    type Item = HighlightEvent;

    fn next(&mut self) -> Option<Self::Item> {
        use HighlightEvent::*;

        // Emit any queued highlight events
        if let Some(event) = self.event_queue.pop_front() {
            return Some(event);
        }

        if self.index == self.spans.len() {
            // There are no more spans. Emit Sources and HighlightEnds for
            // any ranges which have not been terminated yet.
            for end in self.range_ends.drain(..) {
                if self.cursor != end {
                    debug_assert!(self.cursor < end);
                    self.event_queue.push_back(Source {
                        start: self.cursor,
                        end,
                    });
                }
                self.event_queue.push_back(HighlightEnd);
                self.cursor = end;
            }
            return self.event_queue.pop_front();
        }

        let span = self.spans[self.index];
        let mut subslice = None;

        self.range_ends.retain(|end| {
            if span.start >= *end {
                // The new range is past the end of this in-progress range.
                // Complete the in-progress range by emitting a Source,
                // if necessary, and a HighlightEnd and advance the cursor.
                if self.cursor != *end {
                    debug_assert!(self.cursor < *end);
                    self.event_queue.push_back(Source {
                        start: self.cursor,
                        end: *end,
                    });
                }
                self.event_queue.push_back(HighlightEnd);
                self.cursor = *end;
                false
            } else if span.end > *end && subslice.is_none() {
                // If the new range is longer than some in-progress range,
                // we need to subslice this range and any ranges with the
                // same start. `subslice` is set to the smallest `end` for
                // which `range.start < end < range.end`.
                subslice = Some(*end);
                true
            } else {
                true
            }
        });

        // Emit a Source event between consecutive HighlightStart events
        if span.start != self.cursor && !self.range_ends.is_empty() {
            debug_assert!(self.cursor < span.start);
            self.event_queue.push_back(Source {
                start: self.cursor,
                end: span.start,
            });
        }

        self.cursor = span.start;

        // Handle all spans that share this starting point. Either subslice
        // or fully consume the span.
        let mut i = self.index;
        let mut subslices = 0;
        loop {
            match self.spans.get_mut(i) {
                Some(span) if span.start == self.cursor => {
                    self.event_queue
                        .push_back(HighlightStart(Highlight(span.scope)));
                    i += 1;

                    match subslice {
                        Some(intersect) => {
                            // If this span needs to be subsliced, consume the
                            // left part of the subslice and leave the right.
                            self.range_ends.push(intersect);
                            span.start = intersect;
                            subslices += 1;
                        }
                        None => {
                            // If there is no subslice, consume the span.
                            self.range_ends.push(span.end);
                            self.index = i;
                        }
                    }
                }
                _ => break,
            }
        }

        // Ensure range-ends are sorted ascending. Ranges which start at the
        // same point may be in descending order because of the assumed
        // sort-order of input ranges.
        self.range_ends.sort_unstable();

        // When spans are subsliced, the span Vec may need to be re-sorted
        // because the `range.start` may now be greater than some `range.start`
        // later in the Vec. This is not a classic "sort": we take several
        // shortcuts to improve the runtime so that the sort may be done in
        // time linear to the cardinality of the span Vec. Practically speaking
        // the runtime is even better since we only scan from `self.index` to
        // the first element of the Vec with a `range.start` after this range.
        if let Some(intersect) = subslice {
            let mut after = None;

            // Find the index of the largest span smaller than the intersect point.
            // `i` starts on the index after the last subsliced span.
            loop {
                match self.spans.get(i) {
                    Some(span) if span.start < intersect => {
                        after = Some(i);
                        i += 1;
                    }
                    _ => break,
                }
            }

            // Rotate the subsliced spans so that they come after the spans that
            // have smaller `range.start`s.
            if let Some(after) = after {
                self.spans[self.index..=after].rotate_left(subslices);
            }
        }

        self.event_queue.pop_front()
    }
}

struct FlatSpanIter<I> {
    iter: I,
}

/// Converts a Vec of spans into an [Iterator] over [HighlightEvent]s
///
/// This implementation does not resolve overlapping spans. Zero-width spans are
/// eliminated but otherwise the ranges are trusted to not overlap.
///
/// This iterator has much less overhead than [span_iter] and is appropriate for
/// cases where the input spans are known to satisfy all of [super::merge]'s
/// assumptions and invariants, such as with selection highlights.
///
/// # Panics
///
/// Panics on debug builds when the input spans overlap or are not sorted.
pub fn flat_span_iter(spans: Vec<Span>) -> impl Iterator<Item = HighlightEvent> {
    use HighlightEvent::*;

    // Consecutive items are sorted and non-overlapping
    debug_assert!(spans
        .windows(2)
        .all(|window| window[1].start >= window[0].end));

    FlatSpanIter {
        iter: spans
            .into_iter()
            .filter(|span| span.start != span.end)
            .flat_map(|span| {
                [
                    HighlightStart(Highlight(span.scope)),
                    Source {
                        start: span.start,
                        end: span.end,
                    },
                    HighlightEnd,
                ]
            }),
    }
}

impl<I: Iterator<Item = HighlightEvent>> Iterator for FlatSpanIter<I> {
    type Item = HighlightEvent;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! span {
        ($scope:literal, $range:expr) => {
            Span {
                scope: $scope,
                start: $range.start,
                end: $range.end,
            }
        };
    }

    #[test]
    fn test_non_overlapping_span_iter_events() {
        use HighlightEvent::*;
        let input = vec![span!(1, 0..5), span!(2, 6..10)];
        let output: Vec<_> = span_iter(input).collect();
        assert_eq!(
            output,
            &[
                HighlightStart(Highlight(1)),
                Source { start: 0, end: 5 },
                HighlightEnd, // ends 1
                HighlightStart(Highlight(2)),
                Source { start: 6, end: 10 },
                HighlightEnd, // ends 2
            ],
        );
    }

    #[test]
    fn test_simple_overlapping_span_iter_events() {
        use HighlightEvent::*;

        let input = vec![span!(1, 0..10), span!(2, 3..6)];
        let output: Vec<_> = span_iter(input).collect();
        assert_eq!(
            output,
            &[
                HighlightStart(Highlight(1)),
                Source { start: 0, end: 3 },
                HighlightStart(Highlight(2)),
                Source { start: 3, end: 6 },
                HighlightEnd, // ends 2
                Source { start: 6, end: 10 },
                HighlightEnd, // ends 1
            ],
        );
    }

    #[test]
    fn test_many_overlapping_span_iter_events() {
        use HighlightEvent::*;

        /*
        Input:

                                                                    5
                                                                |-------|
                                                                   4
                                                             |----------|
                                                  3
                                    |---------------------------|
                        2
                |---------------|
                                1
            |---------------------------------------|

            |---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
            0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15
        */
        let input = vec![
            span!(1, 0..10),
            span!(2, 1..5),
            span!(3, 6..13),
            span!(4, 12..15),
            span!(5, 13..15),
        ];

        /*
        Output:

                        2                  3                  4     5
                |---------------|   |---------------|       |---|-------|

                                1                         3         4
            |---------------------------------------|-----------|-------|

            |---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
            0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15
        */
        let output: Vec<_> = span_iter(input).collect();

        assert_eq!(
            output,
            &[
                HighlightStart(Highlight(1)),
                Source { start: 0, end: 1 },
                HighlightStart(Highlight(2)),
                Source { start: 1, end: 5 },
                HighlightEnd, // ends 2
                Source { start: 5, end: 6 },
                HighlightStart(Highlight(3)),
                Source { start: 6, end: 10 },
                HighlightEnd, // ends 3
                HighlightEnd, // ends 1
                HighlightStart(Highlight(3)),
                Source { start: 10, end: 12 },
                HighlightStart(Highlight(4)),
                Source { start: 12, end: 13 },
                HighlightEnd, // ends 4
                HighlightEnd, // ends 3
                HighlightStart(Highlight(4)),
                HighlightStart(Highlight(5)),
                Source { start: 13, end: 15 },
                HighlightEnd, // ends 5
                HighlightEnd, // ends 4
            ],
        );
    }

    #[test]
    fn test_multiple_duplicate_overlapping_span_iter_events() {
        use HighlightEvent::*;
        // This is based an a realistic case from rust-analyzer
        // diagnostics. Spans may both overlap and duplicate one
        // another at varying diagnostic levels.

        /*
        Input:

                                      4,5
                            |-----------------------|
                                    3
                            |---------------|
                        1,2
            |-----------------------|

            |---|---|---|---|---|---|---|---|---|---|
            0   1   2   3   4   5   6   7   8   9  10
        */

        let input = vec![
            span!(1, 0..6),
            span!(2, 0..6),
            span!(3, 4..8),
            span!(4, 4..10),
            span!(5, 4..10),
        ];

        /*
        Output:

                   1,2         1..5    3..5    4,5
            |---------------|-------|-------|-------|

            |---|---|---|---|---|---|---|---|---|---|
            0   1   2   3   4   5   6   7   8   9  10
        */
        let output: Vec<_> = span_iter(input).collect();
        assert_eq!(
            output,
            &[
                HighlightStart(Highlight(1)),
                HighlightStart(Highlight(2)),
                Source { start: 0, end: 4 },
                HighlightStart(Highlight(3)),
                HighlightStart(Highlight(4)),
                HighlightStart(Highlight(5)),
                Source { start: 4, end: 6 },
                HighlightEnd, // ends 5
                HighlightEnd, // ends 4
                HighlightEnd, // ends 3
                HighlightEnd, // ends 2
                HighlightEnd, // ends 1
                HighlightStart(Highlight(3)),
                HighlightStart(Highlight(4)),
                HighlightStart(Highlight(5)),
                Source { start: 6, end: 8 },
                HighlightEnd, // ends 5
                Source { start: 8, end: 10 },
                HighlightEnd, // ends 4
                HighlightEnd, // ends 3
            ],
        );
    }

    #[test]
    fn test_span_iter_events_where_ranges_must_be_sorted() {
        use HighlightEvent::*;
        // This case needs the span Vec to be re-sorted because
        // span 3 is subsliced to 9..10, putting it after span 4 and 5
        // in the ordering.

        /*
        Input:

                                          4   5
                                        |---|---|
                        2                   3
                |---------------|   |---------------|
                              1
            |-----------------------------------|

            |---|---|---|---|---|---|---|---|---|---|
            0   1   2   3   4   5   6   7   8   9  10
        */
        let input = vec![
            span!(1, 0..9),
            span!(2, 1..5),
            span!(3, 6..10),
            span!(4, 7..8),
            span!(5, 8..9),
        ];

        /*
        Output:

                                          4   5
                                        |---|---|
                        2                   3
                |---------------|   |-----------|
                              1                   3
            |-----------------------------------|---|

            |---|---|---|---|---|---|---|---|---|---|
            0   1   2   3   4   5   6   7   8   9  10
        */
        let output: Vec<_> = span_iter(input).collect();
        assert_eq!(
            output,
            &[
                HighlightStart(Highlight(1)),
                Source { start: 0, end: 1 },
                HighlightStart(Highlight(2)),
                Source { start: 1, end: 5 },
                HighlightEnd, // ends 2
                Source { start: 5, end: 6 },
                HighlightStart(Highlight(3)),
                Source { start: 6, end: 7 },
                HighlightStart(Highlight(4)),
                Source { start: 7, end: 8 },
                HighlightEnd, // ends 4
                HighlightStart(Highlight(5)),
                Source { start: 8, end: 9 },
                HighlightEnd, // ends 5
                HighlightEnd, // ends 3
                HighlightEnd, // ends 1
                HighlightStart(Highlight(3)),
                Source { start: 9, end: 10 },
                HighlightEnd, // ends 3
            ],
        );
    }
}
