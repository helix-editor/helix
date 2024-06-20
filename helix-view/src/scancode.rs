use crate::input::KeyEvent;
use crate::keyboard::{KeyCode, ModifierKeyCode};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

use keyboard_query::{DeviceQuery, DeviceState};

pub struct KeyboardState {
    device_state: DeviceState,
    previous_codes: Vec<u16>,
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardState {
    pub fn new() -> Self {
        Self {
            previous_codes: Vec::new(),
            device_state: DeviceState::new(),
        }
    }

    pub fn get_keys(&mut self) -> Vec<u16> {
        // return only scancode for new pressed key
        let codes = self.device_state.get_keys();
        let new_codes = if codes.len() <= 1 {
            codes.clone()
        } else {
            codes
                .clone()
                .into_iter()
                .filter(|c| !self.previous_codes.contains(c))
                .collect()
        };
        self.previous_codes = codes;
        new_codes
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct ScanCodeMap {
    // {<name>: {<code>: (char, shifted char)}}
    map: HashMap<u16, (KeyCode, Option<KeyCode>)>,
    modifiers: Vec<u16>,
    shift_modifiers: Vec<u16>,
}

impl ScanCodeMap {
    pub fn new(map: HashMap<u16, (KeyCode, Option<KeyCode>)>) -> Self {
        let modifiers = map
            .iter()
            .filter_map(|(code, (key, _))| {
                if matches!(key, KeyCode::Modifier(_)) {
                    Some(*code)
                } else {
                    None
                }
            })
            .collect();

        let shift_modifiers = map
            .iter()
            .filter_map(|(code, (key, _))| {
                if matches!(
                    key,
                    KeyCode::Modifier(ModifierKeyCode::LeftShift)
                        | KeyCode::Modifier(ModifierKeyCode::RightShift)
                ) {
                    Some(*code)
                } else {
                    None
                }
            })
            .collect();
        Self {
            map,
            modifiers,
            shift_modifiers,
        }
    }

    pub fn apply(&self, event: KeyEvent, scancodes: &[u16]) -> KeyEvent {
        if scancodes.is_empty() {
            return event;
        }

        // get fist non modifier key code
        // TODO how to process multiple key pressed?
        let Some(scancode) = scancodes
            .iter()
            .find(|c| !self.modifiers.contains(c))
            .cloned()
        else {
            return event;
        };

        let Some((key, shift_key)) = self.map.get(&scancode) else {
            return event;
        };

        let event_before = event;

        // TODO how to check capslock on?
        let mut is_shifted = false;
        for c in &self.shift_modifiers {
            if scancodes.contains(c) {
                is_shifted = true;
                break;
            }
        }

        let event = KeyEvent {
            code: match key {
                KeyCode::Char(c) => {
                    if is_shifted | c.is_ascii_uppercase() {
                        (*shift_key).unwrap_or_else(|| KeyCode::Char(c.to_ascii_uppercase()))
                    } else {
                        *key
                    }
                }
                _ => *key,
            },
            ..event
        };

        log::trace!(
            "Scancodes: {scancodes:?} Scancode: {scancode:?} Event source {event_before:?} New Event {event:?}"
        );

        event
    }
}

pub fn deserialize_scancode<'de, D>(deserializer: D) -> Result<ScanCodeMap, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    struct ScanCodeRawConfig {
        layout: String,
        map: HashMap<String, Vec<(u16, Vec<String>)>>,
    }

    let value = ScanCodeRawConfig::deserialize(deserializer)?;
    let map = HashMap::from_iter(
        value
            .map
            .into_iter()
            // load only specified in settings layout
            .find(|(k, _)| k == &value.layout)
            .ok_or_else(|| {
                <D::Error as Error>::custom(format!("Scancode map not found for: {}", value.layout))
            })?
            .1
            .into_iter()
            .map(|(scancode, chars)| {
                if chars.is_empty() {
                    return Err(<D::Error as Error>::custom(format!(
                        "Invalid scancode. Empty map for scancode: {scancode} on layout: {}",
                        value.layout
                    )));
                }
                if chars.len() > 2 {
                    return Err(<D::Error as Error>::custom(format!(
                        "Invalid scancode. To many variants for scancode: {scancode} on layout: {}",
                        value.layout
                    )));
                }
                let keycode = str::parse::<KeyCode>(&chars[0]).map_err(|e| {
                    <D::Error as Error>::custom(format!(
                        "On parse scancode: {scancode} on layout: {} - {e}",
                        value.layout
                    ))
                })?;
                let shifted_keycode = if let Some(c) = chars.get(1) {
                    Some(str::parse::<KeyCode>(c).map_err(|e| {
                        <D::Error as Error>::custom(format!(
                            "On parse scancode: {scancode} on layout: {} - {e}",
                            value.layout
                        ))
                    })?)
                } else {
                    None
                };
                Ok((scancode, (keycode, shifted_keycode)))
            })
            .collect::<Result<Vec<_>, D::Error>>()?,
    );
    Ok(ScanCodeMap::new(map))
}
