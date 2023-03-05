use super::keytrienode::KeyTrieNode;
use helix_view::{info::Info, input::KeyEvent};
use serde::Deserialize;
use std::{cmp::Ordering, collections::HashMap};

/// Edges of the trie are KeyEvents and the nodes are descrbibed by KeyTrieNode
#[derive(Debug, Clone)]
pub struct KeyTrie {
    description: String,
    /// Used for pre-defined order in infoboxes, values represent the index of the key tries children.
    child_order: HashMap<KeyEvent, usize>,
    children: Vec<KeyTrieNode>,
    pub is_sticky: bool,
    /// Used to respect pre-defined stickyness.
    pub explicitly_set_sticky: bool,
    /// Used to override pre-defined descriptions.
    pub explicitly_set_description: bool,
}

impl KeyTrie {
    pub fn new(
        description: &str,
        child_order: HashMap<KeyEvent, usize>,
        children: Vec<KeyTrieNode>,
    ) -> Self {
        Self {
            description: description.to_string(),
            child_order,
            children,
            is_sticky: false,
            explicitly_set_sticky: false,
            explicitly_set_description: false,
        }
    }

    pub fn get_child_order(&self) -> &HashMap<KeyEvent, usize> {
        &self.child_order
    }

    pub fn get_children(&self) -> &Vec<KeyTrieNode> {
        &self.children
    }

    pub fn get_description(&self) -> &str {
        &self.description
    }

    // None symbolizes NotFound
    pub fn traverse(&self, key_events: &[KeyEvent]) -> Option<KeyTrieNode> {
        return _traverse(self, key_events, 0);

        fn _traverse(
            keytrie: &KeyTrie,
            key_events: &[KeyEvent],
            mut depth: usize,
        ) -> Option<KeyTrieNode> {
            if depth == key_events.len() {
                return Some(KeyTrieNode::KeyTrie(keytrie.clone()));
            } else if let Some(found_index) = keytrie.child_order.get(&key_events[depth]) {
                match &keytrie.children[*found_index] {
                    KeyTrieNode::KeyTrie(sub_keytrie) => {
                        depth += 1;
                        return _traverse(sub_keytrie, key_events, depth);
                    }
                    _found_child => return Some(_found_child.clone()),
                }
            }
            None
        }
    }

    /// Other takes precedent.
    pub fn merge_keytrie(&mut self, other_keytrie: Self) {
        if other_keytrie.explicitly_set_sticky {
            self.is_sticky = other_keytrie.is_sticky;
        }

        if other_keytrie.explicitly_set_description {
            self.description = other_keytrie.description.clone();
        }

        for (other_key_event, other_index) in other_keytrie.get_child_order() {
            let other_child_keytrie_node = &other_keytrie.get_children()[*other_index];
            if let Some(existing_index) = self.child_order.get(other_key_event) {
                if let KeyTrieNode::KeyTrie(ref mut self_clashing_child_key_trie) =
                    self.children[*existing_index]
                {
                    if let KeyTrieNode::KeyTrie(other_child_keytrie) = other_child_keytrie_node {
                        self_clashing_child_key_trie.merge_keytrie(other_child_keytrie.clone());
                        continue;
                    }
                }
                self.children[*existing_index] = other_child_keytrie_node.clone();
            } else {
                self.child_order
                    .insert(*other_key_event, self.children.len());
                self.children.push(other_child_keytrie_node.clone());
            }
        }
    }

    // IMPROVEMENT: cache contents and update cache only when config is updated
    /// Open an info box for a given KeyTrie
    /// Shows the children as possible KeyEvents with thier associated description.
    pub fn infobox(&self, sort_infobox: bool) -> Info {
        let mut body: InfoBoxBody = Vec::with_capacity(self.children.len());
        let mut key_event_order = Vec::with_capacity(self.children.len());
        // child_order and children is of same length
        #[allow(clippy::uninit_vec)]
        unsafe {
            key_event_order.set_len(self.children.len());
        }
        for (key_event, index) in &self.child_order {
            key_event_order[*index] = key_event;
        }

        for (index, key_trie) in self.children.iter().enumerate() {
            let description: String = match key_trie {
                KeyTrieNode::MappableCommand(ref command) => {
                    if command.name() == "no_op" {
                        continue;
                    }
                    command.get_description().to_string()
                }
                KeyTrieNode::CommandSequence(command_sequence) => {
                    if let Some(custom_description) = command_sequence.get_description() {
                        custom_description.to_string()
                    } else {
                        command_sequence
                            .get_commands()
                            .iter()
                            .map(|command| command.name().to_string())
                            .collect::<Vec<_>>()
                            .join(" → ")
                            .clone()
                    }
                }
                KeyTrieNode::KeyTrie(key_trie) => key_trie.description.clone(),
            };
            let key_event = key_event_order[index];
            match body
                .iter()
                .position(|(_, existing_description)| &description == existing_description)
            {
                Some(position) => body[position].0.push(key_event.to_string()),
                None => body.push((vec![key_event.to_string()], description)),
            }
        }

        // TODO: Add "A-" acknowledgement?
        // Shortest keyevent (as string) appears first, unless it's a "C-" KeyEvent
        // Those events will always be placed after the one letter KeyEvent
        for (key_events, _) in body.iter_mut() {
            key_events.sort_unstable_by(|a, b| {
                if a.len() == 1 {
                    return Ordering::Less;
                }
                if b.len() > a.len() && b.starts_with("C-") {
                    return Ordering::Greater;
                }
                a.len().cmp(&b.len())
            });
        }

        if sort_infobox {
            body = keyevent_sort_infobox(body);
        }

        let mut stringified_key_events_body = Vec::with_capacity(body.len());
        for (key_events, description) in body {
            stringified_key_events_body.push((key_events.join(", "), description));
        }

        Info::new(&self.description, &stringified_key_events_body)
    }
}

