use crate::graphemes::{nth_next_grapheme_boundary, nth_prev_grapheme_boundary};
use crate::{Buffer, Selection, SelectionRange};

/// A state represents the current editor state of a single buffer.
pub struct State {
    doc: Buffer,
    selection: Selection,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Granularity {
    Character,
    Word,
    Line,
    // LineBoundary
}

impl State {
    #[must_use]
    pub fn new(doc: Buffer) -> Self {
        Self {
            doc,
            selection: Selection::single(0, 0),
        }
    }

    // TODO: buf/selection accessors

    // update/transact:
    // update(desc) => transaction ?  transaction.doc() for applied doc
    // transaction.apply(doc)
    // doc.transact(fn -> ... end)

    // replaceSelection (transaction that replaces selection)
    // changeByRange
    // changes
    // slice
    //
    // getters:
    // tabSize
    // indentUnit
    // languageDataAt()
    //
    // config:
    // indentation
    // tabSize
    // lineUnit
    // syntax
    // foldable
    // changeFilter/transactionFilter

    pub fn move_pos(
        &self,
        pos: usize,
        dir: Direction,
        granularity: Granularity,
        n: usize,
    ) -> usize {
        let text = &self.doc.contents;
        match (dir, granularity) {
            (Direction::Backward, Granularity::Character) => {
                nth_prev_grapheme_boundary(&text.slice(..), pos, n)
            }
            (Direction::Forward, Granularity::Character) => {
                nth_next_grapheme_boundary(&text.slice(..), pos, n)
            }
            _ => pos,
        }
    }

    pub fn move_selection(
        &self,
        sel: Selection,
        dir: Direction,
        granularity: Granularity,
        // TODO: n
    ) -> Selection {
        // TODO: move all selections according to normal cursor move semantics by collapsing it
        // into cursors and moving them vertically

        let ranges = sel.ranges.into_iter().map(|range| {
            // let pos = if !range.is_empty() {
            //     // if selection already exists, bump it to the start or end of current select first
            //     if dir == Direction::Backward {
            //         range.from()
            //     } else {
            //         range.to()
            //     }
            // } else {
            let pos = self.move_pos(range.head, dir, granularity, 1);
            // };
            SelectionRange::new(pos, pos)
        });

        Selection::new(ranges.collect(), sel.primary_index)
        // TODO: update selection in state via transaction
    }

    pub fn extend_selection(
        &self,
        sel: Selection,
        dir: Direction,
        granularity: Granularity,
        n: usize,
    ) -> Selection {
        let ranges = sel.ranges.into_iter().map(|range| {
            let pos = self.move_pos(range.head, dir, granularity, n);
            SelectionRange::new(range.anchor, pos)
        });

        Selection::new(ranges.collect(), sel.primary_index)
        // TODO: update selection in state via transaction
    }
}
