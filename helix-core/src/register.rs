use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Register {
    name: char,
    values: Vec<String>,
}

impl Register {
    pub const fn new(name: char) -> Self {
        Self {
            name,
            values: Vec::new(),
        }
    }

    pub fn new_with_values(name: char, values: Vec<String>) -> Self {
        if name == '_' {
            Self::new(name)
        } else {
            Self { name, values }
        }
    }

    pub const fn name(&self) -> char {
        self.name
    }

    pub fn read(&self) -> &[String] {
        &self.values
    }

    pub fn write(&mut self, values: Vec<String>) {
        if self.name != '_' {
            self.values = values;
        }
    }

    pub fn push(&mut self, value: String) {
        if self.name != '_' {
            self.values.push(value);
        }
    }
}

/// Currently just wraps a `HashMap` of `Register`s
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Registers {
    inner: HashMap<char, Register>,
}

impl Registers {
    pub fn get(&self, name: char) -> Option<&Register> {
        self.inner.get(&name)
    }

    pub fn get_mut(&mut self, name: char) -> &mut Register {
        self.inner
            .entry(name)
            .or_insert_with(|| Register::new(name))
    }

    pub fn write(&mut self, name: char, values: Vec<String>) {
        self.inner
            .insert(name, Register::new_with_values(name, values));
    }

    pub fn load(&mut self) -> std::io::Result<()> {
        let mut file = File::open("foo.macro")?;
        let mut buf_reader = BufReader::new(file);
        let mut content = String::new();

        buf_reader.read_to_string(&mut content)?;

        let deserialized: Register = serde_json::from_str(&content).unwrap();

        self.inner.insert('@', deserialized);

        Ok(())
    }

    pub fn save(&mut self, name: char, file_name: &Path) -> std::io::Result<()> {
        let macro_definition = self.get(name);

        let content = serde_json::to_string(&macro_definition).unwrap();

        let mut file = File::create(&file_name)?;
        file.write_all(&content.as_bytes());

        Ok(())
    }

    pub fn read(&self, name: char) -> Option<&[String]> {
        self.get(name).map(|reg| reg.read())
    }

    pub fn first(&self, name: char) -> Option<&String> {
        self.read(name).and_then(|entries| entries.first())
    }

    pub fn last(&self, name: char) -> Option<&String> {
        self.read(name).and_then(|entries| entries.last())
    }

    pub fn inner(&self) -> &HashMap<char, Register> {
        &self.inner
    }
}
