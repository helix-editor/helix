use crate::input::KeyEvent;
use crate::keyboard::{KeyCode, ModifierKeyCode};
use anyhow;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

type ScanCodeKeyCodeMap = HashMap<u16, (KeyCode, Option<KeyCode>)>;

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct ScanCodeMap {
    // {<code>: (char, shifted char)}
    map: ScanCodeKeyCodeMap,
    modifiers: Vec<u16>,
    shift_modifiers: Vec<u16>,
}

pub use keyboard_state::KeyboardState;

impl Default for KeyboardState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "scancode-query")]
mod keyboard_state {
    use keyboard_query::{DeviceQuery, DeviceState};

    pub struct KeyboardState {
        device_state: DeviceState,
        previous_codes: Vec<u16>,
    }

    impl KeyboardState {
        pub fn new() -> Self {
            Self {
                previous_codes: Vec::new(),
                device_state: DeviceState::new(),
            }
        }

        pub fn get_scancodes(&mut self) -> Vec<u16> {
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
            self.previous_codes = codes.clone();
            new_codes
        }
    }
}

#[cfg(feature = "scancode-evdev")]
mod keyboard_state {
    use evdev::{Device, KeyCode};
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::Arc;

    struct DeviceHandle {
        _path: std::path::PathBuf,
        _handle: tokio::task::JoinHandle<()>,
    }

    pub struct KeyboardState {
        codes: [Arc<AtomicU16>; 2],
        _handle: Vec<DeviceHandle>,
    }

    fn is_keyboard(device: &Device) -> bool {
        device
            .supported_keys()
            .map_or(false, |keys| keys.contains(KeyCode::KEY_ENTER))
    }

    impl KeyboardState {
        pub fn new() -> Self {
            let key1 = Arc::new(AtomicU16::new(0));
            let key2 = Arc::new(AtomicU16::new(0));

            // find keyboards
            let keyboards = evdev::enumerate()
                .filter(|(_, dev)| is_keyboard(dev))
                .collect::<Vec<_>>();

            let mut handles = Vec::new();

            // evdev constant
            const KEY_STATE_RELEASE: i32 = 0;

            for (path, mut item) in keyboards {
                // skip already grabbed keyboards
                let is_grabbed = item.grab().is_err();
                if !is_grabbed {
                    if let Err(e) = item.ungrab() {
                        log::error!("Failed to ungrab input: {e}");
                    }
                }
                if is_grabbed {
                    continue;
                }
                let k1 = Arc::clone(&key1);
                let k2 = Arc::clone(&key2);
                let mut codes = [0, 0];
                let device_path = path.to_str().unwrap_or_default().to_owned();
                let handle = tokio::task::spawn(async move {
                    let device_name = item.name().unwrap_or_default().to_owned();
                    log::info!("Start listen events from: {device_name} ({device_path})");
                    let Ok(mut events) = item.into_event_stream() else {
                        log::error!("Failed to stream events from: {device_name} ({device_path})");
                        return;
                    };

                    while let Ok(event) = events.next_event().await {
                        if evdev::EventType::KEY != event.event_type() {
                            continue;
                        };
                        let scancode: u16 = event.code();
                        if event.value() == KEY_STATE_RELEASE {
                            // reset state
                            codes = match (codes[0] == scancode, codes[1] == scancode) {
                                (true, false) => [0, codes[1]],
                                (false, true) => [0, codes[0]],
                                _ => [0, 0],
                            }
                        } else {
                            // don't repeat
                            if !codes.contains(&scancode) {
                                codes = [codes[1], scancode];
                            }
                        }
                        k1.store(codes[0], Ordering::Relaxed);
                        k2.store(codes[1], Ordering::Relaxed);
                    }
                });

                handles.push(DeviceHandle {
                    _path: path,
                    _handle: handle,
                })
            }

            Self {
                _handle: handles,
                codes: [key1, key2],
            }
        }

