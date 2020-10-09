use std::string::String;

pub struct Prompt {
    pub buffer: String,
}

impl Prompt {
    pub fn new() -> Prompt {
        let prompt = Prompt {
            buffer: String::from(""),
        };
        prompt
    }

    pub fn insert_char(&mut self, c: char) {
        self.buffer.push(c);
    }
}
