use crate::{ChangeSet, Rope, State, Transaction};
use smallvec::{smallvec, SmallVec};
use std::time::{Duration, Instant};

// Stores the history of changes to a buffer.
//
// Currently the history is represented as a vector of revisions. The vector
// always has at least one element: the empty root revision. Each revision
// with the exception of the root has a parent revision, a [Transaction]
// that can be applied to its parent to transition from the parent to itself,
// and an inversion of that transaction to transition from the parent to its
// latest child.
//
// When using `u` to undo a change, an inverse of the stored transaction will
// be applied which will transition the buffer to the parent state.
//
// Each revision with the exception of the last in the vector also has a
// last child revision. When using `U` to redo a change, the last child transaction
// will be applied to the current state of the buffer.
//
// The current revision is the one currently displayed in the buffer.
//
// Commiting a new revision to the history will update the last child of the
// current revision, and push a new revision to the end of the vector.
//
// Revisions are commited with a timestamp. :earlier and :later can be used
// to jump to the closest revision to a moment in time relative to the timestamp
// of the current revision plus (:earlier) or minus (:later) the duration
// given to the command. If a single integer is given, the editor will instead
// jump the given number of revisions in the vector.
//
// Limitations:
//  * Changes in selections currently don't commit history changes. The selection
//    will be updated when switching revisions only if the revision
//  * The vector of history revisions is currently unbounded. This might
//    cause the memory consumption to grow significantly large during long
//    editing sessions.
//  * Because delete transactions currently don't store the text that they
//    delete, we also store an inversion of the transaction.
pub struct History {
    revisions: Vec<Revision>,
    current: usize,
}

// A single point in history. See [History] for more information.
#[derive(Debug)]
struct Revision {
    parent: usize,
    last_child: usize,
    transaction: Transaction,
    inversion: Transaction,
    timestamp: Instant,
}

impl Default for History {
    fn default() -> Self {
        // Add a dummy root revision with empty transaction
        Self {
            revisions: vec![Revision {
                parent: 0,
                last_child: 0,
                transaction: Transaction::from(ChangeSet::new(&Rope::new())),
                inversion: Transaction::from(ChangeSet::new(&Rope::new())),
                timestamp: Instant::now(),
            }],
            current: 0,
        }
    }
}

impl History {
    pub fn commit_revision(&mut self, transaction: &Transaction, original: &State) {
        // TODO: could store a single transaction, if deletes also stored the text they delete
        let inversion = transaction
            .invert(&original.doc)
            // Store the current cursor position
            .with_selection(original.selection.clone());

        let new_current = self.revisions.len();
        self.revisions[self.current].last_child = new_current;
        self.revisions.push(Revision {
            parent: self.current,
            last_child: 0,
            transaction: transaction.clone(),
            inversion,
            timestamp: Instant::now(),
        });
        self.current = new_current;
    }

    #[inline]
    pub fn current_revision(&self) -> usize {
        self.current
    }

    #[inline]
    pub const fn at_root(&self) -> bool {
        self.current == 0
    }

    pub fn undo(&mut self) -> Option<&Transaction> {
        if self.at_root() {
            return None;
        }

        let current_revision = &self.revisions[self.current];
        self.current = current_revision.parent;
        Some(&current_revision.inversion)
    }

    pub fn redo(&mut self) -> Option<&Transaction> {
        let current_revision = &self.revisions[self.current];
        let last_child = current_revision.last_child;
        if last_child == 0 {
            return None;
        }
        self.current = last_child;

        let last_child_revision = &self.revisions[last_child];
        Some(&self.revisions[last_child].transaction)
    }

    fn lowest_common_ancestor(&self, mut a: usize, mut b: usize) -> usize {
        use std::collections::HashSet;
        let mut a_path_set = HashSet::new();
        let mut b_path_set = HashSet::new();
        loop {
            a_path_set.insert(a);
            b_path_set.insert(b);
            if a_path_set.contains(&b) {
                return b;
            }
            if b_path_set.contains(&a) {
                return a;
            }
            a = self.revisions[a].parent; // Relies on the parent of 0 being 0.
            b = self.revisions[b].parent; // Same as above.
        }
    }

    // List of nodes on the way from `n` to 'a`. Doesn`t include `a`.
    // Includes `n` unless `a == n`. `a` must be an ancestor of `n`.
    fn path_up(&self, mut n: usize, a: usize) -> Vec<usize> {
        let mut path = Vec::new();
        while n != a {
            path.push(n);
            n = self.revisions[n].parent;
        }
        path
    }

