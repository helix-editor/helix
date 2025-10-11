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

#[cfg(feature = "scancode-hidapi")]
mod keyboard_state {
    use hidapi::HidApi;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::Arc;

    pub struct KeyboardState {
        codes: [Arc<AtomicU16>; 2],
        _handles: Vec<std::thread::JoinHandle<()>>,
    }
    const HID_KEYBOARD_USAGE_PAGE: u16 = 0x01;
    const HID_KEYBOARD_USAGE_ID: u16 = 0x06;
    const HID_MODIFIERS_MASK: [(u8, u16); 4] = [
        (0x01, 29), // Left Control
        (0x02, 42), // Left Shift
        (0x04, 56), // Left Alt
        (0x20, 54), // Right Shift
    ];

    // https://usb.org/sites/default/files/hut1_22.pdf
    // 10 Keyboard/Keypad Page (0x07)
    fn hid_keycode_to_scancode(hid_keycode: &u8) -> Option<u16> {
        Some(match hid_keycode {
            4 => 30,    // A
            5 => 48,    // B
            6 => 46,    // C
            7 => 32,    // D
            8 => 18,    // E
            9 => 33,    // F
            10 => 34,   // G
            11 => 35,   // H
            12 => 23,   // I
            13 => 36,   // J
            14 => 37,   // K
            15 => 38,   // L
            16 => 50,   // M
            17 => 49,   // N
            18 => 24,   // O
            19 => 25,   // P
            20 => 16,   // Q
            21 => 19,   // R
            22 => 31,   // S
            23 => 20,   // T
            24 => 22,   // U
            25 => 47,   // V
            26 => 17,   // W
            27 => 45,   // X
            28 => 21,   // Y
            29 => 44,   // Z
            30 => 2,    // 1
            31 => 3,    // 2
            32 => 4,    // 3
            33 => 5,    // 4
            34 => 6,    // 5
            35 => 7,    // 6
            36 => 8,    // 7
            37 => 9,    // 8
            38 => 10,   // 9
            39 => 11,   // 0
            40 => 28,   // Enter
            41 => 1,    // Escape
            42 => 14,   // Backspace
            43 => 15,   // Tab
            44 => 57,   // Space
            45 => 12,   // Minus (-)
            46 => 13,   // Equal (=)
            47 => 26,   // Left Bracket ([)
            48 => 27,   // Right Bracket (])
            49 => 43,   // Backslash (\)
            50 => 43,   // Non-US Hash (#)
            51 => 39,   // Semicolon (;)
            52 => 40,   // Apostrophe (')
            53 => 41,   // Grave (`)
            54 => 51,   // Comma (,)
            55 => 52,   // Period (.)
            56 => 53,   // Slash (/)
            57 => 58,   // Caps Lock
            58 => 59,   // F1
            59 => 60,   // F2
            60 => 61,   // F3
            61 => 62,   // F4
            62 => 63,   // F5
            63 => 64,   // F6
            64 => 65,   // F7
            65 => 66,   // F8
            66 => 67,   // F9
            67 => 68,   // F10
            68 => 87,   // F11
            69 => 88,   // F12
            70 => 99,   // Print Screen
            71 => 70,   // Scroll Lock
            72 => 119,  // Pause
            73 => 110,  // Insert
            74 => 102,  // Home
            75 => 104,  // Page Up
            76 => 111,  // Delete
            77 => 107,  // End
            78 => 109,  // Page Down
            79 => 106,  // Right Arrow
            80 => 105,  // Left Arrow
            81 => 108,  // Down Arrow
            82 => 103,  // Up Arrow
            83 => 69,   // Num Lock
            84 => 98,   // Keypad Slash (/)
            85 => 55,   // Keypad Asterisk (*)
            86 => 74,   // Keypad Minus (-)
            87 => 78,   // Keypad Plus (+)
            88 => 96,   // Keypad Enter
            89 => 79,   // Keypad 1
            90 => 80,   // Keypad 2
            91 => 81,   // Keypad 3
            92 => 75,   // Keypad 4
            93 => 76,   // Keypad 5
            94 => 77,   // Keypad 6
            95 => 71,   // Keypad 7
            96 => 72,   // Keypad 8
            97 => 73,   // Keypad 9
            98 => 82,   // Keypad 0
            99 => 83,   // Keypad Period (.)
            100 => 127, // Non-US Backslash (|)
            101 => 115, // Application
            102 => 128, // Power
            103 => 129, // Keypad Equal (=)
            104 => 130, // F13
            105 => 131, // F14
            106 => 132, // F15
            107 => 133, // F16
            108 => 134, // F17
            109 => 135, // F18
            110 => 136, // F19
            111 => 137, // F20
            112 => 138, // F21
            113 => 139, // F22
            114 => 140, // F23
            115 => 141, // F24
            116 => 142, // Execute
            117 => 143, // Help
            118 => 144, // Menu
            119 => 145, // Select
            120 => 146, // Stop
            121 => 147, // Again
            122 => 148, // Undo
            123 => 149, // Cut
            124 => 150, // Copy
            125 => 151, // Paste
            126 => 152, // Find
            127 => 153, // Mute
            128 => 154, // Volume Up
            129 => 155, // Volume Down
            130 => 156, // Locking Caps Lock
            131 => 157, // Locking Num Lock
            132 => 158, // Locking Scroll Lock
            133 => 159, // Keypad Comma (,)
            134 => 160, // Keypad Equal Sign (=)
            135 => 161, // International1 (Ro)
            136 => 162, // International2 (Katakana/Hiragana)
            137 => 163, // International3 (Yen)
            138 => 164, // International4 (Henkan)
            139 => 165, // International5 (Muhenkan)
            140 => 166, // International6 (PC9800 Keypad ,)
            141 => 167, // International7
            142 => 168, // International8
            143 => 169, // International9
            144 => 170, // Lang1 (Hangul/English)
            145 => 171, // Lang2 (Hanja)
            146 => 172, // Lang3 (Katakana)
            147 => 173, // Lang4 (Hiragana)
            148 => 174, // Lang5 (Zenkaku/Hankaku)
            149 => 175, // Lang6
            150 => 176, // Lang7
            151 => 177, // Lang8
            152 => 178, // Lang9
            153 => 179, // Alternate Erase
            154 => 180, // SysReq/Attention
            155 => 181, // Cancel
            156 => 182, // Clear
            157 => 183, // Prior
            158 => 184, // Return
            159 => 185, // Separator
            160 => 186, // Out
            161 => 187, // Oper
            162 => 188, // Clear/Again
            163 => 189, // CrSel/Props
            164 => 190, // ExSel
            _ => return None,
        })
    }

