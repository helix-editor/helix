pub mod default;
pub mod macros;

use std::{collections::HashMap, sync::Arc, time::Duration};

use arc_swap::{
    access::{DynAccess, DynGuard},
    ArcSwap,
};
use chrono::{DateTime, Local};
use helix_view::{
    document::Mode,
    input::{MouseEvent, MouseEventKind, MouseModifiers},
};
use serde::Deserialize;

use crate::commands::mouse::StaticMouseCommand;

pub type MouseTrieMapper = HashMap<MouseEvent, MouseTrie>;

#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, Clone)]
pub enum MouseTrie {
    MappableCommand(StaticMouseCommand),
    Sequence(Vec<StaticMouseCommand>),
}

struct MouseTrieVisitor;

impl<'de> Deserialize<'de> for MouseTrie {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        log::info!("going into Visitor");
        deserializer.deserialize_any(MouseTrieVisitor)
    }
}

impl<'de> serde::de::Visitor<'de> for MouseTrieVisitor {
    type Value = MouseTrie;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "an internal command or list of internal commands"
        )
    }

    fn visit_str<E>(self, command: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        log::info!("gone into seq");
        command
            .parse::<StaticMouseCommand>()
            .map(MouseTrie::MappableCommand)
            .map_err(E::custom)
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: serde::de::SeqAccess<'de>,
    {
        log::info!("gone into seq");
        let mut commands = Vec::new();
        while let Some(command) = seq.next_element::<String>()? {
            commands.push(
                command
                    .parse::<StaticMouseCommand>()
                    .map_err(serde::de::Error::custom)?,
            )
        }
        Ok(MouseTrie::Sequence(commands))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MousemapResult {
    /// Needs more keys to execute a command. Contains valid keys for next keystroke.
    Matched(StaticMouseCommand),
    /// Matched a sequence of commands to execute.
    MatchedSequence(Vec<StaticMouseCommand>),
    /// Key was not found in the root keymap
    NotFound,
}

pub struct Mousemaps {
    pub map: Box<dyn DynAccess<HashMap<Mode, HashMap<MouseEvent, MouseTrie>>>>,
    /// Stores pending keys waiting for the next key. This is relative to a
    /// sticky node if one is in use.
    last_event: Option<MouseEvent>,
    last_time_mouse_pressed: DateTime<Local>,
}

impl Mousemaps {
    pub fn new(map: Box<dyn DynAccess<HashMap<Mode, HashMap<MouseEvent, MouseTrie>>>>) -> Self {
        Self {
            map,
            last_event: None,
            last_time_mouse_pressed: Local::now(),
        }
    }

    pub fn map(&self) -> DynGuard<HashMap<Mode, HashMap<MouseEvent, MouseTrie>>> {
        self.map.load()
    }

    /// Returns list of keys waiting to be disambiguated in current mode.
    pub fn last_event(&self) -> Option<&MouseEvent> {
        self.last_event.as_ref()
    }

    fn get_from_event(
        &self,
        values: &HashMap<MouseEvent, MouseTrie>,
        key: &MouseEvent,
    ) -> MousemapResult {
        match values.get(key) {
            Some(v) => match v {
                MouseTrie::MappableCommand(m) => MousemapResult::Matched(m.to_owned()),
                MouseTrie::Sequence(m) => MousemapResult::MatchedSequence(m.to_owned()),
            },
            None => MousemapResult::NotFound,
        }
    }

    pub fn get(&mut self, mode: Mode, key: &MouseEvent, mouse_idle: &Duration) -> MousemapResult {
        let mousemaps = &*self.map();

        if let Some(values) = mousemaps.get(&mode) {
            match key.kind {
                MouseEventKind::Down(_) => {
                    let current_date = Local::now();
                    let diff = current_date - self.last_time_mouse_pressed;
                    self.last_time_mouse_pressed = current_date;
                    let mut replace_key = true;
                    if diff.num_milliseconds() as u128 > mouse_idle.as_millis() {
                        self.last_event = None;
                    } else if let Some(last_mouse_event) = self.last_event.as_mut() {
                        // same modifiers (not mouse_mofifiers) and buttons (columns are excepted)
                        if last_mouse_event.light_eq(&key) {
                            last_mouse_event.mouse_modifiers =
                                match last_mouse_event.mouse_modifiers {
                                    MouseModifiers::MultipleClick(v) => {
                                        MouseModifiers::MultipleClick(v + 1)
                                    }
                                };
                            replace_key = false;
                        }
                    }

                    if replace_key {
                        self.last_event = Some(key.clone_without_coords());
                    }
                    let res = self.get_from_event(values, self.last_event.as_ref().unwrap());
                    return res;
                }
                MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                    self.last_event = None;
                    let res = self.get_from_event(values, &key.clone_without_coords());
                    return res;
                }
                _ => (),
            }
        }
        return MousemapResult::NotFound;
    }
}

impl Default for Mousemaps {
    fn default() -> Self {
        Self::new(Box::new(ArcSwap::new(Arc::new(default::default()))))
    }
}

pub fn merge_mouse_keys(
    dst: &mut HashMap<Mode, MouseTrieMapper>,
    delta: &HashMap<Mode, MouseTrieMapper>,
) {
    for (mode, mapper) in delta.iter() {
        if let Some(dst_mapper) = dst.get_mut(mode) {
            for (event, trie) in mapper.iter() {
                dst_mapper.insert(*event, trie.to_owned());
            }
        } else {
            dst.insert(*mode, mapper.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mousemap;
    use helix_core::hashmap;

    use super::Mousemaps;

    #[test]
    #[should_panic]
    fn duplicate_mouse_keys_should_panic() {
        mousemap!({
            "1-left" => code_action,
            "1-left" => add_selection_mouse,
        });
    }

    #[test]
    fn check_duplicate_keys_in_default_mousemap() {
        Mousemaps::default();
    }
}
