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
                (&mut range.end, Assoc::Before),
            ]
        });
        changes.update_positions(positions_to_map);

        for (i, tabstop) in self.tabstops.iter_mut().enumerate() {
            if self.active_tabstops.contains(&TabstopIdx(i)) {
                let positions_to_map = tabstop.ranges.iter_mut().flat_map(|range| {
                    let end_assoc = if range.start == range.end {
                        Assoc::Before
                    } else {
                        Assoc::After
                    };
                    [
                        (&mut range.start, Assoc::Before),
                        (&mut range.end, end_assoc),
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
                    range.start = range.start.max(snippet_range.start);
                    range.end = range.end.max(range.start).min(snippet_range.end);
                    // guaranteed by assoc
                    debug_assert!(prev.start <= range.start);
                    debug_assert!(range.start <= range.end);
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
    // computes the primary idx adjusted for the number of cursors in the current tabstop
    fn primary_idx(&self, current_selection: &Selection) -> usize {
        let primary: Range = current_selection.primary().into();
        let res = self
            .ranges
            .iter()
            .position(|&range| range.contains(primary));
        res.unwrap_or_else(|| {
            unreachable!(
                "active snippet must be valid {current_selection:?} {:?}",
                self.ranges
            )
        })
    }

    fn activate_tabstop(&mut self) -> bool {
        let tabstop = &self[self.current_tabstop];
        if tabstop.has_placeholder() && tabstop.ranges.iter().all(|range| range.is_empty()) {
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
        let mut cnt = 0;
        let parent = self[self.current_tabstop].parent;
        let tabstops = snippet.tabstops.into_iter().map(|mut tabstop| {
            cnt += 1;
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

    use ropey::Rope;

    use crate::snippets::{ActiveSnippet, Snippet, SnippetRenderCtx};
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
}
