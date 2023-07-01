use derive_more::{Deref, DerefMut};
use std::collections::HashMap;

pub type Register = Vec<String>;

/// Currently just wraps a `HashMap` of `Register`s
#[derive(Debug, Clone, Default, Deref, DerefMut)]
pub struct Registers(pub HashMap<char, Register>);

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