        pub fn get_scancodes(&mut self) -> [u16; 2] {
            [
                self.codes[1].swap(0, Ordering::Relaxed),
                self.codes[0].swap(0, Ordering::Relaxed),
            ]
        }
    }
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

    pub fn apply(&self, event: KeyEvent, keyboard: &mut KeyboardState) -> KeyEvent {
        let codes = keyboard.get_scancodes();
        if codes.is_empty() {
            return event;
        }

        // get fist non modifier key code
        let Some(scancode) = codes.iter().find(|c| !self.modifiers.contains(c)).cloned() else {
            return event;
        };

        let Some((key, shifted_key)) = self.map.get(&scancode) else {
            return event;
        };

        let event_before = event;

        let mut is_shifted = false;
        for c in &self.shift_modifiers {
            if codes.contains(c) {
                is_shifted = true;
                break;
            }
        }

        let event = KeyEvent {
            code: match key {
                KeyCode::Char(c) => {
                    if is_shifted | c.is_ascii_uppercase() {
                        (*shifted_key).unwrap_or(*key)
                    } else {
                        *key
                    }
                }
                _ => *key,
            },
            ..event
        };

        log::trace!(
            "{:?} map to {:?} by scancode {codes:?} (code: {scancode}, key: {key:?}, shifted key: {shifted_key:?})",
            event_before.code,
            event.code,
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
        map: Option<HashMap<String, Vec<(u16, Vec<String>)>>>,
    }

    let value = ScanCodeRawConfig::deserialize(deserializer)?;

    // load only specified in user settings layout
    let map = if let Some(map) = value
        .map
        .and_then(|m| m.into_iter().find(|(k, _)| k == &value.layout))
    {
        HashMap::from_iter(
            map.1
                .into_iter()
                .map(|(scancode, chars)| {
                    if chars.is_empty() {
                        anyhow::bail!(
                            "Invalid scancode. Empty map for scancode: {scancode} on layout: {}",
                            value.layout
                        );
                    }
                    if chars.len() > 2 {
                        anyhow::bail!(
                        "Invalid scancode. To many variants for scancode: {scancode} on layout: {}",
                        value.layout
                    );
                    }
                    let keycode = str::parse::<KeyCode>(&chars[0]).map_err(|e| {
                        anyhow::anyhow!(
                            "On parse scancode: {scancode} on layout: {} - {e}",
                            value.layout
                        )
                    })?;
                    let shifted_keycode = if let Some(c) = chars.get(1) {
                        Some(str::parse::<KeyCode>(c).map_err(|e| {
                            anyhow::anyhow!(
                                "On parse scancode: {scancode} on layout: {} - {e}",
                                value.layout
                            )
                        })?)
                    } else {
                        None
                    };
                    Ok((scancode, (keycode, shifted_keycode)))
                })
                .collect::<anyhow::Result<Vec<_>>>()
                .map_err(|e| <D::Error as Error>::custom(e))?,
        )
    } else {
        // lookup in hardcoded defaults
        let Some(map) = defaults::LAYOUTS.get(value.layout.as_str()) else {
            return Err(<D::Error as Error>::custom(format!(
                "Scancode layout not found for: {}",
                value.layout
            )));
        };

        log::debug!(
            "User defined scancode layout not found: {}. Use default",
            value.layout
        );

        map.to_owned()
    };

    Ok(ScanCodeMap::new(map))
}

mod defaults {

    use super::ScanCodeKeyCodeMap;
    use crate::keyboard::KeyCode;
    use std::collections::HashMap;
    use std::str::FromStr;

    macro_rules! entry {
        ($scancode:expr, $keycode:literal) => {
            (
                $scancode,
                (
                    KeyCode::from_str($keycode).expect("Failed to parse {$keycode} as KeyCode"),
                    None,
                ),
            )
        };
        ($scancode:expr, $keycode:literal, $shifted_keycode:literal) => {
            (
                $scancode,
                (
                    KeyCode::from_str($keycode).expect("Failed to parse {$keycode} as KeyCode"),
                    Some(
                        KeyCode::from_str($shifted_keycode)
                            .expect("Failed to parse {$shifted_keycode} as KeyCode"),
                    ),
                ),
            )
        };
    }

