use crate::{Assoc, ChangeSet, Range, Rope, Selection, Transaction};
use once_cell::sync::Lazy;
use regex::Regex;
use std::io::{Read, Seek, SeekFrom, Write};
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::combinators::*;

#[derive(Debug, Clone)]
pub struct State {
    pub doc: Rope,
    pub selection: Selection,
}

/// Stores the history of changes to a buffer.
///
/// Currently the history is represented as a vector of revisions. The vector
/// always has at least one element: the empty root revision. Each revision
/// with the exception of the root has a parent revision, a [Transaction]
/// that can be applied to its parent to transition from the parent to itself,
/// and an inversion of that transaction to transition from the parent to its
/// latest child.
///
/// When using `u` to undo a change, an inverse of the stored transaction will
/// be applied which will transition the buffer to the parent state.
///
/// Each revision with the exception of the last in the vector also has a
/// last child revision. When using `U` to redo a change, the last child transaction
/// will be applied to the current state of the buffer.
///
/// The current revision is the one currently displayed in the buffer.
///
/// Committing a new revision to the history will update the last child of the
/// current revision, and push a new revision to the end of the vector.
///
/// Revisions are committed with a timestamp. :earlier and :later can be used
/// to jump to the closest revision to a moment in time relative to the timestamp
/// of the current revision plus (:later) or minus (:earlier) the duration
/// given to the command. If a single integer is given, the editor will instead
/// jump the given number of revisions in the vector.
///
/// Limitations:
///  * Changes in selections currently don't commit history changes. The selection
///    will only be updated to the state after a committed buffer change.
///  * The vector of history revisions is currently unbounded. This might
///    cause the memory consumption to grow significantly large during long
///    editing sessions.
///  * Because delete transactions currently don't store the text that they
///    delete, we also store an inversion of the transaction.
///
/// Using time to navigate the history: <https://github.com/helix-editor/helix/pull/194>
#[derive(Debug)]
pub struct History {
    revisions: Vec<Revision>,
    current: usize,
}

/// A single point in history. See [History] for more information.
#[derive(Debug, Clone)]
struct Revision {
    parent: usize,
    last_child: Option<NonZeroUsize>,
    transaction: Arc<Transaction>,
    // We need an inversion for undos because delete transactions don't store
    // the deleted text.
    inversion: Arc<Transaction>,
    timestamp: SystemTime,
}

impl Default for History {
    fn default() -> Self {
        // Add a dummy root revision with empty transaction
        Self {
            revisions: vec![Revision {
                parent: 0,
                last_child: None,
                transaction: Arc::new(Transaction::from(ChangeSet::new("".into()))),
                inversion: Arc::new(Transaction::from(ChangeSet::new("".into()))),
                timestamp: SystemTime::now(),
            }],
            current: 0,
        }
    }
}

fn get_hash<R: Read>(reader: &mut R) -> std::io::Result<[u8; 20]> {
    const BUF_SIZE: usize = 8192;

    let mut buf = [0u8; BUF_SIZE];
    let mut hash = sha1_smol::Sha1::new();
    loop {
        let total_read = reader.read(&mut buf)?;
        if total_read == 0 {
            break;
        }

        hash.update(&buf[0..total_read]);
    }
    Ok(hash.digest().bytes())
}

// TODO: For testing only.
fn is_tree(n: usize, nodes: &[Revision]) -> bool {
    use std::collections::{HashMap, HashSet};

    let mut adj_list = HashMap::with_capacity(n);
    let mut total_degree = 0;
    for (node, parent, last_child) in nodes
        .iter()
        .enumerate()
        .map(|(idx, r)| (idx, r.parent, r.last_child))
    {
        // Skip loop
        if !(node == 0 && parent == 0) {
            total_degree += 1;

            adj_list
                .entry(node)
                .or_insert_with(|| HashSet::with_capacity(2))
                .insert(parent);
            adj_list
                .entry(parent)
                .or_insert_with(|| HashSet::with_capacity(2))
                .insert(node);
        }

        if let Some(n) = last_child {
            total_degree += 1;
            adj_list
                .entry(node)
                .or_insert_with(|| HashSet::with_capacity(2))
                .insert(n.get());
            adj_list
                .entry(n.get())
                .or_insert_with(|| HashSet::with_capacity(2))
                .insert(node);
        }
    }

    if total_degree != 2 * (n - 1) {
        return false;
    }

    let mut visited = HashSet::new();
    let mut stack = vec![0];
    while let Some(node) = stack.pop() {
        if !visited.insert(node) {
            // Cycle
            return false;
        }

        if let Some(adj_nodes) = adj_list.get(&node) {
            for v in adj_nodes {
                if !visited.contains(v) {
                    stack.push(*v);
                }
            }
        }
    }
    visited.len() == n
}

