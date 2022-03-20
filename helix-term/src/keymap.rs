pub use crate::commands::{FallbackCommand, MappableCommand};
use crate::config::Config;
use helix_core::hashmap;
use helix_view::{document::Mode, info::Info, input::KeyEvent};
use serde::Deserialize;
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    ops::Deref,
};

#[macro_export]
macro_rules! key {
    ($key:ident) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::$key,
            modifiers: ::helix_view::keyboard::KeyModifiers::NONE,
        }
    };
    ($($ch:tt)*) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::Char($($ch)*),
            modifiers: ::helix_view::keyboard::KeyModifiers::NONE,
        }
    };
}

#[macro_export]
macro_rules! shift {
    ($key:ident) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::$key,
            modifiers: ::helix_view::keyboard::KeyModifiers::SHIFT,
        }
    };
    ($($ch:tt)*) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::Char($($ch)*),
            modifiers: ::helix_view::keyboard::KeyModifiers::SHIFT,
        }
    };
}

#[macro_export]
macro_rules! ctrl {
    ($key:ident) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::$key,
            modifiers: ::helix_view::keyboard::KeyModifiers::CONTROL,
        }
    };
    ($($ch:tt)*) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::Char($($ch)*),
            modifiers: ::helix_view::keyboard::KeyModifiers::CONTROL,
        }
    };
}

#[macro_export]
macro_rules! alt {
    ($key:ident) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::$key,
            modifiers: ::helix_view::keyboard::KeyModifiers::ALT,
        }
    };
    ($($ch:tt)*) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::Char($($ch)*),
            modifiers: ::helix_view::keyboard::KeyModifiers::ALT,
        }
    };
}

/// Macro for defining the root of a `Keymap` object. Example:
///
/// ```
/// # use helix_core::hashmap;
/// # use helix_term::keymap;
/// # use helix_term::keymap::Keymap;
/// let normal_mode = keymap!({ "Normal mode"
///     "i" => insert_mode,
///     "g" => { "Goto"
///         "g" => goto_file_start,
///         "e" => goto_file_end,
///     },
///     "j" | "down" => move_line_down,
/// });
/// let keymap = Keymap::new(normal_mode);
/// ```
#[macro_export]
macro_rules! keymap {
    (@trie $cmd:ident) => {
        $crate::keymap::KeyTrie::Leaf($crate::commands::MappableCommand::$cmd)
    };

    (@trie
        {
            $label:literal
            $(sticky=$sticky:literal)?
            $($($key:literal)|+ => $value:tt,)+
            $(_ => $fallback:ident,)?
        }
    ) => {
        keymap!({
            $label
            $(sticky=$sticky)?
            $($($key)|+ => $value,)+
            $(_ => $fallback,)?
        })
    };

    (@trie [$($cmd:ident),* $(,)?]) => {
        $crate::keymap::KeyTrie::Sequence(vec![$($crate::commands::Command::$cmd),*])
    };

    (
        {
            $label:literal
            $(sticky=$sticky:literal)?
            $($($key:literal)|+ => $value:tt,)+
            $(_ => $fallback:ident,)?
        }
    ) => {
        // modified from the hashmap! macro
        {
            let _cap = hashmap!(@count $($($key),+),*);
            let mut _map = ::std::collections::HashMap::with_capacity(_cap);
            let mut _order = ::std::vec::Vec::with_capacity(_cap);
            $(
                $(
                    let _key = $key.parse::<::helix_view::input::KeyEvent>().unwrap();
                    let _duplicate = _map.insert(
                        _key,
                        keymap!(@trie $value)
                    );
                    assert!(_duplicate.is_none(), "Duplicate key found: {:?}", _duplicate.unwrap());
                    _order.push(_key);
                )+
            )*
            let mut _node = $crate::keymap::KeyTrieNode::new($label, _map, _order);
            $( _node.is_sticky = $sticky; )?
            $( _node.fallback = Some($crate::commands::FallbackCommand::$fallback); )?
            $crate::keymap::KeyTrie::Node(_node)
        }
    };
}

#[derive(Debug, Clone)]
pub struct KeyTrieNode {
    /// A label for keys coming under this node, like "Goto mode"
    name: String,
    map: HashMap<KeyEvent, KeyTrie>,
    // TODO: Come up with a serialized representation of this
    fallback: Option<FallbackCommand>,
    order: Vec<KeyEvent>,
    pub is_sticky: bool,
}

