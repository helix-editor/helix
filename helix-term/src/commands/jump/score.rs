use super::locations::cursor_at;
use crate::commands::Context;
use helix_core::{Position, Range};

fn manhattan_distance(p1: &Position, p2: &Position) -> usize {
    // Make it easier to travel along the x-axis
    const Y_WEIGHT: usize = 10;
    Y_WEIGHT
        .saturating_mul(p1.row.abs_diff(p2.row))
        .saturating_add(p1.col.abs_diff(p2.col))
}

struct ScoredTarget {
    range: Range,
    distance: usize,
}

pub fn sort_jump_targets(cx: &mut Context, jump_targets: Vec<Range>) -> Vec<Range> {
    // Each jump target will be scored based on its distance to the cursor position.
    // The position of each jump target in the view is only approximated due to performance issues.
    let cursor = cursor_at(cx);
    let text = doc!(cx.editor).text().slice(..);
    let mut jump_targets: Vec<_> = jump_targets
        .iter()
        .zip((0..jump_targets.len()).map(|i| {
            let cur = jump_targets[i].head;
            let row = text.char_to_line(cur);
            let col = cur - text.line_to_char(row);
            Position { row, col }
        }))
        .map(|(range, pos)| ScoredTarget {
            range: *range,
            distance: manhattan_distance(&cursor, &pos),
        })
        .collect();
    // Sort by the distance (shortest first)
    jump_targets.sort_by(|a, b| a.distance.cmp(&b.distance));
    jump_targets.iter().map(|a| a.range).collect()
}