    fn hid_modifier_to_scancode(modifier_byte: &u8) -> Option<u16> {
        for (mask, scancode) in HID_MODIFIERS_MASK {
            if modifier_byte & mask != 0 {
                return Some(scancode);
            }
        }
        None
    }

    impl KeyboardState {
        pub fn new() -> Self {
            let key1 = Arc::new(AtomicU16::new(0));
            let key2 = Arc::new(AtomicU16::new(0));

            let mut handles = Vec::new();

            match HidApi::new() {
                Ok(api) => {
                    for device in api.device_list() {
                        let device_name = format!(
                            "{:?} ({:04x}:{:04x}) {} {}",
                            device.path(),
                            device.vendor_id(),
                            device.product_id(),
                            device.manufacturer_string().unwrap_or("-"),
                            device.product_string().unwrap_or("-")
                        );

                        if !(device.usage_page() == HID_KEYBOARD_USAGE_PAGE
                            && device.usage() == HID_KEYBOARD_USAGE_ID)
                        {
                            log::trace!("{device_name} isn't keyboard. skip");
                            continue;
                        }

                        let device = match device.open_device(&api) {
                            Ok(device) => {
                                log::info!("{device_name} start listen input reports");
                                device
                            }
                            Err(e) => {
                                log::error!("{device_name} error on open device: {e}");
                                continue;
                            }
                        };

                        let k1 = Arc::clone(&key1);
                        let k2 = Arc::clone(&key2);
                        handles.push(std::thread::spawn(move || {
                            let mut report = [0, 0, 0, 0, 0, 0, 0, 0];
                            loop {
                                match device.read(&mut report) {
                                    Ok(read) if read < 8 => {
                                        log::warn!("{device_name} partial read of input report");
                                        continue;
                                    }
                                    Err(e) => {
                                        log::error!("{device_name} read event error: {e}");
                                        break;
                                    }
                                    _ => (),
                                };

                                let mut pressed= 0;

                                // use last pressed key
                                for i in 0..6  {
                                    let hid_keycode = report[7 - i];
                                    if hid_keycode == 0 {
                                        continue;
                                    };
                                    let Some(scancode) = hid_keycode_to_scancode(&hid_keycode)
                                    else {
                                        continue;
                                    };
                                    log::trace!(
                                        "{device_name} hid_keycode: {hid_keycode} scancode: {scancode}"
                                    );
                                    pressed = scancode;
                                    break;
                                }

                                k1.store(pressed, Ordering::Relaxed);
                                k2.store(hid_modifier_to_scancode(&report[0]).unwrap_or(0), Ordering::Relaxed);
                            }
                        }));
                    }
                }
                Err(e) => {
                    log::error!("Error on initialize hidapi: {e}");
                }
            }

            Self {
                _handles: handles,
                codes: [key1, key2],
            }
        }

        pub fn get_scancodes(&mut self) -> [u16; 2] {
            [
                self.codes[0].load(Ordering::Relaxed), // key
                self.codes[1].load(Ordering::Relaxed), // modifier
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

        // get first non modifier key code
        let Some(scancode) = codes
            .iter()
            .find(|c| **c != 0 || !self.modifiers.contains(c))
            .cloned()
        else {
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
                entry!(53, "/", "?"),
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