#[derive(Debug)]
pub enum StateError {
    Outdated,
    InvalidHeader,
    InvalidOffset,
    InvalidData(String),
    InvalidTree,
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Outdated => f.write_str("Outdated file"),
            Self::InvalidHeader => f.write_str("Invalid undofile header"),
            Self::InvalidOffset => f.write_str("Invalid merge offset"),
            Self::InvalidData(msg) => f.write_str(msg),
            Self::InvalidTree => f.write_str("not a tree"),
        }
    }
}

impl std::error::Error for StateError {}

const HEADER_TAG: &str = "Helix Undofile 1\n";

impl PartialEq for Revision {
    fn eq(&self, other: &Self) -> bool {
        self.parent == other.parent
            && self.last_child == other.last_child
            && self.transaction == other.transaction
            && self.inversion == other.inversion
    }
}
impl Revision {
    fn serialize<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        write_usize(writer, self.parent)?;
        self.transaction.serialize(writer)?;
        self.inversion.serialize(writer)?;
        write_usize(
            writer,
            self.timestamp
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as usize,
        )?;

        Ok(())
    }

    fn deserialize<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let parent = read_usize(reader)?;
        let transaction = Arc::new(Transaction::deserialize(reader)?);
        let inversion = Arc::new(Transaction::deserialize(reader)?);
        let timestamp = std::time::UNIX_EPOCH
            .checked_add(Duration::from_secs(read_usize(reader)? as u64))
            .unwrap_or_else(SystemTime::now);
        Ok(Revision {
            parent,
            last_child: None,
            transaction,
            inversion,
            timestamp,
        })
    }
}

struct HashWriter<'a, W: Write + Seek> {
    w: &'a mut W,
    hash: sha1_smol::Sha1,
}

impl<'a, W: Write + Seek> HashWriter<'a, W> {
    fn new(writer: &'a mut W) -> Self {
        Self {
            w: writer,
            hash: sha1_smol::Sha1::new(),
        }
    }

    fn get_hash(&self) -> [u8; 20] {
        self.hash.digest().bytes()
    }
}

impl<'a, W: Write + Seek> Write for HashWriter<'a, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.hash.update(buf);
        self.w.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.w.flush()
    }
}

impl<'a, W: Write + Seek> Seek for HashWriter<'a, W> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.w.seek(pos)
    }
}

struct HashReader<'a, R: Read> {
    r: &'a mut R,
    hash: sha1_smol::Sha1,
}

impl<'a, R: Read> HashReader<'a, R> {
    fn new(r: &'a mut R) -> Self {
        Self {
            r,
            hash: sha1_smol::Sha1::new(),
        }
    }

    fn get_hash(&self) -> [u8; 20] {
        self.hash.digest().bytes()
    }
}

impl<'a, R: Read> Read for HashReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.hash.update(buf);
        self.r.read(buf)
    }
}

impl History {
    // TODO: Include hash for undofile itself
    pub fn serialize<W: Write + Seek>(
        &self,
        writer: &mut W,
        path: &Path,
        revision: usize,
        last_saved_revision: usize,
    ) -> anyhow::Result<()> {
        // Header
        let mtime = std::fs::metadata(path)?
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        write_string(writer, HEADER_TAG)?;
        write_usize(writer, self.current)?;
        write_usize(writer, revision)?;
        write_u64(writer, mtime)?;
        writer.write_all(&get_hash(&mut std::fs::File::open(path)?)?)?;

        let pos = writer.seek(SeekFrom::Current(0))?;
        // Undofile hash placeholder
        writer.write_all(&[0u8; 20])?;

        let mut hasher = HashWriter::new(writer);

        // Append new revisions to the end of the file.
        write_usize(&mut hasher, self.revisions.len())?;
        hasher.seek(SeekFrom::End(0))?;
        for rev in &self.revisions[last_saved_revision..] {
            rev.serialize(&mut hasher)?;
        }

        let hash = hasher.get_hash();
        writer.seek(SeekFrom::Start(pos))?;
        writer.write_all(&hash)?;

        Ok(())
    }

