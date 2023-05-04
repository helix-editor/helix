use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
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
        Self { name, values }
    }

    pub const fn name(&self) -> char {
        self.name
    }

    pub fn read(&self) -> &[String] {
        &self.values
    }

    pub fn write(&mut self, values: Vec<String>) {
        self.values = values;
    }

    pub fn push(&mut self, value: String) {
        self.values.push(value);
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

    pub fn read(&self, name: char) -> Option<&[String]> {
        self.get(name).map(|reg| reg.read())
    }

    pub fn write(&mut self, name: char, values: Vec<String>) {
        if name != '_' {
            self.inner
                .insert(name, Register::new_with_values(name, values));
        }
    }

    pub fn load(&mut self, name: char) -> Result<(), String> {
        fn _read_data(name: char) -> Result<String, io::Error> {
            let file_name = format!("{}.macro", &name);
            let file = File::open(&file_name)?;
            let mut reader = BufReader::new(file);
            let mut content = String::new();

            reader.read_to_string(&mut content)?;

            return Ok(content);
        }

        fn _string_to_register(str: String) -> Result<Register, serde_json::Error> {
            let deserialized: Register = serde_json::from_str(&str)?;

            return Ok(deserialized);
        }

        match _read_data(name) {
            Ok(content) => match _string_to_register(content) {
                Ok(register) => match self.inner.insert(name, register) {
                    Some(_) => Ok(()),
                    None => Err(format!("Could not insert loaded register.")),
                },
                Err(error) => Err(format!("Insertion failed: {}", error)),
            },
            Err(error) => Err(format!("Deserialization failed: {}", error)),
        }
    }

    pub fn save(&mut self, name: char) -> Result<(), String> {
        fn _write(file_name: &Path, content: String) -> Result<(), io::Error> {
            let mut file = File::create(&file_name)?;
            file.write_all(&content.as_bytes())
        }

        fn _serialize(register: &Register) -> Result<String, serde_json::Error> {
            serde_json::to_string(&register)
        }

        let file_name = format!("{}.macro", &name);
        let file_name = Path::new(&file_name);

        match self.get(name) {
            Some(register) => match _serialize(register) {
                Ok(content) => match _write(file_name, content) {
                    Ok(()) => Ok(()),
                    Err(error) => Err(format!("Error on write: {}", error)),
                },
                Err(error) => Err(format!("Serialization failed: {}", error)),
            },
            None => Err(String::from("Register not found.")),
        }
    }

    pub fn push(&mut self, name: char, value: String) {
        if name != '_' {
            if let Some(r) = self.inner.get_mut(&name) {
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

    pub fn inner(&self) -> &HashMap<char, Register> {
        &self.inner
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn remove(&mut self, name: char) -> Option<Register> {
        self.inner.remove(&name)
    }
}