impl Default for KeyTrie {
    fn default() -> Self {
        Self::new("", HashMap::new(), Vec::new())
    }
}

impl PartialEq for KeyTrie {
    fn eq(&self, other: &Self) -> bool {
        self.children == other.children
    }
}

impl<'de> Deserialize<'de> for KeyTrie {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // NOTE: no assumption of pre-defined order in config
        let child_collection = HashMap::<KeyEvent, KeyTrieNode>::deserialize(deserializer)?;
        let mut child_order = HashMap::<KeyEvent, usize>::new();
        let mut children = Vec::new();
        for (key_event, keytrie_node) in child_collection {
            child_order.insert(key_event, children.len());
            children.push(keytrie_node);
        }

        Ok(Self {
            child_order,
            children,
            ..Default::default()
        })
    }
}

/// (KeyEvents as strings, Description)
type InfoBoxRow = (Vec<String>, String);
type InfoBoxBody = Vec<InfoBoxRow>;
/// Sorts by `ModifierKeyCode`, then by each `KeyCode` category, then by each `KeyEvent`.
/// KeyCode::Char sorting is special in that lower-case and upper-case equivalents are
/// placed together, and alphas are placed before the rest.
fn keyevent_sort_infobox(body: InfoBoxBody) -> InfoBoxBody {
    use helix_view::keyboard::{KeyCode, KeyModifiers, MediaKeyCode};
    use std::collections::BTreeMap;
    use std::str::FromStr;

    let mut category_holder: BTreeMap<KeyModifiers, BTreeMap<KeyCode, Vec<InfoBoxRow>>> =
        BTreeMap::new();
    let mut sorted_body: InfoBoxBody = Vec::with_capacity(body.len());
    for infobox_row in body {
        let first_keyevent = KeyEvent::from_str(infobox_row.0[0].as_str()).unwrap();
        category_holder
            .entry(first_keyevent.modifiers)
            .or_insert_with(BTreeMap::new);

        // HACK: inserting by variant not by variant value.
        // KeyCode:: Char, F, and MediaKeys can have muiltiple values for the given variant
        // Hence the use of mock Variant values
        let keycode_category = match first_keyevent.code {
            KeyCode::Char(_) => KeyCode::Char('a'),
            KeyCode::F(_) => KeyCode::F(0),
            KeyCode::Media(_) => KeyCode::Media(MediaKeyCode::Play),
            other_keycode => other_keycode,
        };

        let modifier_category = category_holder
            .get_mut(&first_keyevent.modifiers)
            .expect("keycode category existence should be checked.");
        modifier_category
            .entry(keycode_category)
            .or_insert_with(Vec::new);
        modifier_category
            .get_mut(&keycode_category)
            .expect("key existence should be checked")
            .push(infobox_row);
    }

    for (_, keycode_categories) in category_holder {
        for (keycode_category, mut infobox_rows) in keycode_categories {
            if infobox_rows.len() > 1 {
                match keycode_category {
                    KeyCode::Char(_) => {
                        infobox_rows.sort_unstable_by(|a, b| {
                            a.0[0].to_lowercase().cmp(&b.0[0].to_lowercase())
                        });

                        // Consistently place lowercase before uppercase of the same letter.
                        let mut x_index = 0;
                        let mut y_index = 1;
                        while y_index < infobox_rows.len() {
                            let x = &infobox_rows[x_index].0[0];
                            let y = &infobox_rows[y_index].0[0];
                            if x.to_lowercase() == y.to_lowercase() && x < y {
                                infobox_rows.swap(x_index, y_index);
                            }
                            x_index = y_index;
                            y_index += 1;
                        }

                        let mut alphas = Vec::new();
                        let mut misc = Vec::new();
                        for infobox_row in infobox_rows {
                            if ('a'..='z')
                                .map(|char| char.to_string())
                                .any(|alpha_char| *alpha_char == infobox_row.0[0].to_lowercase())
                            {
                                alphas.push(infobox_row);
                            } else {
                                misc.push(infobox_row);
                            }
                        }
                        infobox_rows = Vec::with_capacity(alphas.len() + misc.len());
                        for alpha_row in alphas {
                            infobox_rows.push(alpha_row);
                        }
                        for misc_row in misc {
                            infobox_rows.push(misc_row);
                        }
                    }
                    _ => {
                        infobox_rows.sort_unstable();
                    }
                }
            }
            sorted_body.append(infobox_rows.as_mut());
        }
    }
    sorted_body
}