    /// Returns the deserialized [`History`] and the last_saved_revision.
    pub fn deserialize<R: Read>(reader: &mut R, path: &Path) -> anyhow::Result<(usize, Self)> {
        let (current, last_saved_revision, hash) = Self::read_header(reader, path)?;
        let mut reader = HashReader::new(reader);

        // Read the revisions and construct the tree.
        let len = read_usize(&mut reader)?;
        let mut revisions: Vec<Revision> = Vec::with_capacity(len);
        for _ in 0..len {
            let rev = Revision::deserialize(&mut reader)?;
            let len = revisions.len();
            match revisions.get_mut(rev.parent) {
                Some(r) => r.last_child = NonZeroUsize::new(len),
                None if len != 0 => {
                    anyhow::bail!(StateError::InvalidData(format!(
                        "non-contiguous history: {} >= {}",
                        rev.parent, len
                    )));
                }
                None => {
                    // Starting revision check
                    let default_rev = History::default().revisions.pop().unwrap();
                    if rev != default_rev {
                        anyhow::bail!(StateError::InvalidData(String::from(
                            "Missing 0th revision"
                        )));
                    }
                }
            }
            revisions.push(rev);
        }

        if !is_tree(len, &revisions) || hash != reader.get_hash() {
            anyhow::bail!(StateError::InvalidTree);
        }
        let history = History { current, revisions };
        Ok((last_saved_revision, history))
    }

    /// If `self.revisions = [A, B, C, D]` and `other.revisions = `[A, B, E, F]`, then
    /// they are merged into `[A, B, E, F, C, D]` where the tree can be represented as:
    /// ```md
    /// A -> B -> C -> D
    ///       \  
    ///        E -> F
    /// ```
    // TODO: return transaction to update view
    // TODO: Which history should take precedence?
    pub fn merge(&mut self, mut other: History) -> anyhow::Result<()> {
        let after_n = self
            .revisions
            .iter()
            .zip(other.revisions.iter())
            .take_while(|(a, b)| {
                a.parent == b.parent && a.transaction == b.transaction && a.inversion == b.inversion
            })
            .count();

        let revisions = self.revisions.split_off(after_n);
        other.revisions.reserve_exact(revisions.len());

        // Converts the number of new elements to an index offset
        let offset = (other.revisions.len() - after_n) - 1;
        for mut r in revisions {
            // Update parents of new revisions
            if r.parent >= after_n {
                r.parent += offset;
            }
            debug_assert!(r.parent < other.revisions.len());

            // Update the corresponding parent.
            other.revisions.get_mut(r.parent).unwrap().last_child =
                NonZeroUsize::new(other.revisions.len());
            other.revisions.push(r);
        }
        self.current += offset;
        self.revisions = other.revisions;

        if !is_tree(self.revisions.len(), &self.revisions) {
            anyhow::bail!(StateError::InvalidTree);
        }
        Ok(())
    }

    pub fn read_header<R: Read>(
        reader: &mut R,
        path: &Path,
    ) -> anyhow::Result<(usize, usize, [u8; 20])> {
        let header = read_string(reader)?;
        if HEADER_TAG != header {
            Err(anyhow::anyhow!(StateError::InvalidHeader))
        } else {
            let current = read_usize(reader)?;
            let last_saved_revision = read_usize(reader)?;
            let mtime = read_u64(reader)?;
            let last_mtime = std::fs::metadata(path)?
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let mut hash = [0u8; 20];
            reader.read_exact(&mut hash)?;

            if mtime != last_mtime && hash != get_hash(&mut std::fs::File::open(path)?)? {
                anyhow::bail!(StateError::Outdated);
            }

            // Tree hash
            reader.read_exact(&mut hash)?;
            Ok((current, last_saved_revision, hash))
        }
    }
}

impl History {
    pub fn commit_revision(&mut self, transaction: &Transaction, original: &State) {
        self.commit_revision_at_timestamp(transaction, original, SystemTime::now());
    }

