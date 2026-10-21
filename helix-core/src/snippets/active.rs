use std::ops::{Index, IndexMut};

use foldhash::HashSet;
use helix_stdx::range::{is_exact_subset, is_subset};
use helix_stdx::Range;
use ropey::Rope;

use crate::movement::Direction;
use crate::snippets::render::{RenderedSnippet, Tabstop};
use crate::snippets::TabstopIdx;
use crate::{Assoc, ChangeSet, Selection, Transaction};

pub struct ActiveSnippet {
    ranges: Vec<Range>,
    active_tabstops: HashSet<TabstopIdx>,
    current_tabstop: TabstopIdx,
    tabstops: Vec<Tabstop>,
}

impl Index<TabstopIdx> for ActiveSnippet {
    type Output = Tabstop;
    fn index(&self, index: TabstopIdx) -> &Tabstop {
        &self.tabstops[index.0]
    }
}

impl IndexMut<TabstopIdx> for ActiveSnippet {
    fn index_mut(&mut self, index: TabstopIdx) -> &mut Tabstop {
        &mut self.tabstops[index.0]
    }
}

impl ActiveSnippet {
    pub fn new(snippet: RenderedSnippet) -> Option<Self> {
        let snippet = Self {
            ranges: snippet.ranges,
            tabstops: snippet.tabstops,
            active_tabstops: HashSet::default(),
            current_tabstop: TabstopIdx(0),
        };
        (snippet.tabstops.len() != 1).then_some(snippet)
    }

    pub fn is_valid(&self, new_selection: &Selection) -> bool {
        is_subset::<false>(self.ranges.iter().copied(), new_selection.range_bounds())
    }

    pub fn tabstops(&self) -> impl Iterator<Item = &Tabstop> {
        self.tabstops.iter()
    }

    pub fn delete_placeholder(&self, doc: &Rope) -> Transaction {
        Transaction::delete(
            doc,
            self[self.current_tabstop]
                .ranges
                .iter()
                .map(|range| (range.start, range.end)),
        )
    }

    /// maps the active snippets through a `ChangeSet` updating all tabstop ranges
    pub fn map(&mut self, changes: &ChangeSet) -> bool {
        let positions_to_map = self.ranges.iter_mut().flat_map(|range| {
            [
                (&mut range.start, Assoc::After),
                (&mut range.end, Assoc::After),
            ]
        });
        changes.update_positions(positions_to_map);

        for (i, tabstop) in self.tabstops.iter_mut().enumerate() {
            if self.active_tabstops.contains(&TabstopIdx(i)) {
                let positions_to_map = tabstop.ranges.iter_mut().flat_map(|range| {
                    [
                        (&mut range.start, Assoc::After),
                        (&mut range.end, Assoc::After),
                    ]
                });
                changes.update_positions(positions_to_map);
            } else {
                let positions_to_map = tabstop.ranges.iter_mut().flat_map(|range| {
                    let end_assoc = if range.start == range.end {
                        Assoc::After
                    } else {
                        Assoc::Before
                    };
                    [
                        (&mut range.start, Assoc::After),
                        (&mut range.end, end_assoc),
                    ]
                });
                changes.update_positions(positions_to_map);
            }
            let mut snippet_ranges = self.ranges.iter();
            let mut snippet_range = snippet_ranges.next().unwrap();
            let mut tabstop_i = 0;
            let mut prev = Range { start: 0, end: 0 };
            let num_ranges = tabstop.ranges.len() / self.ranges.len();
            tabstop.ranges.retain_mut(|range| {
                if tabstop_i == num_ranges {
                    snippet_range = snippet_ranges.next().unwrap();
                    tabstop_i = 0;
                }
                tabstop_i += 1;
                let retain = snippet_range.start <= snippet_range.end;
                if retain {
                    // guaranteed by assoc
                    debug_assert!(prev.start <= range.start);
                    debug_assert!(
                        range.start <= range.end,
                        "Changes: {changes:?} | Range: {range:?} | Tabstop_i: {tabstop_i:?} | Snippet_range: {snippet_range:?} | Prev: {prev:?}"
                    );
                    if prev.end > range.start {
                        // not really sure what to do in this case. It shouldn't
                        // really occur in practice, the below just ensures
                        // our invariants hold
                        range.start = prev.end;
                        range.end = range.end.max(range.start)
                    }
                    prev = *range;
                }
                retain
            });
        }
        self.ranges.iter().all(|range| range.end <= range.start)
    }

