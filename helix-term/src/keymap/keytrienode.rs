use super::keytrie::KeyTrie;
use crate::commands::MappableCommand;
use helix_view::input::KeyEvent;
use serde::{de::Visitor, Deserialize};
use std::collections::HashMap;

/// Each variant includes a description property.
/// For the MappableCommand and CommandSequence variants, the property is self explanatory.
/// For KeyTrie, the description is used for respective infobox titles,
/// or infobox KeyEvent descriptions that in themselves trigger the opening of another infobox.
/// See remapping.md for a further explanation of how descriptions are used.
#[derive(Debug, Clone)]
pub enum KeyTrieNode {
    MappableCommand(MappableCommand),
    CommandSequence(CommandSequence),
    KeyTrie(KeyTrie),
}

impl KeyTrieNode {
    pub fn get_description(&self) -> Option<&str> {
        match self {
            Self::MappableCommand(mappable_command) => Some(mappable_command.get_description()),
            Self::CommandSequence(command_sequence) => command_sequence.get_description(),
            Self::KeyTrie(node) => Some(node.get_description()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandSequence {
    description: Option<String>,
    commands: Vec<MappableCommand>,
}

impl CommandSequence {
    pub fn descriptionless(commands: Vec<MappableCommand>) -> Self {
        Self {
            description: None,
            commands,
        }
    }

    pub fn get_commands(&self) -> &Vec<MappableCommand> {
        &self.commands
    }

    pub fn get_description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl<'de> Deserialize<'de> for KeyTrieNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(KeyTrieNodeVisitor)
    }
}

impl PartialEq for KeyTrieNode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (KeyTrieNode::MappableCommand(_self), KeyTrieNode::MappableCommand(_other)) => {
                _self == _other
            }
            (KeyTrieNode::CommandSequence(_self), KeyTrieNode::CommandSequence(_other)) => {
                _self == _other
            }
            (KeyTrieNode::KeyTrie(_self), KeyTrieNode::KeyTrie(_other)) => {
                _self.get_children() == _other.get_children()
            }
            _ => false,
        }
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
        while let Some(command) = seq.next_element::<String>()? {
            commands.push(
                command
                    .parse::<MappableCommand>()
                    .map_err(serde::de::Error::custom)?,
            )
        }
        Ok(KeyTrieNode::CommandSequence(CommandSequence {
            description: None,
            commands,
        }))
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let into_keytrie = |peeked_key: String,
                            mut map: M,
                            description: &str,
                            user_explicit_description: bool|
         -> Result<Self::Value, M::Error> {
            let mut children = Vec::new();
            let mut child_order = HashMap::new();
            let mut keytrie_is_sticky = false;
            let mut user_explicit_sticky = false;
            let mut next_key = Some(peeked_key);

            while let Some(ref peeked_key) = next_key {
                if peeked_key == "sticky" {
                    keytrie_is_sticky = map.next_value::<bool>()?;
                    user_explicit_sticky = true;
                } else {
                    let key_event = peeked_key
                        .parse::<KeyEvent>()
                        .map_err(serde::de::Error::custom)?;
                    let keytrie_node = map.next_value::<KeyTrieNode>()?;
                    child_order.insert(key_event, children.len());
                    children.push(keytrie_node);
                }
                next_key = map.next_key::<String>()?;
            }

            let mut keytrie = KeyTrie::new(description, child_order, children);
            keytrie.is_sticky = keytrie_is_sticky;
            keytrie.explicitly_set_sticky = user_explicit_sticky;
            keytrie.explicitly_set_description = user_explicit_description;
            Ok(KeyTrieNode::KeyTrie(keytrie))
        };

        let Some(first_key) = map.next_key::<String>()? else {
            return Err(serde::de::Error::custom("Maps without keys are undefined keymap remapping behaviour."))
        };

        if first_key != "description" {
            return into_keytrie(first_key, map, "", false);
        }

        let description = map.next_value::<String>()?;

        if let Some(second_key) = map.next_key::<String>()? {
            if &second_key != "exec" {
                return into_keytrie(second_key, map, &description, true);
            }
            let keytrie_node: KeyTrieNode = map.next_value::<KeyTrieNode>()?;
            match keytrie_node {
            KeyTrieNode::KeyTrie(_) => Err(serde::de::Error::custom(
                "'exec' key reserved for command(s) only, omit when adding custom descriptions to nested remappings.",
            )),
            KeyTrieNode::MappableCommand(mappable_command) => {
                match mappable_command {
                    MappableCommand::Typable { name, args, .. } => {
                        Ok(KeyTrieNode::MappableCommand(MappableCommand::Typable {
                            name,
                            args,
                            description,
                        }))
                    }
                    MappableCommand::Static { .. } => {
                        Err(serde::de::Error::custom("Currently not possible to rename static commands, only typables. (Those that begin with a colon.) "))
                    }
                }
            },
            KeyTrieNode::CommandSequence(command_sequence) => {
                Ok(KeyTrieNode::CommandSequence(CommandSequence {
                    description: Some(description),
                    commands: command_sequence.commands,
                }))
            }
        }
        } else {
            let mut keytrie_node = KeyTrie::new(&description, HashMap::new(), Vec::new());
            keytrie_node.explicitly_set_description = true;
            Ok(KeyTrieNode::KeyTrie(keytrie_node))
        }
    }
}