    pub fn commit_revision_at_timestamp(
        &mut self,
        transaction: &Transaction,
        original: &State,
        timestamp: SystemTime,
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
            transaction: Arc::new(transaction.clone()),
            inversion: Arc::new(inversion),
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

    pub fn is_empty(&self) -> bool {
        self.revisions.len() <= 1
    }

    /// Returns the changes since the given revision composed into a transaction.
    /// Returns None if there are no changes between the current and given revisions.
    pub fn changes_since(&self, revision: usize) -> Option<Transaction> {
        let lca = self.lowest_common_ancestor(revision, self.current);
        let up = self.path_up(revision, lca);
        let down = self.path_up(self.current, lca);
        let up_txns = up
            .iter()
            .rev()
            .map(|&n| self.revisions[n].inversion.as_ref().clone());
        let down_txns = down
            .iter()
            .map(|&n| self.revisions[n].transaction.as_ref().clone());

        down_txns.chain(up_txns).reduce(|acc, tx| tx.compose(acc))
    }

    /// Undo the last edit.
    pub fn undo(&mut self) -> Option<&Transaction> {
        if self.at_root() {
            return None;
        }

        let current_revision = &self.revisions[self.current];
        self.current = current_revision.parent;
        Some(&current_revision.inversion)
    }

    /// Redo the last edit.
    pub fn redo(&mut self) -> Option<&Transaction> {
        let current_revision = &self.revisions[self.current];
        let last_child = current_revision.last_child?;
        self.current = last_child.get();

        Some(&self.revisions[last_child.get()].transaction)
    }

    // Get the position of last change
    pub fn last_edit_pos(&self) -> Option<usize> {
        if self.current == 0 {
            return None;
        }
        let current_revision = &self.revisions[self.current];
        let primary_selection = current_revision
            .inversion
            .selection()
            .expect("inversion always contains a selection")
            .primary();
        let (_from, to, _fragment) = current_revision
            .transaction
            .changes_iter()
            // find a change that matches the primary selection
            .find(|(from, to, _fragment)| Range::new(*from, *to).overlaps(&primary_selection))
            // or use the first change
            .or_else(|| current_revision.transaction.changes_iter().next())
            .unwrap();
        let pos = current_revision
            .transaction
            .changes()
            .map_pos(to, Assoc::After);
        Some(pos)
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

    /// List of nodes on the way from `n` to 'a`. Doesn't include `a`.
    /// Includes `n` unless `a == n`. `a` must be an ancestor of `n`.
    fn path_up(&self, mut n: usize, a: usize) -> Vec<usize> {
        let mut path = Vec::new();
        while n != a {
            path.push(n);
            n = self.revisions[n].parent;
        }
        path
    }

    /// Create a [`Transaction`] that will jump to a specific revision in the history.
    fn jump_to(&mut self, to: usize) -> Vec<Transaction> {
        let lca = self.lowest_common_ancestor(self.current, to);
        let up = self.path_up(self.current, lca);
        let down = self.path_up(to, lca);
        self.current = to;
        let up_txns = up
            .iter()
            .map(|&n| self.revisions[n].inversion.as_ref().clone());
        let down_txns = down
            .iter()
            .rev()
            .map(|&n| self.revisions[n].transaction.as_ref().clone());
        up_txns.chain(down_txns).collect()
    }

    /// Creates a [`Transaction`] that will undo `delta` revisions.
    fn jump_backward(&mut self, delta: usize) -> Vec<Transaction> {
        self.jump_to(self.current.saturating_sub(delta))
    }

    /// Creates a [`Transaction`] that will redo `delta` revisions.
    fn jump_forward(&mut self, delta: usize) -> Vec<Transaction> {
        self.jump_to(
            self.current
                .saturating_add(delta)
                .min(self.revisions.len() - 1),
        )
    }

    /// Helper for a binary search case below.
    fn revision_closer_to_time(&self, i: usize, timestamp: SystemTime) -> usize {
        let dur_im1 = timestamp
            .duration_since(self.revisions[i - 1].timestamp)
            .unwrap_or_default();
        let dur_i = self.revisions[i]
            .timestamp
            .duration_since(timestamp)
            .unwrap_or_default();
        use std::cmp::Ordering::*;
        match dur_im1.cmp(&dur_i) {
            Less => i - 1,
            Equal | Greater => i,
        }
    }

    /// Creates a [`Transaction`] that will match a revision created at around
    /// `time`.
    fn jump_time(&mut self, time: SystemTime) -> Vec<Transaction> {
        let search_result = self
            .revisions
            .binary_search_by(|rev| rev.timestamp.cmp(&time));
        let revision = match search_result {
            Ok(revision) => revision,
            Err(insert_point) => match insert_point {
                0 => 0,
                n if n == self.revisions.len() => n - 1,
                i => self.revision_closer_to_time(i, time),
            },
        };
        self.jump_to(revision)
    }

    /// Creates a [`Transaction`] that will match a revision created `duration` ago
    /// from the timestamp of current revision.
    fn jump_duration_backward(&mut self, duration: Duration) -> Vec<Transaction> {
        match self.revisions[self.current].timestamp.checked_sub(duration) {
            Some(timestamp) => self.jump_time(timestamp),
            None => self.jump_to(0),
        }
    }

    /// Creates a [`Transaction`] that will match a revision created `duration` in
    /// the future from the timestamp of the current revision.
    fn jump_duration_forward(&mut self, duration: Duration) -> Vec<Transaction> {
        match self.revisions[self.current].timestamp.checked_add(duration) {
            Some(timestamp) => self.jump_time(timestamp),
            None => self.jump_to(self.revisions.len() - 1),
        }
    }

    /// Creates an undo [`Transaction`].
    pub fn earlier(&mut self, uk: UndoKind) -> Vec<Transaction> {
        use UndoKind::*;
        match uk {
            Steps(n) => self.jump_backward(n),
            TimePeriod(d) => self.jump_duration_backward(d),
        }
    }

    /// Creates a redo [`Transaction`].
    pub fn later(&mut self, uk: UndoKind) -> Vec<Transaction> {
        use UndoKind::*;
        match uk {
            Steps(n) => self.jump_forward(n),
            TimePeriod(d) => self.jump_duration_forward(d),
        }
    }
}

/// Whether to undo by a number of edits or a duration of time.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UndoKind {
    Steps(usize),
    TimePeriod(std::time::Duration),
}

/// A subset of systemd.time time span syntax units.
const TIME_UNITS: &[(&[&str], &str, u64)] = &[
    (&["seconds", "second", "sec", "s"], "seconds", 1),
    (&["minutes", "minute", "min", "m"], "minutes", 60),
    (&["hours", "hour", "hr", "h"], "hours", 60 * 60),
    (&["days", "day", "d"], "days", 24 * 60 * 60),
];

/// Checks if the duration input can be turned into a valid duration. It must be a
/// positive integer and denote the [unit of time.](`TIME_UNITS`)
/// Examples of valid durations:
///  * `5 sec`
///  * `5 min`
///  * `5 hr`
///  * `5 days`
static DURATION_VALIDATION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?:\d+\s*[a-z]+\s*)+$").unwrap());

