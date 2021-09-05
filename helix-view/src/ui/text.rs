/// Displays raw text, used as building block for other components.
pub struct Text {
    pub contents: String,
}

impl Text {
    pub fn new(contents: String) -> Self {
        Self { contents }
    }
}
