use std::collections::HashMap;

#[derive(Debug)]
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
#[derive(Debug, Default)]
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
