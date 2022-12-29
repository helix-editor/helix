use super::keytrie::KeyTrie;
use crate::commands::MappableCommand;
use helix_view::input::KeyEvent;
use std::collections::HashMap;
use serde::{Deserialize, de::Visitor};

/// Each variant includes a documentaion property.
/// For the MappableCommand and CommandSequence variants, the property is self explanatory.
/// For KeyTrie, the documentation is used for respective infobox titles,
/// or infobox KeyEvent descriptions that in themselves trigger the opening of another infobox.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyTrieNode {
    MappableCommand(MappableCommand),
    CommandSequence(Vec<MappableCommand>),
    KeyTrie(KeyTrie),
}

impl<'de> Deserialize<'de> for KeyTrieNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(KeyTrieNodeVisitor)
    }
}

struct KeyTrieNodeVisitor;

impl<'de> Visitor<'de> for KeyTrieNodeVisitor {
    type Value = KeyTrieNode;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a KeyTrieNode")
    }

    fn visit_str<E>(self, command: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        command
            .parse::<MappableCommand>()
            .map(KeyTrieNode::MappableCommand)
            .map_err(E::custom)
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: serde::de::SeqAccess<'de>,
    {
        let mut commands = Vec::new();
        while let Some(command) = seq.next_element::<&str>()? {
            commands.push(
                command
                    .parse::<MappableCommand>()
                    .map_err(serde::de::Error::custom)?,
            )
        }
        Ok(KeyTrieNode::CommandSequence(commands))
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut sub_key_trie = HashMap::new();
        while let Some((key_event, key_trie_node)) = map.next_entry::<KeyEvent, KeyTrieNode>()? {
            sub_key_trie.insert(key_event, key_trie_node);
        }
        Ok(KeyTrieNode::KeyTrie(KeyTrie::new("", sub_key_trie)))
    }
}