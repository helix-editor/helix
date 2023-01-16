use std::collections::VecDeque;

use helix_core::Range;

/// Sequence of characters that need to be pressed to reach a destination.
#[derive(Debug, Eq, PartialEq)]
pub struct JumpSequence(Vec<u8>);

impl JumpSequence {
    /// Prefix the current sequence with the given character
    pub fn prefix(&mut self, c: u8) {
        assert!(c.is_ascii());
        // We are appending and not inserting to the 0th element, because the
        // consumation order is LIFO.
        self.0.push(c);
    }
}

impl From<u8> for JumpSequence {
    fn from(key: u8) -> Self {
        Self(vec![key])
    }
}

impl From<JumpSequence> for String {
    fn from(mut seq: JumpSequence) -> Self {
        seq.0.reverse();
        String::from_utf8(seq.0).expect("Jump keys should be ascii letters")
    }
}

#[derive(Debug)]
pub struct JumpAnnotation {
    // Starting location of a jump annotation.
    pub loc: usize,
    pub keys: String,
}

/// Generator that generates a list of jump annotations
pub trait JumpSequencer {
    /// Generates a list of JumpSequence. The order of the JumpSequence should
    /// be highest priority first, lowest priority last
    fn generate(&self) -> Vec<JumpAnnotation>;
    // Advance the state machine
    fn choose(self, key: u8) -> Option<Box<Self>>;
    // Returns Some if the sequencer is in a terminal state. None otherwise.
    // The value represents the target position we should jump to.
    fn try_get_range(&self) -> Option<Range>;
}

#[derive(Debug)]
pub struct TrieNode {
    key: u8,
    children: Vec<TrieNode>,
    // Some if leaf node. None otherwise.
    range_in_text: Option<Range>,
}

impl From<u8> for TrieNode {
    fn from(key: u8) -> Self {
        TrieNode {
            key,
            children: vec![],
            range_in_text: None, // Calculation happens after trie construction
        }
    }
}

fn make_trie_children(keys: &[u8]) -> Vec<TrieNode> {
    keys.iter().map(|c| TrieNode::from(*c)).collect()
}

fn attach_jump_targets_to_leaves(
    node: &mut TrieNode,
    jump_targets: &mut impl Iterator<Item = Range>,
) {
    if node.children.is_empty() {
        node.range_in_text = jump_targets.next();
        return;
    }
    for child in node.children.iter_mut() {
        attach_jump_targets_to_leaves(child, jump_targets);
    }
}

impl TrieNode {
    pub fn build(keys: &[u8], jump_targets: Vec<Range>) -> Self {
        assert!(!keys.is_empty());
        // Invalid key for the root node since it doesn't represent a key
        let mut root = TrieNode::from(0);
        let n = jump_targets.len();
        if n <= keys.len() {
            root.children = make_trie_children(&keys[0..n]);
            attach_jump_targets_to_leaves(&mut root, &mut jump_targets.into_iter());
            return root;
        }
        root.children = make_trie_children(keys);

        // Running BFS, expanding trie nodes along the way.
        let mut queue = VecDeque::with_capacity(root.children.len());
        // Reverse-iterating the children such that the last key gets expanded first
        queue.extend(root.children.iter_mut().rev());

        let mut remaining = n - keys.len();
        loop {
            let mut trie = queue.pop_front().unwrap();
            if remaining < keys.len() {
                // We need to make remaining + 1 children because the current leaf
                // node will no longer be a leaf node
                trie.children = make_trie_children(&keys[0..remaining + 1]);
                break;
            }
            trie.children = make_trie_children(keys);
            // subtract 1 to account for the no-longer-leaf node
            remaining -= keys.len() - 1;
            queue.extend(trie.children.iter_mut().rev());
        }
        attach_jump_targets_to_leaves(&mut root, &mut jump_targets.into_iter());
        root
    }
}

fn depth_first_search(node: &TrieNode) -> Vec<(Range, JumpSequence)> {
    let key = node.key;
    if node.children.is_empty() {
        return vec![(node.range_in_text.unwrap(), JumpSequence::from(key))];
    }
    node.children
        .iter()
        .flat_map(|child| {
            depth_first_search(child).into_iter().map(|(pos, mut v)| {
                v.prefix(key);
                (pos, v)
            })
        })
        .collect()
}