impl<'de> Deserialize<'de> for KeyTrieNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = HashMap::<KeyEvent, KeyTrie>::deserialize(deserializer)?;
        let order = map.keys().copied().collect::<Vec<_>>(); // NOTE: map.keys() has arbitrary order
        Ok(Self {
            map,
            order,
            ..Default::default()
        })
    }
}

impl KeyTrieNode {
    pub fn new(name: &str, map: HashMap<KeyEvent, KeyTrie>, order: Vec<KeyEvent>) -> Self {
        Self {
            name: name.to_string(),
            map,
            fallback: None,
            order,
            is_sticky: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn len(&self) -> usize {
        self.map.len() + self.fallback.is_some() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Merge another Node in. Leaves and subnodes from the other node replace
    /// corresponding keyevent in self, except when both other and self have
    /// subnodes for same key. In that case the merge is recursive.
    pub fn merge(&mut self, mut other: Self) {
        for (key, trie) in std::mem::take(&mut other.map) {
            if let Some(KeyTrie::Node(node)) = self.map.get_mut(&key) {
                if let KeyTrie::Node(other_node) = trie {
                    node.merge(other_node);
                    continue;
                }
            }
            self.map.insert(key, trie);
        }
        for &key in self.map.keys() {
            if !self.order.contains(&key) {
                self.order.push(key);
            }
        }
    }

    pub fn infobox(&self) -> Info {
        let mut body: Vec<(&str, BTreeSet<Option<KeyEvent>>)> = Vec::with_capacity(self.len());
        for (&key, trie) in &self.map {
            let desc = match trie {
                KeyTrie::Leaf(cmd) => {
                    if cmd.name() == "no_op" {
                        continue;
                    }
                    cmd.doc()
                }
                KeyTrie::Node(n) => n.name(),
                KeyTrie::Sequence(_) => "[Multiple commands]",
            };
            match body.iter().position(|(d, _)| d == &desc) {
                Some(pos) => {
                    body[pos].1.insert(Some(key));
                }
                None => body.push((desc, [Some(key)].into())),
            }
        }
        if let Some(cmd) = &self.fallback {
            let desc = cmd.doc();
            match body.iter().position(|(d, _)| d == &desc) {
                Some(pos) => {
                    body[pos].1.insert(None);
                }
                None => body.push((desc, [None].into())),
            }
        }
        body.sort_unstable_by_key(|(_, keys)| {
            match keys.iter().next().unwrap() {
                Some(key) => self.order.iter().position(|k| k == key).unwrap(),
                None => usize::MAX, // fallback goes at the end
            }
        });
        let prefix = format!("{} ", self.name());
        if body.iter().all(|(desc, _)| desc.starts_with(&prefix)) {
            for (desc, _) in &mut body {
                *desc = desc.strip_prefix(&prefix).unwrap();
            }
        }
        Info::from_keymap(self.name(), body)
    }
    /// Get a reference to the key trie node's order.
    pub fn order(&self) -> &[KeyEvent] {
        self.order.as_slice()
    }

    fn search(&self, keys: &'_ [KeyEvent]) -> KeyTrieResult<'_> {
        use KeyTrieResult::*;
        match keys {
            [] => Pending(self),
            [k] => match self.map.get(k) {
                Some(trie) => trie.search(&[]),
                None => self.fallback.as_ref().map_or(NotFound, MatchedFallback),
            },
            [k, rest @ ..] => match self.map.get(k) {
                Some(trie) => trie.search(rest),
                None => NotFound,
            },
        }
    }
}

impl Default for KeyTrieNode {
    fn default() -> Self {
        Self::new("", HashMap::new(), Vec::new())
    }
}

impl PartialEq for KeyTrieNode {
    fn eq(&self, other: &Self) -> bool {
        self.map == other.map && self.fallback == other.fallback
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum KeyTrie {
    Leaf(MappableCommand),
    Sequence(Vec<MappableCommand>),
    Node(KeyTrieNode),
}

impl KeyTrie {
    pub fn node(&self) -> Option<&KeyTrieNode> {
        match *self {
            KeyTrie::Node(ref node) => Some(node),
            KeyTrie::Leaf(_) | KeyTrie::Sequence(_) => None,
        }
    }

    pub fn node_mut(&mut self) -> Option<&mut KeyTrieNode> {
        match *self {
            KeyTrie::Node(ref mut node) => Some(node),
            KeyTrie::Leaf(_) | KeyTrie::Sequence(_) => None,
        }
    }

    /// Merge another KeyTrie in, assuming that this KeyTrie and the other
    /// are both Nodes. Panics otherwise.
    pub fn merge_nodes(&mut self, mut other: Self) {
        let node = std::mem::take(other.node_mut().unwrap());
        self.node_mut().unwrap().merge(node);
    }

    fn search(&self, keys: &'_ [KeyEvent]) -> KeyTrieResult<'_> {
        match self {
            KeyTrie::Leaf(cmd) if keys.is_empty() => KeyTrieResult::Matched(cmd),
            KeyTrie::Sequence(cmds) if keys.is_empty() => KeyTrieResult::MatchedSequence(cmds),
            KeyTrie::Node(node) => node.search(keys),
            _ => KeyTrieResult::NotFound,
        }
    }
}

#[derive(Debug, PartialEq)]
enum KeyTrieResult<'a> {
    Pending(&'a KeyTrieNode),
    Matched(&'a MappableCommand),
    MatchedFallback(&'a FallbackCommand),
    MatchedSequence(&'a [MappableCommand]),
    NotFound,
}

impl From<KeyTrieResult<'_>> for KeymapResult {
    fn from(r: KeyTrieResult<'_>) -> Self {
        match r {
            KeyTrieResult::Pending(node) => Self::Pending(node.clone()),
            KeyTrieResult::Matched(cmd) => Self::Matched(cmd.clone()),
            KeyTrieResult::MatchedFallback(cmd) => Self::MatchedFallback(cmd.clone()),
            KeyTrieResult::MatchedSequence(cmds) => Self::MatchedSequence(cmds.to_vec()),
            KeyTrieResult::NotFound => Self::NotFound,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeymapResult {
    /// Needs more keys to execute a command. Contains valid keys for next keystroke.
    Pending(KeyTrieNode),
    Matched(MappableCommand),
    /// Matched a fallback command, which uses the last key event.
    MatchedFallback(FallbackCommand),
    /// Matched a sequence of commands to execute.
    MatchedSequence(Vec<MappableCommand>),
    /// Key was not found in the root keymap
    NotFound,
    /// Key is invalid in combination with previous keys. Contains keys leading upto
    /// and including current (invalid) key.
    Cancelled(Vec<KeyEvent>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Binding {
    pub keys: Vec<KeyEvent>,
    pub is_fallback: bool,
}

impl Binding {
    pub fn new(keys: &[KeyEvent]) -> Self {
        Self {
            keys: keys.to_vec(),
            is_fallback: false,
        }
    }

    pub fn fallback(keys: &[KeyEvent]) -> Self {
        Self {
            keys: keys.to_vec(),
            is_fallback: true,
        }
    }
}

impl std::fmt::Display for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        debug_assert!(!self.keys.is_empty());
        self.keys[0].fmt(f)?;
        for key in &self.keys[1..] {
            write!(f, "+{key}")?;
        }
        if self.is_fallback {
            write!(f, "+…")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bindings {
    pub bindings: Vec<Binding>,
}

impl Bindings {
    pub fn add_binding(&mut self, keys: &[KeyEvent]) {
        self.bindings.push(Binding::new(keys));
    }

    pub fn add_fallback(&mut self, keys: &[KeyEvent]) {
        self.bindings.push(Binding::fallback(keys));
    }
}

impl std::fmt::Display for Bindings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.bindings.is_empty() {
            return Ok(());
        }
        self.bindings[0].fmt(f)?;
        for binding in &self.bindings[1..] {
            write!(f, ", {binding}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Keymap {
    /// Always a Node
    root: KeyTrie,
}

impl Keymap {
    pub fn new(root: KeyTrie) -> Self {
        Keymap { root }
    }

    pub fn reverse_map(&self) -> HashMap<String, Bindings> {
        // recursively visit all nodes in keymap
        fn map_node(
            cmd_map: &mut HashMap<String, Bindings>,
            node: &KeyTrie,
            keys: &mut Vec<KeyEvent>,
        ) {
            match node {
                KeyTrie::Leaf(cmd) => cmd_map
                    .entry(cmd.name().into())
                    .or_default()
                    .add_binding(keys),
                KeyTrie::Node(next) => {
                    for (key, trie) in &next.map {
                        keys.push(*key);
                        map_node(cmd_map, trie, keys);
                        keys.pop();
                    }
                    if let Some(fallback) = &next.fallback {
                        cmd_map
                            .entry(fallback.with_prompt.name().into())
                            .or_default()
                            .add_fallback(keys)
                    }
                }
                KeyTrie::Sequence(_) => {}
            };
        }

        let mut res = HashMap::new();
        map_node(&mut res, &self.root, &mut Vec::new());
        res
    }

    pub fn root(&self) -> &KeyTrie {
        &self.root
    }

    pub fn merge(&mut self, other: Self) {
        self.root.merge_nodes(other.root);
    }
}

impl Deref for Keymap {
    type Target = KeyTrieNode;

    fn deref(&self) -> &Self::Target {
        self.root.node().unwrap()
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new(KeyTrie::Node(KeyTrieNode::default()))
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Keymaps {
    #[serde(flatten)]
    pub map: HashMap<Mode, Keymap>,

    /// Stores pending keys waiting for the next key. This is relative to a
    /// sticky node if one is in use.
    #[serde(skip)]
    state: Vec<KeyEvent>,

    /// Stores the sticky node if one is activated.
    #[serde(skip)]
    pub sticky: Option<KeyTrieNode>,
}

impl Keymaps {
    pub fn new(map: HashMap<Mode, Keymap>) -> Self {
        Self {
            map,
            state: Vec::new(),
            sticky: None,
        }
    }

    /// Returns list of keys waiting to be disambiguated in current mode.
    pub fn pending(&self) -> &[KeyEvent] {
        &self.state
    }

    pub fn sticky(&self) -> Option<&KeyTrieNode> {
        self.sticky.as_ref()
    }

    /// Lookup `key` in the keymap to try and find a command to execute. Escape
    /// key cancels pending keystrokes. If there are no pending keystrokes but a
    /// sticky node is in use, it will be cleared.
    pub fn get(&mut self, mode: Mode, key: KeyEvent) -> KeymapResult {
        // TODO: remove the sticky part and look up manually
        let keymap = &self.map[&mode];

        if key!(Esc) == key {
            if !self.state.is_empty() {
                // Note that Esc is not included here
                return KeymapResult::Cancelled(self.state.drain(..).collect());
            }
            self.sticky = None;
        }

        let first = self.state.get(0).unwrap_or(&key);
        let trie_node = match self.sticky {
            Some(ref trie) => Cow::Owned(KeyTrie::Node(trie.clone())),
            None => Cow::Borrowed(&keymap.root),
        };

        let trie = match trie_node.search(&[*first]) {
            KeyTrieResult::Pending(node) => node,
            res => return res.into(),
        };

        self.state.push(key);
        let res = trie.search(&self.state[1..]);
        match res {
            KeyTrieResult::Pending(node) => {
                if node.is_sticky {
                    self.state.clear();
                    self.sticky = Some(node.clone());
                }
            }
            KeyTrieResult::NotFound => {
                let keys = std::mem::take(&mut self.state);
                return KeymapResult::Cancelled(keys);
            }
            _ => self.state.clear(),
        }
        res.into()
    }
}

impl Default for Keymaps {
    fn default() -> Self {
        let normal = keymap!({ "Normal mode"
            "h" | "left" => move_char_left,
            "j" | "down" => move_line_down,
            "k" | "up" => move_line_up,
            "l" | "right" => move_char_right,

            "t" => find_till_char,
            "f" => find_next_char,
            "T" => till_prev_char,
            "F" => find_prev_char,
            "r" => replace,
            "R" => replace_with_yanked,
            "A-." =>  repeat_last_motion,

            "~" => switch_case,
            "`" => switch_to_lowercase,
            "A-`" => switch_to_uppercase,

            "home" => goto_line_start,
            "end" => goto_line_end,

            "w" => move_next_word_start,
            "b" => move_prev_word_start,
            "e" => move_next_word_end,

            "W" => move_next_long_word_start,
            "B" => move_prev_long_word_start,
            "E" => move_next_long_word_end,

            "v" => select_mode,
            "G" => goto_line,
            "g" => { "Goto"
                "g" => goto_file_start,
                "e" => goto_last_line,
                "f" => goto_file,
                "h" => goto_line_start,
                "l" => goto_line_end,
                "s" => goto_first_nonwhitespace,
                "d" => goto_definition,
                "y" => goto_type_definition,
                "r" => goto_reference,
                "i" => goto_implementation,
                "t" => goto_window_top,
                "c" => goto_window_center,
                "b" => goto_window_bottom,
                "a" => goto_last_accessed_file,
                "m" => goto_last_modified_file,
                "n" => goto_next_buffer,
                "p" => goto_previous_buffer,
                "." => goto_last_modification,
            },
            ":" => command_mode,

            "i" => insert_mode,
            "I" => prepend_to_line,
            "a" => append_mode,
            "A" => append_to_line,
            "o" => open_below,
            "O" => open_above,

            "d" => delete_selection,
            "A-d" => delete_selection_noyank,
            "c" => change_selection,
            "A-c" => change_selection_noyank,

            "C" => copy_selection_on_next_line,
            "A-C" => copy_selection_on_prev_line,


            "s" => select_regex,
            "A-s" => split_selection_on_newline,
            "S" => split_selection,
            ";" => collapse_selection,
            "A-;" => flip_selections,
            "A-k" | "A-up" => expand_selection,
            "A-j" | "A-down" => shrink_selection,
            "A-h" | "A-left" => select_prev_sibling,
            "A-l" | "A-right" => select_next_sibling,

            "%" => select_all,
            "x" => extend_line,
            "X" => extend_to_line_bounds,
            // crop_to_whole_line

            "m" => { "Match"
                "m" => match_brackets,
                "s" => surround_add,
                "r" => surround_replace,
                "d" => surround_delete,
                "a" => { "Select around"
                    "w" => select_around_word,
                    "W" => select_around_long_word,
                    "c" => select_around_class,
                    "f" => select_around_function,
                    "a" => select_around_parameter,
                    "m" => select_around_cursor_pair,
                    _ => select_around_pair,
                },
                "i" => { "Select inside"
                    "w" => select_inside_word,
                    "W" => select_inside_long_word,
                    "c" => select_inside_class,
                    "f" => select_inside_function,
                    "a" => select_inside_parameter,
                    "m" => select_inside_cursor_pair,
                    _ => select_inside_pair,
                },
            },
            "[" => { "Left bracket"
                "d" => goto_prev_diag,
                "D" => goto_first_diag,
                "f" => goto_prev_function,
                "c" => goto_prev_class,
                "a" => goto_prev_parameter,
                "o" => goto_prev_comment,
                "space" => add_newline_above,
            },
            "]" => { "Right bracket"
                "d" => goto_next_diag,
                "D" => goto_last_diag,
                "f" => goto_next_function,
                "c" => goto_next_class,
                "a" => goto_next_parameter,
                "o" => goto_next_comment,
                "space" => add_newline_below,
            },

            "/" => search,
            "?" => rsearch,
            "n" => search_next,
            "N" => search_prev,
            "*" => search_selection,

            "u" => undo,
            "U" => redo,
            "A-u" => earlier,
            "A-U" => later,

            "y" => yank,
            // yank_all
            "p" => paste_after,
            // paste_all
            "P" => paste_before,

            "Q" => record_macro,
            "q" => replay_macro,

            ">" => indent,
            "<" => unindent,
            "=" => format_selections,
            "J" => join_selections,
            "K" => keep_selections,
            "A-K" => remove_selections,

            "," => keep_primary_selection,
            "A-," => remove_primary_selection,

            // "q" => record_macro,
            // "Q" => replay_macro,

            "&" => align_selections,
            "_" => trim_selections,

            "(" => rotate_selections_backward,
            ")" => rotate_selections_forward,
            "A-(" => rotate_selection_contents_backward,
            "A-)" => rotate_selection_contents_forward,

            "A-:" => ensure_selections_forward,

            "esc" => normal_mode,
            "C-b" | "pageup" => page_up,
            "C-f" | "pagedown" => page_down,
            "C-u" => half_page_up,
            "C-d" => half_page_down,

            "C-w" => { "Window"
                "C-w" | "w" => rotate_view,
                "C-s" | "s" => hsplit,
                "C-v" | "v" => vsplit,
                "f" => goto_file_hsplit,
                "F" => goto_file_vsplit,
                "C-q" | "q" => wclose,
                "C-o" | "o" => wonly,
                "C-h" | "h" | "left" => jump_view_left,
                "C-j" | "j" | "down" => jump_view_down,
                "C-k" | "k" | "up" => jump_view_up,
                "C-l" | "l" | "right" => jump_view_right,
                "n" => { "New split scratch buffer"
                    "C-s" | "s" => hsplit_new,
                    "C-v" | "v" => vsplit_new,
                },
            },

            // move under <space>c
            "C-c" => toggle_comments,

            // z family for save/restore/combine from/to sels from register

            "tab" => jump_forward, // tab == <C-i>
            "C-o" => jump_backward,
            "C-s" => save_selection,

            "space" => { "Space"
                "f" => file_picker,
                "b" => buffer_picker,
                "s" => symbol_picker,
                "S" => workspace_symbol_picker,
                "a" => code_action,
                "'" => last_picker,
                "d" => { "Debug (experimental)" sticky=true
                    "l" => dap_launch,
                    "b" => dap_toggle_breakpoint,
                    "c" => dap_continue,
                    "h" => dap_pause,
                    "i" => dap_step_in,
                    "o" => dap_step_out,
                    "n" => dap_next,
                    "v" => dap_variables,
                    "t" => dap_terminate,
                    "C-c" => dap_edit_condition,
                    "C-l" => dap_edit_log,
                    "s" => { "Switch"
                        "t" => dap_switch_thread,
                        "f" => dap_switch_stack_frame,
                        // sl, sb
                    },
                    "e" => dap_enable_exceptions,
                    "E" => dap_disable_exceptions,
                },
                "w" => { "Window"
                    "C-w" | "w" => rotate_view,
                    "C-s" | "s" => hsplit,
                    "C-v" | "v" => vsplit,
                    "f" => goto_file_hsplit,
                    "F" => goto_file_vsplit,
                    "C-q" | "q" => wclose,
                    "C-o" | "o" => wonly,
                    "C-h" | "h" | "left" => jump_view_left,
                    "C-j" | "j" | "down" => jump_view_down,
                    "C-k" | "k" | "up" => jump_view_up,
                    "C-l" | "l" | "right" => jump_view_right,
                    "n" => { "New split scratch buffer"
                        "C-s" | "s" => hsplit_new,
                        "C-v" | "v" => vsplit_new,
                    },
                },
                "y" => yank_joined_to_clipboard,
                "Y" => yank_main_selection_to_clipboard,
                "p" => paste_clipboard_after,
                "P" => paste_clipboard_before,
                "R" => replace_selections_with_clipboard,
                "/" => global_search,
                "k" => hover,
                "r" => rename_symbol,
                "?" => command_palette,
            },
            "z" => { "View"
                "z" | "c" => align_view_center,
                "t" => align_view_top,
                "b" => align_view_bottom,
                "m" => align_view_middle,
                "k" | "up" => scroll_up,
                "j" | "down" => scroll_down,
                "C-b" | "pageup" => page_up,
                "C-f" | "pagedown" => page_down,
                "C-u" => half_page_up,
                "C-d" => half_page_down,
            },
            "Z" => { "View" sticky=true
                "z" | "c" => align_view_center,
                "t" => align_view_top,
                "b" => align_view_bottom,
                "m" => align_view_middle,
                "k" | "up" => scroll_up,
                "j" | "down" => scroll_down,
                "C-b" | "pageup" => page_up,
                "C-f" | "pagedown" => page_down,
                "C-u" => half_page_up,
                "C-d" => half_page_down,
            },

            "\"" => select_register,
            "|" => shell_pipe,
            "A-|" => shell_pipe_to,
            "!" => shell_insert_output,
            "A-!" => shell_append_output,
            "$" => shell_keep_pipe,
            "C-z" => suspend,

            "C-a" => increment,
            "C-x" => decrement,
        });
        let mut select = normal.clone();
        select.merge_nodes(keymap!({ "Select mode"
            "h" | "left" => extend_char_left,
            "j" | "down" => extend_line_down,
            "k" | "up" => extend_line_up,
            "l" | "right" => extend_char_right,

            "w" => extend_next_word_start,
            "b" => extend_prev_word_start,
            "e" => extend_next_word_end,
            "W" => extend_next_long_word_start,
            "B" => extend_prev_long_word_start,
            "E" => extend_next_long_word_end,

            "n" => extend_search_next,
            "N" => extend_search_prev,

            "t" => extend_till_char,
            "f" => extend_next_char,
            "T" => extend_till_prev_char,
            "F" => extend_prev_char,

            "home" => extend_to_line_start,
            "end" => extend_to_line_end,
            "esc" => exit_select_mode,

            "v" => normal_mode,
        }));
        let insert = keymap!({ "Insert mode"
            "esc" => normal_mode,

            "backspace" => delete_char_backward,
            "C-h" => delete_char_backward,
            "del" => delete_char_forward,
            "C-d" => delete_char_forward,
            "ret" => insert_newline,
            "C-j" => insert_newline,
            "tab" => insert_tab,
            "C-w" => delete_word_backward,
            "A-backspace" => delete_word_backward,
            "A-d" => delete_word_forward,

            "left" => move_char_left,
            "C-b" => move_char_left,
            "down" => move_line_down,
            "C-n" => move_line_down,
            "up" => move_line_up,
            "C-p" => move_line_up,
            "right" => move_char_right,
            "C-f" => move_char_right,
            "A-b" => move_prev_word_end,
            "A-left" => move_prev_word_end,
            "A-f" => move_next_word_start,
            "A-right" => move_next_word_start,
            "A-<" => goto_file_start,
            "A->" => goto_file_end,
            "pageup" => page_up,
            "pagedown" => page_down,
            "home" => goto_line_start,
            "C-a" => goto_line_start,
            "end" => goto_line_end_newline,
            "C-e" => goto_line_end_newline,

            "C-k" => kill_to_line_end,
            "C-u" => kill_to_line_start,

            "C-x" => completion,
            "C-r" => insert_register,
        });
        Self::new(hashmap!(
            Mode::Normal => Keymap::new(normal),
            Mode::Select => Keymap::new(select),
            Mode::Insert => Keymap::new(insert),
        ))
    }
}

/// Merge default config keys with user overwritten keys for custom user config.
pub fn merge_keys(mut config: Config) -> Config {
    let mut delta = std::mem::take(&mut config.keys);
    for (mode, keys) in &mut config.keys.map {
        keys.merge(delta.map.remove(mode).unwrap_or_default())
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn duplicate_keys_should_panic() {
        keymap!({ "Normal mode"
            "i" => normal_mode,
            "i" => goto_definition,
        });
    }

    #[test]
    fn check_duplicate_keys_in_default_keymap() {
        // will panic on duplicate keys, assumes that `Keymaps` uses keymap! macro
        Keymaps::default();
    }

    #[test]
    fn merge_partial_keys() {
        let config = Config {
            keys: Keymaps::new(hashmap! {
                Mode::Normal => Keymap::new(
                    keymap!({ "Normal mode"
                        "i" => normal_mode,
                        "无" => insert_mode,
                        "z" => jump_backward,
                        "g" => { "Merge into goto mode"
                            "$" => goto_line_end,
                            "g" => delete_char_forward,
                        },
                    })
                )
            }),
            ..Default::default()
        };
        let mut merged_config = merge_keys(config.clone());
        assert_ne!(config, merged_config);

        let keymap = &mut merged_config.keys;
        assert_eq!(
            keymap.get(Mode::Normal, key!('i')),
            KeymapResult::Matched(MappableCommand::normal_mode),
            "Leaf should replace leaf"
        );
        assert_eq!(
            keymap.get(Mode::Normal, key!('无')),
            KeymapResult::Matched(MappableCommand::insert_mode),
            "New leaf should be present in merged keymap"
        );
        // Assumes that z is a node in the default keymap
        assert_eq!(
            keymap.get(Mode::Normal, key!('z')),
            KeymapResult::Matched(MappableCommand::jump_backward),
            "Leaf should replace node"
        );

        let keymap = merged_config.keys.map.get_mut(&Mode::Normal).unwrap();
        // Assumes that `g` is a node in default keymap
        assert_eq!(
            keymap.root().search(&[key!('g'), key!('$')]),
            KeyTrieResult::Matched(&MappableCommand::goto_line_end),
            "Leaf should be present in merged subnode"
        );
        // Assumes that `gg` is in default keymap
        assert_eq!(
            keymap.root().search(&[key!('g'), key!('g')]),
            KeyTrieResult::Matched(&MappableCommand::delete_char_forward),
            "Leaf should replace old leaf in merged subnode"
        );
        // Assumes that `ge` is in default keymap
        assert_eq!(
            keymap.root().search(&[key!('g'), key!('e')]),
            KeyTrieResult::Matched(&MappableCommand::goto_last_line),
            "Old leaves in subnode should be present in merged node"
        );

        assert!(merged_config.keys.map.get(&Mode::Normal).unwrap().len() > 1);
        assert!(merged_config.keys.map.get(&Mode::Insert).unwrap().len() > 0);
    }

    #[test]
    fn order_should_be_set() {
        let config = Config {
            keys: Keymaps::new(hashmap! {
                Mode::Normal => Keymap::new(
                    keymap!({ "Normal mode"
                        "space" => { ""
                            "s" => { ""
                                "v" => vsplit,
                                "c" => hsplit,
                            },
                        },
                    })
                )
            }),
            ..Default::default()
        };
        let mut merged_config = merge_keys(config.clone());
        assert_ne!(config, merged_config);
        let keymap = merged_config.keys.map.get_mut(&Mode::Normal).unwrap();
        // Make sure mapping works
        assert_eq!(
            keymap.root().search(&[key!(' '), key!('s'), key!('v')]),
            KeyTrieResult::Matched(&MappableCommand::vsplit),
            "Leaf should be present in merged subnode"
        );
        let node = match keymap.root().search(&[key!(' ')]) {
            KeyTrieResult::Pending(n) => n,
            _ => panic!("Expected node"),
        };
        // Make sure an order was set during merge
        assert!(!node.order().is_empty())
    }

    #[test]
    fn aliased_modes_are_same_in_default_keymap() {
        let keymaps = Keymaps::default();
        let root = keymaps.map.get(&Mode::Normal).unwrap().root();
        assert_eq!(
            root.search(&[key!(' '), key!('w')]),
            root.search(&["C-w".parse::<KeyEvent>().unwrap()]),
            "Mismatch for window mode on `Space-w` and `Ctrl-w`"
        );
        assert_eq!(
            root.search(&[key!('z')]),
            root.search(&[key!('Z')]),
            "Mismatch for view mode on `z` and `Z`"
        );
    }

    #[test]
    fn reverse_map() {
        let normal_mode = keymap!({ "Normal mode"
            "i" => insert_mode,
            "g" => { "Goto"
                "g" => goto_file_start,
                "e" => goto_file_end,
            },
            "j" | "k" => move_line_down,
            "m" => { "Match"
                "m" => match_brackets,
                "a" => { "Select around"
                    "w" => select_around_word,
                    "W" => select_around_long_word,
                    _ => select_around_pair,
                },
            },
        });
        let keymap = Keymap::new(normal_mode);
        let mut reverse_map = keymap.reverse_map();

        // sort keybindings in order to have consistent tests
        // HashMaps can be compared but we can still get different ordering of bindings
        // for commands that have multiple bindings assigned
        for v in reverse_map.values_mut() {
            v.bindings.sort()
        }

        fn make_binding(pair: &(&str, bool)) -> Binding {
            Binding {
                keys: pair.0.chars().map(|k| key!(k,)).collect(),
                is_fallback: pair.1,
            }
        }

        let expected_map = [
            ("insert_mode", &[("i", false)][..]),
            ("goto_file_start", &[("gg", false)]),
            ("goto_file_end", &[("ge", false)]),
            ("move_line_down", &[("j", false), ("k", false)]),
            ("match_brackets", &[("mm", false)]),
            ("select_around_word", &[("maw", false)]),
            ("select_around_long_word", &[("maW", false)]),
            ("prompt_and_select_around_pair", &[("ma", true)]),
        ]
        .into_iter()
        .map(|(command, bindings)| {
            let mut bindings = bindings.iter().map(make_binding).collect::<Vec<_>>();
            bindings.sort();
            (command.to_owned(), Bindings { bindings })
        })
        .collect::<HashMap<String, Bindings>>();

        assert_eq!(reverse_map, expected_map, "Mismatch");
    }
}