    pub fn next_tabstop(&mut self, current_selection: &Selection) -> (Selection, bool) {
        let primary_idx = self.primary_idx(current_selection);
        while self.current_tabstop.0 + 1 < self.tabstops.len() {
            self.current_tabstop.0 += 1;
            if self.activate_tabstop() {
                let selection = self.tabstop_selection(primary_idx, Direction::Forward);
                return (selection, self.current_tabstop.0 + 1 == self.tabstops.len());
            }
        }

        (
            self.tabstop_selection(primary_idx, Direction::Forward),
            true,
        )
    }

    pub fn prev_tabstop(&mut self, current_selection: &Selection) -> Option<Selection> {
        let primary_idx = self.primary_idx(current_selection);
        while self.current_tabstop.0 != 0 {
            self.current_tabstop.0 -= 1;
            if self.activate_tabstop() {
                return Some(self.tabstop_selection(primary_idx, Direction::Forward));
            }
        }
        None
    }
    /// Computes the primary index adjusted for the number of cursors in the current tabstop.
    /// We allow for the selection range to 1 over the end to avoid issues with tabstops
    /// at the very end of snippets.
    fn primary_idx(&self, current_selection: &Selection) -> usize {
        let primary: Range = current_selection.primary().into();
        let res = self
            .ranges
            .iter()
            .position(|&range| range.start <= primary.start && primary.end <= range.end + 1);
        res.unwrap_or_else(|| {
            unreachable!(
                "active snippet must be valid {current_selection:?} {:?}",
                self.ranges
            )
        })
    }

    fn activate_tabstop(&mut self) -> bool {
        // We can activate a tabstop if are not at the final one
        if self.current_tabstop.0 == self.tabstops.len() - 1 {
            return false;
        }
        self.active_tabstops.clear();
        self.active_tabstops.insert(self.current_tabstop);
        let mut parent = self[self.current_tabstop].parent;
        while let Some(tabstop) = parent {
            self.active_tabstops.insert(tabstop);
            parent = self[tabstop].parent;
        }
        true
        // TODO: if the user removes the selection(s) in one snippet (but
        // there are still other cursors in other snippets) and jumps to the
        // next tabstop the selection in that tabstop is restored (at the
        // next tabstop). This could be annoying since its not possible to
        // remove a snippet cursor until the snippet is complete. On the other
        // hand it may be useful since the user may just have meant to edit
        // a subselection (like with s) of the tabstops and so the selection
        // removal was just temporary. Potentially this could have some sort of
        // separate keymap
    }

    pub fn tabstop_selection(&self, primary_idx: usize, direction: Direction) -> Selection {
        let tabstop = &self[self.current_tabstop];
        tabstop.selection(direction, primary_idx, self.ranges.len())
    }