impl JumpSequencer for TrieNode {
    fn generate(&self) -> Vec<JumpAnnotation> {
        if self.children.is_empty() {
            return vec![];
        }
        let mut annotations: Vec<JumpAnnotation> = self
            .children
            .iter()
            .flat_map(|child| depth_first_search(child).into_iter())
            .map(|(range, sequence)| JumpAnnotation {
                loc: range.head,
                keys: String::from(sequence),
            })
            .collect();
        annotations.sort_by_key(|annot| annot.loc);
        annotations
    }

    fn choose(self, key: u8) -> Option<Box<Self>> {
        for child in self.children {
            if child.key == key {
                return Some(Box::new(child));
            }
        }
        None
    }

    fn try_get_range(&self) -> Option<Range> {
        self.range_in_text
    }
}

#[cfg(test)]
mod jump_tests {
    use super::*;

    fn next(it: &mut std::vec::IntoIter<JumpAnnotation>) -> Option<(String, usize)> {
        match it.next() {
            Some(jump) => Some((jump.keys, jump.loc)),
            None => None,
        }
    }

    fn iota(n: usize) -> Vec<Range> {
        (0..n).map(Range::point).collect()
    }

    #[test]
    fn more_keys_than_jump_targets() {
        let mut paths = TrieNode::build(b"abcdefg", iota(2)).generate().into_iter();
        assert_eq!(next(&mut paths), Some((String::from("a"), 0)));
        assert_eq!(next(&mut paths), Some((String::from("b"), 1)));
        assert_eq!(next(&mut paths), None);
    }

    #[test]
    fn equal_number_of_keys_and_jump_targets() {
        let mut paths = TrieNode::build(b"xyz", iota(3)).generate().into_iter();
        assert_eq!(next(&mut paths), Some((String::from("x"), 0)));
        assert_eq!(next(&mut paths), Some((String::from("y"), 1)));
        assert_eq!(next(&mut paths), Some((String::from("z"), 2)));
        assert_eq!(next(&mut paths), None);
    }

    #[test]
    fn more_jump_targets_than_keys_1() {
        let ranges = vec![1usize, 5, 9, 100]
            .into_iter()
            .map(Range::point)
            .collect();
        let mut paths = TrieNode::build(b"xyz", ranges).generate().into_iter();
        assert_eq!(next(&mut paths), Some((String::from("x"), 1)));
        assert_eq!(next(&mut paths), Some((String::from("y"), 5)));
        assert_eq!(next(&mut paths), Some((String::from("zx"), 9)));
        assert_eq!(next(&mut paths), Some((String::from("zy"), 100)));
        assert_eq!(next(&mut paths), None);
    }

    #[test]
    fn more_jump_targets_than_keys_2() {
        let mut paths = TrieNode::build(b"xyz", iota(5)).generate().into_iter();
        assert_eq!(next(&mut paths), Some((String::from("x"), 0)));
        assert_eq!(next(&mut paths), Some((String::from("y"), 1)));
        assert_eq!(next(&mut paths), Some((String::from("zx"), 2)));
        assert_eq!(next(&mut paths), Some((String::from("zy"), 3)));
        assert_eq!(next(&mut paths), Some((String::from("zz"), 4)));
        assert_eq!(next(&mut paths), None);
    }

    #[test]
    fn more_jump_targets_than_keys_3() {
        let mut paths = TrieNode::build(b"xyz", iota(6)).generate().into_iter();
        assert_eq!(next(&mut paths), Some((String::from("x"), 0)));
        assert_eq!(next(&mut paths), Some((String::from("yx"), 1)));
        assert_eq!(next(&mut paths), Some((String::from("yy"), 2)));
        assert_eq!(next(&mut paths), Some((String::from("zx"), 3)));
        assert_eq!(next(&mut paths), Some((String::from("zy"), 4)));
        assert_eq!(next(&mut paths), Some((String::from("zz"), 5)));
        assert_eq!(next(&mut paths), None);
    }

