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

const HISTORY_LENGTH: usize = 10;

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
    pub fn display_recent(&self) -> Vec<(String, &str)> {
        let mut body = Vec::with_capacity(self.simple.len() + self.nested.len());
        for register in self.list_writable() {
            body.push((
                register.to_string(),
                self.newest(register)
                    .expect("Register should exist")
                    .first()
                    .map(|string_ref| string_ref.as_str())
                    .unwrap_or_default(),
            ))
        }

        body
    }

    pub fn infobox(&self) -> Info {
        Self::prepare_infobox("Registers", self.display_recent())
    }

    pub fn list_registers_infobox(&self) -> Info {
        Self::prepare_infobox(
            "Select registers",
            self.list_writable()
                .into_iter()
                .map(|register| (register.to_string(), ""))
                .collect(),
        )
    }

    /// Newest pushed values are shown first with the indices reversed.
    /// E.g internal `[(0, oldest), (1, mid), (2, newest)]` is shown as `[(0, newest), (1, mid), (2, oldest)]`.
    pub fn register_history_infobox(&self, register: &Register) -> Info {
        let body = match RegisterClass::from(register) {
            RegisterClass::Nested => {
                prepare_history_infobox(register, &self.nested, |(index, values)| {
                    (
                        index,
                        values
                            .first()
                            .map(|string_ref| string_ref.as_str())
                            .unwrap_or_default(),
                    )
                })
            }
            RegisterClass::Simple => {
                prepare_history_infobox(register, &self.simple, |(i, s)| (i, s.as_str()))
            }
            RegisterClass::NonWritable => unreachable!(),
        };

        return Self::prepare_infobox(format!("Register: {}", register), body);

        fn prepare_history_infobox<'a, 'b, T, F: Fn((String, &'b T)) -> (String, &'b str)>(
            register_key: &'a Register,
            register_map: &'b HashMap<Register, Vec<T>>,
            values_map_fn: F,
        ) -> Vec<(String, &'b str)> {
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

    fn prepare_infobox<S: AsRef<str>>(title: S, mut body: Vec<(String, &str)>) -> Info {
        // Show only the first line:
        body.iter_mut()
            .for_each(|(_, value)| *value = value.lines().next().unwrap_or_default());
        let mut infobox = Info::new(title.as_ref(), &body);
        // Line could be very long
        infobox.width = 30;
        infobox
    }

    fn list_writable(&self) -> Vec<&Register> {
        self.simple.keys().chain(self.nested.keys()).collect()
    }
}
