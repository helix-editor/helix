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

    pub fn load(&mut self, name: char) {
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

        println!("Loading");

        // todo: log results
        match _read_data(name) {
            Ok(content) => match _string_to_register(content) {
                Ok(register) => {
                    println!("INSERTED");
                    self.inner.insert(name, register)
                }
                Err(_error) => None,
            },
            Err(_error) => None,
        };
    }

    pub fn save(&mut self, name: char) {
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
                    Ok(_) => (),
                    Err(_) => (),
                },
                Err(_) => todo!(),
            },
            None => (),
        }
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