    fn jump_to(&mut self, to: usize) -> Vec<Transaction> {
        let lca = self.lowest_common_ancestor(self.current, to);
        let up = self.path_up(self.current, lca);
        let down = self.path_up(to, lca);
        self.current = to;
        let up_txns = up.iter().map(|&n| self.revisions[n].inversion.clone());
        let down_txns = down
            .iter()
            .rev()
            .map(|&n| self.revisions[n].transaction.clone());
        up_txns.chain(down_txns).collect()
    }

    fn jump_backward(&mut self, delta: usize) -> Vec<Transaction> {
        self.jump_to(self.current.saturating_sub(delta))
    }

    fn jump_forward(&mut self, delta: usize) -> Vec<Transaction> {
        use std::cmp::max;
        self.jump_to(max(
            self.revisions.len() - 1,
            self.current.saturating_add(delta),
        ))
    }

    // Helper for a binary search case below.
    fn revision_closer_to_instant(&self, i: usize, instant: Instant) -> usize {
        let dur_im1 = instant.duration_since(self.revisions[i - 1].timestamp);
        let dur_i = self.revisions[i].timestamp.duration_since(instant);
        use std::cmp::Ordering::*;
        match dur_im1.cmp(&dur_i) {
            Less => i - 1,
            Equal | Greater => i,
        }
    }

    fn jump_instant(&mut self, instant: Instant) -> Vec<Transaction> {
        let search_result = self
            .revisions
            .binary_search_by(|rev| rev.timestamp.cmp(&instant));
        let revision = match search_result {
            Ok(revision) => revision,
            Err(insert_point) => match insert_point {
                0 => 0,
                n if n == self.revisions.len() => n - 1,
                i => self.revision_closer_to_instant(i, instant),
            },
        };
        self.jump_to(revision)
    }

    fn jump_duration_backward(&mut self, duration: Duration) -> Vec<Transaction> {
        match self.revisions[self.current].timestamp.checked_sub(duration) {
            Some(instant) => self.jump_instant(instant),
            None => self.jump_to(0),
        }
    }

    fn jump_duration_forward(&mut self, duration: Duration) -> Vec<Transaction> {
        match self.revisions[self.current].timestamp.checked_add(duration) {
            Some(instant) => self.jump_instant(instant),
            None => self.jump_to(self.revisions.len() - 1),
        }
    }

    pub fn earlier(&mut self, sotp: StepsOrTimePeriod) -> Vec<Transaction> {
        use StepsOrTimePeriod::*;
        match sotp {
            Steps(n) => self.jump_backward(n),
            TimePeriod(d) => self.jump_duration_backward(d),
        }
    }

    pub fn later(&mut self, sotp: StepsOrTimePeriod) -> Vec<Transaction> {
        use StepsOrTimePeriod::*;
        match sotp {
            Steps(n) => self.jump_forward(n),
            TimePeriod(d) => self.jump_duration_forward(d),
        }
    }
}

pub enum StepsOrTimePeriod {
    Steps(usize),
    TimePeriod(std::time::Duration),
}

impl std::str::FromStr for StepsOrTimePeriod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(n) = s.parse::<usize>() {
            return Ok(StepsOrTimePeriod::Steps(n));
        }
        if let Ok(d) = parse_duration::parse(s) {
            return Ok(StepsOrTimePeriod::TimePeriod(d));
        }
        Err("couldn't parse the argument as a number of steps or a duration".to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_undo_redo() {
        let mut history = History::default();
        let doc = Rope::from("hello");
        let mut state = State::new(doc);

        let transaction1 =
            Transaction::change(&state.doc, vec![(5, 5, Some(" world!".into()))].into_iter());

        // Need to commit before applying!
        history.commit_revision(&transaction1, &state);
        transaction1.apply(&mut state.doc);
        assert_eq!("hello world!", state.doc);

        // ---

        let transaction2 =
            Transaction::change(&state.doc, vec![(6, 11, Some("世界".into()))].into_iter());

        // Need to commit before applying!
        history.commit_revision(&transaction2, &state);
        transaction2.apply(&mut state.doc);
        assert_eq!("hello 世界!", state.doc);

        // ---
        fn undo(history: &mut History, state: &mut State) {
            if let Some(transaction) = history.undo() {
                transaction.apply(&mut state.doc);
            }
        }
        fn redo(history: &mut History, state: &mut State) {
            if let Some(transaction) = history.redo() {
                transaction.apply(&mut state.doc);
            }
        }

        undo(&mut history, &mut state);
        assert_eq!("hello world!", state.doc);
        redo(&mut history, &mut state);
        assert_eq!("hello 世界!", state.doc);
        undo(&mut history, &mut state);
        undo(&mut history, &mut state);
        assert_eq!("hello", state.doc);

        // undo at root is a no-op
        undo(&mut history, &mut state);
        assert_eq!("hello", state.doc);
    }
}
