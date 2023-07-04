use crate::info::Info;
use derive_more::{Display, From};
use std::collections::HashMap;

pub const YANK: Register = Register('"');
pub const SEARCH: Register = Register('/');
pub const COMMAND: Register = Register(':');
pub const BLACKHOLE: Register = Register('_');
pub const MACRO: Register = Register('@');
pub const SELECTION_INDICES: Register = Register('#');
pub const PIPE: Register = Register('|');

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Display)]
pub struct Register(char);

enum RegisterClass {
    Nested,
    Simple,
    NonWritable,
}

impl From<&Register> for RegisterClass {
    fn from(register: &Register) -> Self {
        match register {
            &YANK | &MACRO => RegisterClass::Nested,
            &SELECTION_INDICES | &BLACKHOLE => RegisterClass::NonWritable,
            _ => RegisterClass::Simple,
        }
    }
}

#[derive(Debug, Default)]
pub struct Registers {
    simple: HashMap<Register, Vec<String>>,
    nested: HashMap<Register, Vec<Vec<String>>>,
}

impl Registers {
    pub fn push(&mut self, register: Register, mut values: Vec<String>) {
        match RegisterClass::from(&register) {
            RegisterClass::Nested => {
                if let Some(register_values) = self.nested.get_mut(&register) {
                    register_values.push(values);
                } else {
                    self.nested.insert(register, vec![values]);
                }
            }
            RegisterClass::Simple => {
                if let Some(register_values) = self.simple.get_mut(&register) {
                    register_values.append(&mut values);
                } else {
                    self.simple.insert(register, values);
                }
            }
            RegisterClass::NonWritable => (),
        }
    }

    /// Pushes to newest entry for nested registers.
    pub fn push_singular(&mut self, register: Register, value: String) {
        match RegisterClass::from(&register) {
            RegisterClass::Nested => {
                if let Some(register_values) = self.nested.get_mut(&register) {
                    match register_values.last_mut() {
                        Some(newest_entry) => newest_entry.push(value),
                        None => register_values.push(vec![value]),
                    }
                } else {
                    self.nested.insert(register, vec![vec![value]]);
                }
            }
            RegisterClass::Simple => {
                if let Some(register_values) = self.simple.get_mut(&register) {
                    register_values.push(value);
                } else {
                    self.simple.insert(register, vec![value]);
                }
            }
            RegisterClass::NonWritable => (),
        }
    }

    /// All contents for nested registers, simple return a slice with only one value.
    pub fn newest(&self, register: &Register) -> Option<&[String]> {
        match RegisterClass::from(register) {
            RegisterClass::Nested => self
                .nested
                .get(register)
                .and_then(|nested_register| nested_register.last())
                .map(|values| values.as_slice()),
            RegisterClass::Simple => self
                .values(register)
                .map(|values| values.split_at(values.len() - 1).1),
            RegisterClass::NonWritable => None,
        }
    }

    /// First value in newest entry for nested registers.
    /// Newest value for simple registers.
    pub fn newest_singular(&self, register: &Register) -> Option<&str> {
        match RegisterClass::from(register) {
            RegisterClass::NonWritable => None,
            _ => self
                .newest(register)
                .and_then(|values| values.first())
                .map(|value| value.as_str()),
        }
    }

    /// All values of the newest entry for nested registers and all values from simple registers.
    pub fn values(&self, register: &Register) -> Option<&[String]> {
        match RegisterClass::from(register) {
            RegisterClass::Nested => self.newest(register),
            RegisterClass::Simple => self
                .simple
                .get(register)
                .map(|simple_register| simple_register.as_slice()),
            RegisterClass::NonWritable => None,
        }
    }

    pub fn clear(&mut self) {
        self.simple.clear();
        self.nested.clear();
    }

    pub fn remove(&mut self, register: &Register) -> bool {
        self.simple.remove(register).is_some() || self.nested.remove(register).is_some()
    }

    pub fn display_recent(&self) -> Vec<(String, &str)> {
        let mut body = Vec::with_capacity(self.simple.len() + self.nested.len());

        for register in self.nested.keys() {
            body.push((
                register.to_string(),
                self.newest(register)
                    .expect("Register should exist")
                    .first()
                    .and_then(|s| s.lines().next())
                    .unwrap_or_default(),
            ))
        }

        for register in self.simple.keys() {
            body.push((
                register.to_string(),
                self.newest(register)
                    .expect("Register should exist")
                    .first()
                    .and_then(|s| s.lines().next())
                    .unwrap_or_default(),
            ))
        }

        body
    }

    pub fn infobox(&self) -> Info {
        let mut infobox = Info::new("Registers", &self.display_recent());
        infobox.width = 30; // copied content could be very long
        infobox
    }
}
