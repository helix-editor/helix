use crate::document::Document;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::fmt;

struct Readonly(String, Box<dyn Fn(&Document) -> Option<String>>);

impl fmt::Debug for Readonly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Readonly").field(&self.0).finish()
    }
}

#[derive(Debug)]
enum Register {
    Static(Vec<String>),
    Readonly(Readonly),
}

/// Currently just wraps a `BTreeMap` of `Register`s
#[derive(Debug)]
pub struct Registers {
    inner: BTreeMap<char, Register>,
}

impl Registers {
    pub fn read(&self, doc: &Document, name: char) -> Option<Vec<String>> {
        self.inner.get(&name).and_then(|reg| match reg {
            Register::Static(content) => Some(content.clone()),
            Register::Readonly(Readonly(_, func)) => func(doc).map(|content| vec![content]),
        })
    }

    pub fn write(&mut self, name: char, values: Vec<String>) {
        match self.inner.entry(name) {
            Entry::Vacant(v) => {
                v.insert(Register::Static(values));
            }
            Entry::Occupied(mut o) => {
                let v = o.get_mut();
                match v {
                    Register::Static(_) => *v = Register::Static(values),
                    Register::Readonly(_) => {}
                }
            }
        }
    }

    pub fn clear(&mut self, name: char) {
        self.inner.remove(&name);
    }

    pub fn push(&mut self, name: char, value: String) {
        if name != '_' {
            if let Some(r) = self.inner.get_mut(&name) {
                match r {
                    Register::Static(content) => content.push(value),
                    Register::Readonly(_) => {}
                }
            } else {
                self.write(name, vec![value]);
            }
        }
    }

    pub fn first(&self, doc: &Document, name: char) -> Option<String> {
        self.inner.get(&name).and_then(|reg| match reg {
            Register::Static(content) => content.first().cloned(),
            Register::Readonly(Readonly(_, func)) => func(doc),
        })
    }

    pub fn last(&self, doc: &Document, name: char) -> Option<String> {
        self.inner.get(&name).and_then(|reg| match reg {
            Register::Static(content) => content.last().cloned(),
            Register::Readonly(Readonly(_, func)) => func(doc),
        })
    }

    pub fn iter_preview(&self) -> impl Iterator<Item = (char, &str)> {
        self.inner.iter().map(|(&ch, reg)| {
            let preview = match reg {
                Register::Static(content) => content
                    .get(0)
                    .and_then(|s| s.lines().next())
                    .unwrap_or_default(),
                Register::Readonly(Readonly(name, _)) => name,
            };
            (ch, preview)
        })
    }
}

impl Default for Registers {
    fn default() -> Registers {
        let mut inner = BTreeMap::new();
        inner.insert(
            '_',
            Register::Readonly(Readonly("null register".to_owned(), Box::new(|_doc| None))),
        );
        inner.insert(
            '%',
            Register::Readonly(Readonly(
                "buffer name".to_owned(),
                Box::new(|doc| Some(doc.display_name().to_string())),
            )),
        );
        Registers { inner }
    }
}
