use crate::{ChangeSet, Rope, State, Transaction};
use once_cell::sync::Lazy;
use regex::Regex;
use smallvec::{smallvec, SmallVec};
use std::num::NonZeroUsize;
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
// of the current revision plus (:later) or minus (:earlier) the duration
// given to the command. If a single integer is given, the editor will instead
// jump the given number of revisions in the vector.
//
// Limitations:
//  * Changes in selections currently don't commit history changes. The selection
//    will only be updated to the state after a commited buffer change.
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
    last_child: Option<NonZeroUsize>,
    transaction: Transaction,
    // We need an inversion for undos because delete transactions don't store
    // the deleted text.
    inversion: Transaction,
    timestamp: Instant,
}

impl Default for History {
    fn default() -> Self {
        // Add a dummy root revision with empty transaction
        Self {
            revisions: vec![Revision {
                parent: 0,
                last_child: None,
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
        self.commit_revision_at_timestamp(transaction, original, Instant::now());
    }

    pub fn commit_revision_at_timestamp(
        &mut self,
        transaction: &Transaction,
        original: &State,
        timestamp: Instant,
    ) {
        let inversion = transaction
            .invert(&original.doc)
            // Store the current cursor position
            .with_selection(original.selection.clone());

        let new_current = self.revisions.len();
        self.revisions[self.current].last_child = NonZeroUsize::new(new_current);
        self.revisions.push(Revision {
            parent: self.current,
            last_child: None,
            transaction: transaction.clone(),
            inversion,
            timestamp,
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
        let last_child = match current_revision.last_child {
            None => return None,
            Some(n) => n,
        };
        self.current = last_child.get();

        let last_child_revision = &self.revisions[last_child.get()];
        Some(&self.revisions[last_child.get()].transaction)
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
        self.jump_to(
            self.current
                .saturating_add(delta)
                .min(self.revisions.len() - 1),
        )
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

#[derive(Debug, PartialEq)]
pub enum StepsOrTimePeriod {
    Steps(usize),
    TimePeriod(std::time::Duration),
}

const TIME_UNITS: &[&str] = &["seconds", "minutes", "hours", "days"];

fn find_time_unit(s: &str) -> Result<&'static str, String> {
    for unit in TIME_UNITS {
        if unit.starts_with(&s.to_lowercase()) {
            return Ok(unit);
        }
    }
    Err(format!("incorrect time unit: {}", s))
}

static DURATION_VALIDATION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?:\d+\s*[a-zA-Z]+\s*)+$").unwrap());

static NUMBER_UNIT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)\s*([a-zA-Z]+)").unwrap());

fn parse_human_duration(s: &str) -> Result<Duration, String> {
    if !DURATION_VALIDATION_REGEX.is_match(s) {
        return Err("duration should be composed \
        of positive integers followed by time units"
            .to_string());
    }

    let mut units = std::collections::HashMap::new();
    for cap in NUMBER_UNIT_REGEX.captures_iter(s) {
        let (n_str, short_unit) = (&cap[1], &cap[2]);

        let n = match n_str.parse::<u64>() {
            Err(_) => return Err(format!("integer too large: {}", n_str)),
            Ok(n) => n,
        };

        let unit = find_time_unit(short_unit)?;
        if units.contains_key(unit) {
            return Err(format!("{} specified more than once", unit));
        }
        units.insert(unit, n);
    }

    let units_in_seconds = [
        Some(*units.get("seconds").unwrap_or(&0)),
        units.get("minutes").unwrap_or(&0).checked_mul(60),
        units.get("hours").unwrap_or(&0).checked_mul(60 * 60),
        units.get("days").unwrap_or(&0).checked_mul(60 * 60 * 24),
    ];
    // A checked sum.
    let seconds = units_in_seconds
        .iter()
        .try_fold(0u64, |acc, x| x.and_then(|x| acc.checked_add(x)));

    match seconds {
        Some(seconds) => Ok(Duration::new(seconds, 0)),
        None => Err("duration too large".to_string()),
    }
}

impl std::str::FromStr for StepsOrTimePeriod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(Self::Steps(1usize));
        }
        if let Ok(n) = s.parse::<usize>() {
            return Ok(StepsOrTimePeriod::Steps(n));
        }
        let d = parse_human_duration(s)?;
        Ok(Self::TimePeriod(d))
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

    #[test]
    fn test_earlier_later() {
        let mut history = History::default();
        let doc = Rope::from("a\n");
        let mut state = State::new(doc);

        fn undo(history: &mut History, state: &mut State) {
            if let Some(transaction) = history.undo() {
                transaction.apply(&mut state.doc);
            }
        };

        fn earlier(history: &mut History, state: &mut State, sotp: StepsOrTimePeriod) {
            let txns = history.earlier(sotp);
            for txn in txns {
                txn.apply(&mut state.doc);
            }
        };

        fn later(history: &mut History, state: &mut State, sotp: StepsOrTimePeriod) {
            let txns = history.later(sotp);
            for txn in txns {
                txn.apply(&mut state.doc);
            }
        };

        fn commit_change(
            history: &mut History,
            state: &mut State,
            change: crate::transaction::Change,
            instant: Instant,
        ) {
            let txn = Transaction::change(&state.doc, vec![change.clone()].into_iter());
            history.commit_revision_at_timestamp(&txn, &state, instant);
            txn.apply(&mut state.doc);
        };

        let t0 = Instant::now();
        let t = |n| t0.checked_add(Duration::new(n, 0)).unwrap();

        commit_change(&mut history, &mut state, (1, 1, Some(" b".into())), t(0));
        assert_eq!("a b\n", state.doc);

        commit_change(&mut history, &mut state, (3, 3, Some(" c".into())), t(10));
        assert_eq!("a b c\n", state.doc);

        commit_change(&mut history, &mut state, (5, 5, Some(" d".into())), t(20));
        assert_eq!("a b c d\n", state.doc);

        undo(&mut history, &mut state);
        assert_eq!("a b c\n", state.doc);

        commit_change(&mut history, &mut state, (5, 5, Some(" e".into())), t(30));
        assert_eq!("a b c e\n", state.doc);

        undo(&mut history, &mut state);
        undo(&mut history, &mut state);
        assert_eq!("a b\n", state.doc);

        commit_change(&mut history, &mut state, (1, 3, None), t(40));
        assert_eq!("a\n", state.doc);

        commit_change(&mut history, &mut state, (1, 1, Some(" f".into())), t(50));
        assert_eq!("a f\n", state.doc);

        use StepsOrTimePeriod::*;

        earlier(&mut history, &mut state, Steps(3));
        assert_eq!("a b c d\n", state.doc);

        later(&mut history, &mut state, TimePeriod(Duration::new(20, 0)));
        assert_eq!("a\n", state.doc);

        earlier(&mut history, &mut state, TimePeriod(Duration::new(19, 0)));
        assert_eq!("a b c d\n", state.doc);

        earlier(
            &mut history,
            &mut state,
            TimePeriod(Duration::new(10000, 0)),
        );
        assert_eq!("a\n", state.doc);

        later(&mut history, &mut state, Steps(50));
        assert_eq!("a f\n", state.doc);

        earlier(&mut history, &mut state, Steps(4));
        assert_eq!("a b c\n", state.doc);

        later(&mut history, &mut state, TimePeriod(Duration::new(1, 0)));
        assert_eq!("a b c\n", state.doc);

        later(&mut history, &mut state, TimePeriod(Duration::new(5, 0)));
        assert_eq!("a b c d\n", state.doc);

        later(&mut history, &mut state, TimePeriod(Duration::new(6, 0)));
        assert_eq!("a b c e\n", state.doc);

        later(&mut history, &mut state, Steps(1));
        assert_eq!("a\n", state.doc);
    }

    #[test]
    fn test_parse_steps_or_time_period() {
        use StepsOrTimePeriod::*;

        // Default is one step.
        assert_eq!("".parse(), Ok(Steps(1)));

        // An integer means the number of steps.
        assert_eq!("1".parse(), Ok(Steps(1)));
        assert_eq!("  16 ".parse(), Ok(Steps(16)));

        // Duration has a strict format.
        let validation_err = Err("duration should be composed \
         of positive integers followed by time units"
            .to_string());
        assert_eq!("  16 33".parse::<StepsOrTimePeriod>(), validation_err);
        assert_eq!(
            "  seconds 22  ".parse::<StepsOrTimePeriod>(),
            validation_err
        );
        assert_eq!("  -4 m".parse::<StepsOrTimePeriod>(), validation_err);
        assert_eq!("5s 3".parse::<StepsOrTimePeriod>(), validation_err);

        // Units are u64.
        assert_eq!(
            "18446744073709551616minutes".parse::<StepsOrTimePeriod>(),
            Err("integer too large: 18446744073709551616".to_string())
        );

        // Units are validated.
        assert_eq!(
            "1 millenium".parse::<StepsOrTimePeriod>(),
            Err("incorrect time unit: millenium".to_string())
        );

        // Units can't be specified twice.
        assert_eq!(
            "2 seconds 6s".parse::<StepsOrTimePeriod>(),
            Err("seconds specified more than once".to_string())
        );

        // Various formats are correctly handled.
        assert_eq!(
            "4s".parse::<StepsOrTimePeriod>(),
            Ok(TimePeriod(Duration::new(4, 0)))
        );
        assert_eq!(
            "2m".parse::<StepsOrTimePeriod>(),
            Ok(TimePeriod(Duration::new(120, 0)))
        );
        assert_eq!(
            "5h".parse::<StepsOrTimePeriod>(),
            Ok(TimePeriod(Duration::new(5 * 60 * 60, 0)))
        );
        assert_eq!(
            "3d".parse::<StepsOrTimePeriod>(),
            Ok(TimePeriod(Duration::new(3 * 24 * 60 * 60, 0)))
        );
        assert_eq!(
            "1m30s".parse::<StepsOrTimePeriod>(),
            Ok(TimePeriod(Duration::new(90, 0)))
        );
        assert_eq!(
            "1m 20 seconds".parse::<StepsOrTimePeriod>(),
            Ok(TimePeriod(Duration::new(80, 0)))
        );
        assert_eq!(
            "  2 minute 1day".parse::<StepsOrTimePeriod>(),
            Ok(TimePeriod(Duration::new(24 * 60 * 60 + 2 * 60, 0)))
        );
        assert_eq!(
            "3 d 2hour 5 minutes 30sec".parse::<StepsOrTimePeriod>(),
            Ok(TimePeriod(Duration::new(
                3 * 24 * 60 * 60 + 2 * 60 * 60 + 5 * 60 + 30,
                0
            )))
        );

        // Sum overflow is handled.
        assert_eq!(
            "18446744073709551615minutes".parse::<StepsOrTimePeriod>(),
            Err("duration too large".to_string())
        );
        assert_eq!(
            "1 minute 18446744073709551615 seconds".parse::<StepsOrTimePeriod>(),
            Err("duration too large".to_string())
        );
    }
}
