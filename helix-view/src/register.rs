//! Editor/Context independent registers.
use std::{collections::HashMap, fmt::Display};

pub const YANK: Register = Register('"');
pub const SEARCH: Register = Register('/');
pub const COMMAND: Register = Register(':');
pub const BLACKHOLE: Register = Register('_');
pub const MACRO: Register = Register('@');
pub const PIPE: Register = Register('|');

const HISTORY_LENGTH: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Register(char);

impl Register {
    pub const fn from_char(ch: char) -> Self {
        Self(ch)
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

enum RegisterClass {
    Nested,
    Simple,
    NonWritable,
}

impl From<&Register> for RegisterClass {
    fn from(register: &Register) -> Self {
        match register {
            &YANK | &MACRO => RegisterClass::Nested,
            &BLACKHOLE => RegisterClass::NonWritable,
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
    pub fn push_values(&mut self, register: Register, mut values: Vec<String>) {
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

    /// Error if the register_index is out of bounds.
    /// Does nothing is the register does not exist/is non-writable.
    pub fn set_newest(&mut self, register: &Register, index: usize) -> Result<(), usize> {
        match RegisterClass::from(register) {
            RegisterClass::Nested => {
                if let Some(register_values) = self.nested.get_mut(register) {
                    bump_history(index, register_values)?;
                }
            }
            RegisterClass::Simple => {
                if let Some(register_values) = self.simple.get_mut(register) {
                    bump_history(index, register_values)?;
                }
            }
            RegisterClass::NonWritable => (),
        }

        return Ok(());

        // Empty does not have to be checked as an empty register is assumed to not extist.
        fn bump_history<T>(index: usize, history: &mut [T]) -> Result<(), usize> {
            // index == history.len() is a no-op operation in rotate_left/right().
            if index > history.len() {
                return Err(index);
            }

            history[index..].rotate_left(1);

            Ok(())
        }
    }

    /// All contents for nested registers, simple returns a slice with only one value.
    pub fn newest_values(&self, register: &Register) -> Option<&[String]> {
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
    pub fn newest_value(&self, register: &Register) -> Option<&str> {
        match RegisterClass::from(register) {
            RegisterClass::NonWritable => None,
            _ => self
                .newest_values(register)
                .and_then(|values| values.first())
                .map(|value| value.as_str()),
        }
    }

    /// All values of the newest entry for nested registers and all values from simple registers.
    pub fn values(&self, register: &Register) -> Option<&[String]> {
        match RegisterClass::from(register) {
            RegisterClass::Nested => self.newest_values(register),
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

    pub fn size(&self, register: &Register) -> Option<usize> {
        match RegisterClass::from(register) {
            RegisterClass::Nested => self.nested.get(register).map(|values| values.len()),
            RegisterClass::Simple => self.simple.get(register).map(|values| values.len()),
            RegisterClass::NonWritable => None,
        }
    }
}

// Presentation methods
impl Registers {
    pub fn newest_values_display(&self) -> Vec<(String, String)> {
        let mut body = Vec::with_capacity(self.simple.len() + self.nested.len());
        for register in self.list_writable() {
            body.push((
                register.to_string(),
                self.newest_values(register)
                    .expect("Register should exist")
                    .first()
                    .map(String::to_string)
                    .unwrap_or_default(),
            ))
        }

        body
    }

    pub fn listed_info_body(&self) -> Vec<(String, String)> {
        self.list_writable()
            .into_iter()
            .map(|register| {
                (
                    register.to_string(),
                    format!(
                        "({})",
                        self.size(register)
                            .expect("Writable register should have a size.")
                    ),
                )
            })
            .collect()
    }

    /// Newest pushed values are shown first with the indices reversed.
    /// E.g internal `[(0, oldest), (1, mid), (2, newest)]` is shown as `[(0, newest), (1, mid), (2, oldest)]`.
    pub fn register_history_info_body(&self, register: &Register) -> Vec<(String, String)> {
        return match RegisterClass::from(register) {
            RegisterClass::Nested => {
                prepare_history_infobox(register, &self.nested, |(index, values)| {
                    (
                        index,
                        values.first().map(String::to_string).unwrap_or_default(),
                    )
                })
            }
            RegisterClass::Simple => {
                prepare_history_infobox(register, &self.simple, |(i, s)| (i, s.to_string()))
            }
            RegisterClass::NonWritable => unreachable!(),
        };

        fn prepare_history_infobox<T, F: Fn((String, &T)) -> (String, String)>(
            register_key: &Register,
            register_map: &HashMap<Register, Vec<T>>,
            values_map_fn: F,
        ) -> Vec<(String, String)> {
            register_map
                .get(register_key)
                .expect("Register should exist")
                .iter()
                .take(HISTORY_LENGTH)
                .rev()
                .enumerate()
                .map(|(index, value_preview)| (index.to_string(), value_preview))
                .map(values_map_fn)
                .collect()
        }
    }

    fn list_writable(&self) -> Vec<&Register> {
        self.simple.keys().chain(self.nested.keys()).collect()
    }
}
