#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlameLine {
    author: String,
    timestamp: String,
}

impl BlameLine {
    pub fn new(author: String, timestamp: String) -> Self {
        Self { author, timestamp }
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }
}
