use crate::graphemes::{nth_next_grapheme_boundary, nth_prev_grapheme_boundary, RopeGraphemes};
use crate::{coords_at_pos, pos_at_coords, ChangeSet, Position, Range, Rope, RopeSlice, Selection};

/// A state represents the current editor state of a single buffer.
#[derive(Clone)]
pub struct State {
    // TODO: fields should be private but we need to refactor commands.rs first
    pub doc: Rope,
    pub selection: Selection,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Granularity {
    Character,
    Line,
}

impl State {
    #[must_use]
    pub fn new(doc: Rope) -> Self {
        Self {
            doc,
            selection: Selection::point(0),
        }
    }

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

    pub fn move_range(
        &self,
        range: Range,
        dir: Direction,
        granularity: Granularity,
        count: usize,
        extend: bool,
    ) -> Range {
        let text = self.doc.slice(..);
        let pos = range.head;
        let line = text.char_to_line(pos);
        // TODO: we can optimize clamping by passing in RopeSlice limited to current line. that way
        // we stop calculating past start/end of line.
        let pos = match (dir, granularity) {
            (Direction::Backward, Granularity::Character) => {
                let start = text.line_to_char(line);
                nth_prev_grapheme_boundary(text, pos, count).max(start)
            }
            (Direction::Forward, Granularity::Character) => {
                // Line end is pos at the start of next line - 1
                // subtract another 1 because the line ends with \n
                let end = text.line_to_char(line + 1).saturating_sub(2);
                nth_next_grapheme_boundary(text, pos, count).min(end)
            }
            (_, Granularity::Line) => return move_vertically(text, dir, range, count, extend),
        };
        Range::new(if extend { range.anchor } else { pos }, pos)
    }

    pub fn move_selection(
        &self,
        dir: Direction,
        granularity: Granularity,
        count: usize,
    ) -> Selection {
        self.selection
            .transform(|range| self.move_range(range, dir, granularity, count, false))
    }

    pub fn extend_selection(
        &self,
        dir: Direction,
        granularity: Granularity,
        count: usize,
    ) -> Selection {
        self.selection
            .transform(|range| self.move_range(range, dir, granularity, count, true))
    }
}

fn move_vertically(
    text: RopeSlice,
    dir: Direction,
    range: Range,
    count: usize,
    extend: bool,
) -> Range {
    let Position { row, col } = coords_at_pos(text, range.head);

    let horiz = range.horiz.unwrap_or(col as u32);

    let new_line = match dir {
        Direction::Backward => row.saturating_sub(count),
        Direction::Forward => std::cmp::min(row.saturating_add(count), text.len_lines() - 1),
    };

    // convert to 0-indexed, subtract another 1 because len_chars() counts \n
    let new_line_len = text.line(new_line).len_chars().saturating_sub(2);

    let new_col = std::cmp::min(horiz as usize, new_line_len);

    let pos = pos_at_coords(text, Position::new(new_line, new_col));

    let mut range = Range::new(if extend { range.anchor } else { pos }, pos);
    range.horiz = Some(horiz);
    range
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_vertical_move() {
        let text = Rope::from("abcd\nefg\nwrs");
        let slice = text.slice(..);
        let pos = pos_at_coords(slice, (0, 4).into());

        let range = Range::new(pos, pos);
        assert_eq!(
            coords_at_pos(
                slice,
                move_vertically(slice, Direction::Forward, range, 1, false).head
            ),
            (1, 2).into()
        );
    }
}
