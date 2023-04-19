use core::fmt::Debug;
use std::collections::HashMap;

pub trait Register: Debug {
    fn read(&self) -> &[String];
    fn write(&mut self, values: Vec<String>);
}

#[derive(Debug, Default)]
pub struct SimpleRegister {
    values: Vec<String>,
}

impl Register for SimpleRegister {
    fn read(&self) -> &[String] {
        &self.values
    }

    fn write(&mut self, values: Vec<String>) {
        self.values = values;
    }
}

#[derive(Debug, Default)]
pub struct HistoryRegister {
    values: Vec<String>,
}

impl Register for HistoryRegister {
    fn read(&self) -> &[String] {
        &self.values
    }

    fn write(&mut self, values: Vec<String>) {
        self.values.extend(values.into_iter());
    }
}

#[derive(Debug, Default)]
pub struct BlackHoleRegister {
    values: [String; 0],
}

impl Register for BlackHoleRegister {
    fn read(&self) -> &[String] {
        &self.values
    }

    fn write(&mut self, _values: Vec<String>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    const TEST_STRING: [&str; 2] = ["Hello World!", "BuzzFuzz"];

    fn create_and_assign<T: Register + Default>(mut register: T) -> T {
        assert_eq!(register.read().len(), 0);
        register.write(TEST_STRING.into_iter().map(str::to_string).collect());
        register
    }

    #[test]
    fn test_simple_register() {
        let register = create_and_assign(SimpleRegister::default());
        assert_eq!(register.read(), TEST_STRING);
    }
    #[test]
    fn test_history_register() {
        let mut register = create_and_assign(HistoryRegister::default());
        assert_eq!(register.read(), TEST_STRING);
        register.write(vec!["history".to_string()]);
        assert_eq!(register.read().last().unwrap(), "history");
    }
    #[test]
    fn test_black_hole_register() {
        let register = create_and_assign(BlackHoleRegister::default());
        assert_eq!(register.read().len(), 0);
    }
}

#[derive(Debug, Default)]
pub struct Registers {
    inner: HashMap<char, Box<dyn Register>>,
}

impl Registers {
    pub fn set_register(&mut self, name: char, register: Box<dyn Register>) {
        self.inner.insert(name, register);
    }

    pub fn get(&self, name: char) -> Option<&Box<dyn Register>> {
        self.inner.get(&name)
    }

    pub fn read(&self, name: char) -> Option<&[String]> {
        self.get(name).map(|reg| reg.read())
    }

    pub fn write(&mut self, name: char, values: Vec<String>) {
        let entry = self
            .inner
            .entry(name)
            .or_insert(Box::new(SimpleRegister::default()));
        entry.write(values);
    }

    pub fn first(&self, name: char) -> Option<&String> {
        self.read(name).and_then(|entries| entries.first())
    }

    pub fn last(&self, name: char) -> Option<&String> {
        self.read(name).and_then(|entries| entries.last())
    }

    pub fn inner(&self) -> &HashMap<char, Box<dyn Register>> {
        &self.inner
    }
}