    pub fn insert_subsnippet(mut self, snippet: RenderedSnippet) -> Option<Self> {
        if !snippet.ranges.len().is_multiple_of(self.ranges.len())
            || !is_exact_subset(self.ranges.iter().copied(), snippet.ranges.iter().copied())
        {
            log::warn!("number of subsnippets did not match, discarding outer snippet");
            return ActiveSnippet::new(snippet);
        }
        // The tabstops of `snippet` replace the current tabstop, so every tabstop after it moves.
        // Update the parents that point to a tabstop that moved.
        let shift = snippet.tabstops.len() - 1;
        for tabstop in &mut self.tabstops {
            if let Some(parent) = &mut tabstop.parent {
                if parent.0 > self.current_tabstop.0 {
                    parent.0 += shift;
                }
            }
        }
        let parent = self[self.current_tabstop].parent;
        let tabstops = snippet.tabstops.into_iter().map(|mut tabstop| {
            if let Some(parent) = &mut tabstop.parent {
                parent.0 += self.current_tabstop.0;
            } else {
                tabstop.parent = parent;
            }
            tabstop
        });
        self.tabstops
            .splice(self.current_tabstop.0..=self.current_tabstop.0, tabstops);
        self.activate_tabstop();
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use std::iter::{self};

    use helix_stdx::Range;
    use ropey::Rope;

    use crate::snippets::{ActiveSnippet, Snippet, SnippetRenderCtx, TabstopIdx};
    use crate::{Selection, Transaction};

    #[test]
    fn fully_remove() {
        let snippet = Snippet::parse("foo(${1:bar})$0").unwrap();
        let mut doc = Rope::from("bar.\n");
        let (transaction, _, snippet) = snippet.render(
            &doc,
            &Selection::point(4),
            |_| (4, 4),
            &mut SnippetRenderCtx::test_ctx(),
        );
        assert!(transaction.apply(&mut doc));
        assert_eq!(doc, "bar.foo(bar)\n");
        let mut snippet = ActiveSnippet::new(snippet).unwrap();
        let edit = Transaction::change(&doc, iter::once((4, 12, None)));
        assert!(edit.apply(&mut doc));
        snippet.map(edit.changes());
        assert!(!snippet.is_valid(&Selection::point(4)))
    }

    #[test]
    fn tabstop_zero_with_placeholder() {
        // The `$0` tabstop should not have placeholder text. When we receive a snippet like this
        // (from older versions of clangd for example) we should discard the placeholder text.
        let snippet = Snippet::parse("sizeof(${0:expression-or-type})").unwrap();
        let mut doc = Rope::from("\n");
        let (transaction, _, snippet) = snippet.render(
            &doc,
            &Selection::point(0),
            |_| (0, 0),
            &mut SnippetRenderCtx::test_ctx(),
        );
        assert!(transaction.apply(&mut doc));
        assert_eq!(doc, "sizeof()\n");
        assert!(ActiveSnippet::new(snippet).is_none());
    }

    #[test]
    fn insert_subsnippet_rebases_parents_of_following_tabstops() {
        // outer snippet: `$3` is nested inside `$2`, both come after `$1`
        let outer = Snippet::parse("x$1 ${2:a ${3:b}}").unwrap();
        let mut doc = Rope::from("s\n");
        let (transaction, _, outer) = outer.render(
            &doc,
            &Selection::point(1),
            |_| (1, 1),
            &mut SnippetRenderCtx::test_ctx(),
        );
        assert!(transaction.apply(&mut doc));
        assert_eq!(doc, "sx a b\n");
        let mut active = ActiveSnippet::new(outer).unwrap();

        // accept a completion snippet with one tabstop (plus `$0`) at `$1`
        let inner = Snippet::parse("f($1)").unwrap();
        let (transaction, _, inner) = inner.render(
            &doc,
            &Selection::point(2),
            |_| (2, 2),
            &mut SnippetRenderCtx::test_ctx(),
        );
        assert!(transaction.apply(&mut doc));
        assert_eq!(doc, "sxf() a b\n");
        // what the DocumentDidChange hook does before completion.rs calls insert_subsnippet
        assert!(!active.map(transaction.changes()));
        let mut active = active.insert_subsnippet(inner).unwrap();

        // tabstops are now [f:$1, f:$0, $2, $3, $0]; `$3` must still point at `$2`
        assert_eq!(active.tabstops[2].ranges[0], Range { start: 6, end: 9 });
        assert_eq!(active.tabstops[3].parent, Some(TabstopIdx(2)));

        // jumping to `$3` must activate its enclosing `$2`
        for _ in 0..3 {
            active.next_tabstop(&Selection::point(4));
        }
        assert!(active.active_tabstops.contains(&TabstopIdx(2)));
    }
}