/// Captures both the number and unit as separate capture groups.
static NUMBER_UNIT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)\s*([a-z]+)").unwrap());

/// Parse a string (e.g. "5 sec") and try to convert it into a [`Duration`].
fn parse_human_duration(s: &str) -> Result<Duration, String> {
    if !DURATION_VALIDATION_REGEX.is_match(s) {
        return Err("duration should be composed \
        of positive integers followed by time units"
            .to_string());
    }

    let mut specified = [false; TIME_UNITS.len()];
    let mut seconds = 0u64;
    for cap in NUMBER_UNIT_REGEX.captures_iter(s) {
        let (n, unit_str) = (&cap[1], &cap[2]);

        let n: u64 = n.parse().map_err(|_| format!("integer too large: {}", n))?;

        let time_unit = TIME_UNITS
            .iter()
            .enumerate()
            .find(|(_, (forms, _, _))| forms.iter().any(|f| f == &unit_str));

        if let Some((i, (_, unit, mul))) = time_unit {
            if specified[i] {
                return Err(format!("{} specified more than once", unit));
            }
            specified[i] = true;

            let new_seconds = n.checked_mul(*mul).and_then(|s| seconds.checked_add(s));
            match new_seconds {
                Some(ns) => seconds = ns,
                None => return Err("duration too large".to_string()),
            }
        } else {
            return Err(format!("incorrect time unit: {}", unit_str));
        }
    }

    Ok(Duration::from_secs(seconds))
}

