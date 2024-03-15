use crate::highlighter::{Highlight, HighlightEvent};

pub struct Merge<I> {
    iter: I,
    spans: Box<dyn Iterator<Item = (usize, std::ops::Range<usize>)>>,

    next_event: Option<HighlightEvent>,
    next_span: Option<(usize, std::ops::Range<usize>)>,

    queue: Vec<HighlightEvent>,
}

/// Merge a list of spans into the highlight event stream.
pub fn merge<I: Iterator<Item = HighlightEvent>>(
    iter: I,
    spans: Vec<(usize, std::ops::Range<usize>)>,
) -> Merge<I> {
    let spans = Box::new(spans.into_iter());
    let mut merge = Merge {
        iter,
        spans,
        next_event: None,
        next_span: None,
        queue: Vec::new(),
    };
    merge.next_event = merge.iter.next();
    merge.next_span = merge.spans.next();
    merge
}

impl<I: Iterator<Item = HighlightEvent>> Iterator for Merge<I> {
    type Item = HighlightEvent;
    fn next(&mut self) -> Option<Self::Item> {
        use HighlightEvent::*;
        if let Some(event) = self.queue.pop() {
            return Some(event);
        }

        loop {
            match (self.next_event, &self.next_span) {
                // this happens when range is partially or fully offscreen
                (Some(Source { start, .. }), Some((span, range))) if start > range.start => {
                    if start > range.end {
                        self.next_span = self.spans.next();
                    } else {
                        self.next_span = Some((*span, start..range.end));
                    };
                }
                _ => break,
            }
        }

        match (self.next_event, &self.next_span) {
            (Some(HighlightStart(i)), _) => {
                self.next_event = self.iter.next();
                Some(HighlightStart(i))
            }
            (Some(HighlightEnd), _) => {
                self.next_event = self.iter.next();
                Some(HighlightEnd)
            }
            (Some(Source { start, end }), Some((_, range))) if start < range.start => {
                let intersect = range.start.min(end);
                let event = Source {
                    start,
                    end: intersect,
                };

                if end == intersect {
                    // the event is complete
                    self.next_event = self.iter.next();
                } else {
                    // subslice the event
                    self.next_event = Some(Source {
                        start: intersect,
                        end,
                    });
                };

                Some(event)
            }
            (Some(Source { start, end }), Some((span, range))) if start == range.start => {
                let intersect = range.end.min(end);
                let event = HighlightStart(Highlight(*span));

                // enqueue in reverse order
                self.queue.push(HighlightEnd);
                self.queue.push(Source {
                    start,
                    end: intersect,
                });

                if end == intersect {
                    // the event is complete
                    self.next_event = self.iter.next();
                } else {
                    // subslice the event
                    self.next_event = Some(Source {
                        start: intersect,
                        end,
                    });
                };

                if intersect == range.end {
                    self.next_span = self.spans.next();
                } else {
                    self.next_span = Some((*span, intersect..range.end));
                }

                Some(event)
            }
            (Some(event), None) => {
                self.next_event = self.iter.next();
                Some(event)
            }
            // Can happen if cursor at EOF and/or diagnostic reaches past the end.
            // We need to actually emit events for the cursor-at-EOF situation,
            // even though the range is past the end of the text.  This needs to be
            // handled appropriately by the drawing code by not assuming that
            // all `Source` events point to valid indices in the rope.
            (None, Some((span, range))) => {
                let event = HighlightStart(Highlight(*span));
                self.queue.push(HighlightEnd);
                self.queue.push(Source {
                    start: range.start,
                    end: range.end,
                });
                self.next_span = self.spans.next();
                Some(event)
            }
            (None, None) => None,
            e => unreachable!("{:?}", e),
        }
    }
}