    pub static LAYOUTS: once_cell::sync::Lazy<HashMap<&'static str, ScanCodeKeyCodeMap>> =
        once_cell::sync::Lazy::new(init);

    fn init() -> HashMap<&'static str, ScanCodeKeyCodeMap> {
        HashMap::from_iter([qwerty()])
    }

    fn qwerty() -> (&'static str, ScanCodeKeyCodeMap) {
        // https://github.com/emberian/evdev/blob/8feea0685b0acb8153e394ffc393cf560d30a16f/src/scancodes.rs#L30
        (
            "qwerty",
            HashMap::from_iter([
                entry!(1, "esc"),
                entry!(2, "1", "!"),
                entry!(3, "2", "@"),
                entry!(4, "3", "#"),
                entry!(5, "4", "$"),
                entry!(5, "4", "$"),
                entry!(6, "5", "%"),
                entry!(7, "6", "^"),
                entry!(8, "7", "&"),
                entry!(9, "8", "*"),
                entry!(10, "9", "("),
                entry!(11, "0", ")"),
                entry!(12, "-", "_"),
                entry!(13, "=", "+"),
                entry!(14, "backspace"),
                entry!(15, "tab"),
                entry!(16, "q", "Q"),
                entry!(17, "w", "W"),
                entry!(18, "e", "E"),
                entry!(19, "r", "R"),
                entry!(20, "t", "T"),
                entry!(21, "y", "Y"),
                entry!(22, "u", "U"),
                entry!(23, "i", "I"),
                entry!(24, "o", "O"),
                entry!(25, "p", "P"),
                entry!(26, "[", "{"),
                entry!(27, "]", "}"),
                entry!(28, "ret"),
                entry!(29, "leftcontrol"),
                entry!(30, "a", "A"),
                entry!(31, "s", "S"),
                entry!(32, "d", "D"),
                entry!(33, "f", "F"),
                entry!(34, "g", "G"),
                entry!(35, "h", "H"),
                entry!(36, "j", "J"),
                entry!(37, "k", "K"),
                entry!(38, "l", "L"),
                entry!(39, ";", ":"),
                entry!(40, "'", "\""),
                entry!(41, "`", "~"),
                entry!(42, "leftshift"),
                entry!(43, "\\", "|"),
                entry!(44, "z", "Z"),
                entry!(45, "x", "X"),
                entry!(46, "c", "C"),
                entry!(47, "v", "V"),
                entry!(48, "b", "B"),
                entry!(49, "n", "N"),
                entry!(50, "m", "M"),
                entry!(51, ",", "<"),
                entry!(52, ".", ">"),
                entry!(53, "/", "|"),
                entry!(54, "rightshift"),
                entry!(55, "printscreen"),
                entry!(56, "leftalt"),
                entry!(57, "space"),
                entry!(58, "capslock"),
                entry!(59, "F1"),
                entry!(60, "F2"),
                entry!(61, "F3"),
                entry!(62, "F4"),
                entry!(63, "F5"),
                entry!(64, "F6"),
                entry!(65, "F7"),
                entry!(66, "F8"),
                entry!(67, "F9"),
                entry!(68, "F10"),
                entry!(74, "-"),
                entry!(78, "+"),
                // Not processes by Helix
                // entry!(69, "numlock"),
                // entry!(70, "scrolllock"),
                // entry!(71, "home"),
                // entry!(72, "up"),
                // entry!(73, "pageup"),
                // entry!(75, "left"),
                // entry!(77, "right"),
                // entry!(79, "end"),
                // entry!(80, "down"),
                // entry!(81, "pagedown"),
                // entry!(82, "ins"),
                // entry!(83, "del"),
            ]),
        )
    }
}
