use derive_more::{Deref, DerefMut};
use std::collections::HashMap;

pub type Register = Vec<String>;

/// Currently just wraps a `HashMap` of `Register`s
#[derive(Debug, Clone, Default, Deref, DerefMut)]
pub struct Registers(HashMap<char, Register>);

impl Registers {
    pub fn read(&self, name: char) -> Option<&[String]> {
        self.get(&name).map(|register| register.as_slice())
    }

    pub fn write(&mut self, name: char, values: Vec<String>) {
        if name != '_' {
            self.insert(name, values);
        }
    }

    pub fn push(&mut self, name: char, value: String) {
        if name != '_' {
            if let Some(r) = self.get_mut(&name) {
                r.push(value);
            } else {
                self.write(name, vec![value]);
            }
        }
    }

    pub fn first(&self, name: char) -> Option<&String> {
        self.read(name).and_then(|entries| entries.first())
    }

    pub fn last(&self, name: char) -> Option<&String> {
        self.read(name).and_then(|entries| entries.last())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::info::Info;
    use once_cell::sync::Lazy;

    const REGISTER_VALUE_1_MOCK: &str = "value_1";
    const REGISTER_VALUE_2_MOCK: &str = "value_2";
    static REGISTERS_MOCK: Lazy<Registers> =
        Lazy::new(|| Registers([('/', vec![REGISTER_VALUE_1_MOCK.to_string()])].into()));

    #[test]
    fn infobox_shows_latest_value() {
        let mut registers = (*REGISTERS_MOCK).clone();
        registers.push('/', REGISTER_VALUE_2_MOCK.to_string());

        assert!(Info::from_registers(&registers)
            .text
            .contains(REGISTER_VALUE_2_MOCK));
    }
}