impl std::str::FromStr for UndoKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            Ok(Self::Steps(1usize))
        } else if let Ok(n) = s.parse::<usize>() {
            Ok(UndoKind::Steps(n))
        } else {
            Ok(Self::TimePeriod(parse_human_duration(s)?))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Selection;

    #[test]
    fn test_undo_redo() {
        let mut history = History::default();
        let doc = Rope::from("hello");
        let mut state = State {
            doc,
            selection: Selection::point(0),
        };

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
        let mut state = State {
            doc,
            selection: Selection::point(0),
        };

        fn undo(history: &mut History, state: &mut State) {
            if let Some(transaction) = history.undo() {
                transaction.apply(&mut state.doc);
            }
        }

        fn earlier(history: &mut History, state: &mut State, uk: UndoKind) {
            let txns = history.earlier(uk);
            for txn in txns {
                txn.apply(&mut state.doc);
            }
        }

        fn later(history: &mut History, state: &mut State, uk: UndoKind) {
            let txns = history.later(uk);
            for txn in txns {
                txn.apply(&mut state.doc);
            }
        }

        fn commit_change(
            history: &mut History,
            state: &mut State,
            change: crate::transaction::Change,
            time: SystemTime,
        ) {
            let txn = Transaction::change(&state.doc, vec![change].into_iter());
            history.commit_revision_at_timestamp(&txn, state, time);
            txn.apply(&mut state.doc);
        }

        let t0 = SystemTime::now();
        let t = |n| t0.checked_add(Duration::from_secs(n)).unwrap();

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

        use UndoKind::*;

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
    fn test_parse_undo_kind() {
        use UndoKind::*;

        // Default is one step.
        assert_eq!("".parse(), Ok(Steps(1)));

        // An integer means the number of steps.
        assert_eq!("1".parse(), Ok(Steps(1)));
        assert_eq!("  16 ".parse(), Ok(Steps(16)));

        // Duration has a strict format.
        let validation_err = Err("duration should be composed \
         of positive integers followed by time units"
            .to_string());
        assert_eq!("  16 33".parse::<UndoKind>(), validation_err);
        assert_eq!("  seconds 22  ".parse::<UndoKind>(), validation_err);
        assert_eq!("  -4 m".parse::<UndoKind>(), validation_err);
        assert_eq!("5s 3".parse::<UndoKind>(), validation_err);

        // Units are u64.
        assert_eq!(
            "18446744073709551616minutes".parse::<UndoKind>(),
            Err("integer too large: 18446744073709551616".to_string())
        );

        // Units are validated.
        assert_eq!(
            "1 millennium".parse::<UndoKind>(),
            Err("incorrect time unit: millennium".to_string())
        );

        // Units can't be specified twice.
        assert_eq!(
            "2 seconds 6s".parse::<UndoKind>(),
            Err("seconds specified more than once".to_string())
        );

        // Various formats are correctly handled.
        assert_eq!(
            "4s".parse::<UndoKind>(),
            Ok(TimePeriod(Duration::from_secs(4)))
        );
        assert_eq!(
            "2m".parse::<UndoKind>(),
            Ok(TimePeriod(Duration::from_secs(120)))
        );
        assert_eq!(
            "5h".parse::<UndoKind>(),
            Ok(TimePeriod(Duration::from_secs(5 * 60 * 60)))
        );
        assert_eq!(
            "3d".parse::<UndoKind>(),
            Ok(TimePeriod(Duration::from_secs(3 * 24 * 60 * 60)))
        );
        assert_eq!(
            "1m30s".parse::<UndoKind>(),
            Ok(TimePeriod(Duration::from_secs(90)))
        );
        assert_eq!(
            "1m 20 seconds".parse::<UndoKind>(),
            Ok(TimePeriod(Duration::from_secs(80)))
        );
        assert_eq!(
            "  2 minute 1day".parse::<UndoKind>(),
            Ok(TimePeriod(Duration::from_secs(24 * 60 * 60 + 2 * 60)))
        );
        assert_eq!(
            "3 d 2hour 5 minutes 30sec".parse::<UndoKind>(),
            Ok(TimePeriod(Duration::from_secs(
                3 * 24 * 60 * 60 + 2 * 60 * 60 + 5 * 60 + 30
            )))
        );

        // Sum overflow is handled.
        assert_eq!(
            "18446744073709551615minutes".parse::<UndoKind>(),
            Err("duration too large".to_string())
        );
        assert_eq!(
            "1 minute 18446744073709551615 seconds".parse::<UndoKind>(),
            Err("duration too large".to_string())
        );
    }
}
