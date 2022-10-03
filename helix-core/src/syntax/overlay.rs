use std::iter::Peekable;
use std::mem::replace;
use std::ops::Range;

use crate::syntax::{Highlight, HighlightEvent};
use HighlightEvent::*;

#[cfg(test)]
mod test;

pub type MonotonicOverlay<Events, Spans> = Overlay<Events, Spans, false>;

/// Overlays multiple different highlights from `spans` onto the `HighlightEvent` stream `events`.
///
/// The [`Span`]s yielded by `spans` **must never overlap** or the iterator will produce incorrect results.
/// The [`Span`]s **must be sorted** in ascending order by their start.
/// If multiple [`Span`]s  share the same start, the ordering is arbitrary.
///
/// Together these two properties mean that `spans` must prduce monotonically increasing [`Span`]s.
/// That means that the next span must always start after the last span ended:
/// `span.end <= next_span.start`
pub fn monotonic_overlay<Events, Spans>(
    events: Events,
    spans: Spans,
) -> MonotonicOverlay<Events, Spans>
where
    Events: Iterator<Item = HighlightEvent>,
    Spans: Iterator<Item = Span>,
{
    let mut overlay = Overlay {
        events,
        spans: spans.peekable(),
        next_event: None,
        current_span: None,
        queue: EventQueue::new(),
    };
    overlay.next_event = overlay.events.next();
    overlay.current_span = overlay.spans.next();
    overlay
}

pub struct RangeToSpan<I: Iterator<Item = Range<usize>>> {
    scope: Highlight,
    ranges: I,
}

impl<I: Iterator<Item = Range<usize>>> Iterator for RangeToSpan<I> {
    type Item = Span;

    fn next(&mut self) -> Option<Self::Item> {
        self.ranges.next().map(|range| Span {
            start: range.start,
            end: range.end,
            scope: self.scope,
        })
    }
}

pub type OverlappingOverlay<Events, Ranges> = Overlay<Events, RangeToSpan<Ranges>, true>;

/// Overlays a `scope` highlight onto the `HighlightEvent`  stream `events`
/// at the ranges specified in `ranges`.
///
/// Multiple `ranges` **may overlap**, the iterator will merge these ranges into a single range
/// This is possible because all `ranges` use the **same highlighting scope**.
///
/// The `ranges` **must be sorted** in ascending order by their start.
/// If multiple `ranges`  share the same start, the ordering is arbitrary.
pub fn overlapping_overlay<Events, Ranges>(
    events: Events,
    ranges: Ranges,
    scope: Highlight,
) -> OverlappingOverlay<Events, Ranges>
where
    Events: Iterator<Item = HighlightEvent>,
    Ranges: Iterator<Item = Range<usize>>,
{
    let mut overlay = Overlay {
        events,
        spans: RangeToSpan { scope, ranges }.peekable(),
        next_event: None,
        current_span: None,
        queue: EventQueue::new(),
    };
    overlay.next_event = overlay.events.next();
    overlay.current_span = overlay.spans.next();
    overlay
}

struct EventQueue {
    data: [HighlightEvent; 2],
    len: u32,
}

impl EventQueue {
    fn new() -> EventQueue {
        EventQueue {
            data: [HighlightEnd; 2],
            len: 0,
        }
    }
    fn pop(&mut self) -> Option<HighlightEvent> {
        if self.len > 0 {
            self.len -= 1;
            let res = self.data[self.len as usize];
            Some(res)
        } else {
            None
        }
    }

    fn push(&mut self, event: HighlightEvent) {
        self.data[self.len as usize] = event;
        self.len += 1;
    }
}

#[derive(Clone, Copy)]
pub struct Span {
    pub scope: Highlight,
    pub start: usize,
    pub end: usize,
}

pub struct Overlay<Events, Spans, const MERGE: bool>
where
    Events: Iterator<Item = HighlightEvent>,
    Spans: Iterator<Item = Span>,
{
    events: Events,
    spans: Peekable<Spans>,

    next_event: Option<HighlightEvent>,
    current_span: Option<Span>,

    queue: EventQueue,
}

/// merge spans from `self.spans` that overlap `span` into one larger span
/// this function assumes that `span` and all spans yielded by `spans`
/// have the same scope
fn merge_spans<I>(span: &mut Span, spans: &mut Peekable<I>)
where
    I: Iterator<Item = Span>,
{
    while let Some(next_span) = spans.peek() {
        if next_span.start > span.end {
            break;
        }

        if next_span.end > span.end {
            span.end = next_span.end
        }
        spans.next();
    }
}

impl<Events, Spans, const MERGE: bool> Overlay<Events, Spans, MERGE>
where
    Events: Iterator<Item = HighlightEvent>,
    Spans: Iterator<Item = Span>,
{
    fn partition_source_event(
        &mut self,
        start: usize,
        end: usize,
        partition_point: usize,
    ) -> HighlightEvent {
        debug_assert!(start < partition_point && partition_point < end);
        let source_1 = Source {
            start,
            end: partition_point,
        };
        let source_2 = Source {
            start: partition_point,
            end,
        };
        self.next_event = Some(source_2);
        source_1
    }
}

impl<Events, Spans, const MERGE: bool> Iterator for Overlay<Events, Spans, MERGE>
where
    Events: Iterator<Item = HighlightEvent>,
    Spans: Iterator<Item = Span>,
{
    type Item = HighlightEvent;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(event) = self.queue.pop() {
            return Some(event);
        }

        while let Some(Source { start, end }) = self.next_event {
            if start == end {
                self.next_event = self.events.next();
                continue;
            }
            // skip empty spans and spans that end before this source
            while matches!(&self.current_span, Some(span) if span.end <= start || span.start == span.end)
            {
                self.current_span = self.spans.next();
            }

            if let Some(span) = &mut self.current_span {
                // only process the span if it actually covers this source (so starts before)
                if span.start < end {
                    // if the span starts inside the source,
                    // split off the start of the source that is not highlighted
                    // and emit this source span first
                    if start < span.start {
                        let partition_point = span.start;
                        let unhighlighted =
                            self.partition_source_event(start, end, partition_point);
                        return Some(unhighlighted);
                    }

                    // overlapping spans (with the same scope) must be merged into a single span
                    if MERGE {
                        merge_spans(span, &mut self.spans);
                    }

                    // copy out the span to satisfy the borrow checker
                    let span = *span;

                    // push `HighlightEnd` and `Source` to queue and return `HighlightStart` right now
                    self.queue.push(HighlightEnd);

                    // advance the span as the current one has been fully processed
                    if span.end <= end {
                        self.current_span = self.spans.next();
                    }
                    let event = if span.end < end {
                        // the span ends before the current source event.
                        // Add the highlighted part to the queue and process the remainder of the event later
                        let partition_point = span.end;
                        self.partition_source_event(start, end, partition_point)
                    } else {
                        // advance to the next event as the current one has been fully processed
                        self.next_event = self.events.next();
                        // the source event is fully contained within the span
                        // just emit the source event to the que and process the next event
                        Source { start, end }
                    };

                    self.queue.push(event);
                    return Some(HighlightStart(span.scope));
                }
            }

            break;
        }

        match replace(&mut self.next_event, self.events.next()) {
            Some(event) => Some(event),
            None => {
                // Unfinished span at EOF is allowed to finish.
                let span = self.current_span.take()?;
                self.queue.push(HighlightEnd);
                self.queue.push(Source {
                    start: span.start,
                    end: span.end,
                });
                Some(HighlightStart(span.scope))
            }
        }
    }
}