    #[test]
    fn more_jump_targets_than_keys_4() {
        let mut paths = TrieNode::build(b"xyz", iota(7)).generate().into_iter();
        assert_eq!(next(&mut paths), Some((String::from("x"), 0)));
        assert_eq!(next(&mut paths), Some((String::from("yx"), 1)));
        assert_eq!(next(&mut paths), Some((String::from("yy"), 2)));
        assert_eq!(next(&mut paths), Some((String::from("yz"), 3)));
        assert_eq!(next(&mut paths), Some((String::from("zx"), 4)));
        assert_eq!(next(&mut paths), Some((String::from("zy"), 5)));
        assert_eq!(next(&mut paths), Some((String::from("zz"), 6)));
        assert_eq!(next(&mut paths), None);
    }

    #[test]
    fn more_jump_targets_than_keys_5() {
        let mut paths = TrieNode::build(b"xyz", iota(8)).generate().into_iter();
        assert_eq!(next(&mut paths), Some((String::from("xx"), 0)));
        assert_eq!(next(&mut paths), Some((String::from("xy"), 1)));
        assert_eq!(next(&mut paths), Some((String::from("yx"), 2)));
        assert_eq!(next(&mut paths), Some((String::from("yy"), 3)));
        assert_eq!(next(&mut paths), Some((String::from("yz"), 4)));
        assert_eq!(next(&mut paths), Some((String::from("zx"), 5)));
        assert_eq!(next(&mut paths), Some((String::from("zy"), 6)));
        assert_eq!(next(&mut paths), Some((String::from("zz"), 7)));
        assert_eq!(next(&mut paths), None);
    }

    #[test]
    fn more_jump_targets_than_keys_6() {
        let mut paths = TrieNode::build(b"xyz", iota(9)).generate().into_iter();
        assert_eq!(next(&mut paths), Some((String::from("xx"), 0)));
        assert_eq!(next(&mut paths), Some((String::from("xy"), 1)));
        assert_eq!(next(&mut paths), Some((String::from("xz"), 2)));
        assert_eq!(next(&mut paths), Some((String::from("yx"), 3)));
        assert_eq!(next(&mut paths), Some((String::from("yy"), 4)));
        assert_eq!(next(&mut paths), Some((String::from("yz"), 5)));
        assert_eq!(next(&mut paths), Some((String::from("zx"), 6)));
        assert_eq!(next(&mut paths), Some((String::from("zy"), 7)));
        assert_eq!(next(&mut paths), Some((String::from("zz"), 8)));
        assert_eq!(next(&mut paths), None);
    }

    #[test]
    fn more_jump_targets_than_keys_7() {
        let root = TrieNode::build(b"xyz", iota(10));
        let mut paths = root.generate().into_iter();
        assert_eq!(next(&mut paths), Some((String::from("xx"), 0)));
        assert_eq!(next(&mut paths), Some((String::from("xy"), 1)));
        assert_eq!(next(&mut paths), Some((String::from("xz"), 2)));
        assert_eq!(next(&mut paths), Some((String::from("yx"), 3)));
        assert_eq!(next(&mut paths), Some((String::from("yy"), 4)));
        assert_eq!(next(&mut paths), Some((String::from("yz"), 5)));
        assert_eq!(next(&mut paths), Some((String::from("zx"), 6)));
        assert_eq!(next(&mut paths), Some((String::from("zy"), 7)));
        assert_eq!(next(&mut paths), Some((String::from("zzx"), 8)));
        assert_eq!(next(&mut paths), Some((String::from("zzy"), 9)));
        assert_eq!(next(&mut paths), None);

        let node = root.choose(b'z').unwrap();
        let mut paths = node.generate().into_iter();
        assert_eq!(next(&mut paths), Some((String::from("x"), 6)));
        assert_eq!(next(&mut paths), Some((String::from("y"), 7)));
        assert_eq!(next(&mut paths), Some((String::from("zx"), 8)));
        assert_eq!(next(&mut paths), Some((String::from("zy"), 9)));
        assert_eq!(next(&mut paths), None);
    }
}
