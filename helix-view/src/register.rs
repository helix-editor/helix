use crate::{info::Info, input::KeyEvent};
use derive_more::{Deref, DerefMut};
use std::{collections::HashMap, str::FromStr};
use strum_macros::{AsRefStr, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr, EnumString)]
pub enum Register {
    #[strum(serialize = "/")]
    Search,
    #[strum(serialize = ":")]
    Command,
    #[strum(serialize = "\"")]
    Yank,
    #[strum(serialize = "_")]
    BlackHole,
    #[strum(serialize = "@")]
    Macro,
    #[strum(serialize = "#")]
    SelectionIndices,
    #[strum(serialize = "|")]
    Pipe,
}

impl Register {
    pub fn from_key_event(key_event: KeyEvent) -> Option<Register> {
        key_event
            .char()
            .and_then(|ch| Register::from_str(&ch.to_string()).ok())
    }
}

/// Currently just wraps a `HashMap` of `Register`s
#[derive(Debug, Clone, Default, Deref, DerefMut)]
pub struct Registers(HashMap<Register, Vec<String>>);

impl Registers {
    pub fn read(&self, register: Register) -> Option<&[String]> {
        self.get(&register).map(|register| register.as_slice())
    }

    pub fn write(&mut self, register: Register, values: Vec<String>) {
        if register != Register::BlackHole {
            self.insert(register, values);
        }
    }

    pub fn push(&mut self, register: Register, value: String) {
        if register != Register::BlackHole {
            if let Some(r) = self.get_mut(&register) {
                r.push(value);
            } else {
                self.write(register, vec![value]);
            }
        }
    }

    pub fn first(&self, register: Register) -> Option<&String> {
        self.read(register).and_then(|entries| entries.first())
    }

    pub fn last(&self, register: Register) -> Option<&String> {
        self.read(register).and_then(|entries| entries.last())
    }

    pub fn infobox(&self) -> Info {
        let body: Vec<_> = self
            .iter()
            .map(|(register, value)| {
                let content = value
                    .last()
                    .and_then(|s| s.lines().next())
                    .unwrap_or_default();
                (register.as_ref(), content)
            })
            .collect();

        let mut infobox = Info::new("Registers", &body);
        infobox.width = 30; // copied content could be very long
        infobox
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;

    const REGISTER_VALUE_1_MOCK: &str = "value_1";
    const REGISTER_VALUE_2_MOCK: &str = "value_2";
    static REGISTERS_MOCK: Lazy<Registers> = Lazy::new(|| {
        Registers([(Register::Search, vec![REGISTER_VALUE_1_MOCK.to_string()])].into())
    });

    #[test]
    fn infobox_shows_latest_value() {
        let mut registers = (*REGISTERS_MOCK).clone();
        registers.push(Register::Search, REGISTER_VALUE_2_MOCK.to_string());

        assert!(registers.infobox().text.contains(REGISTER_VALUE_2_MOCK));
    }
}
