/// Register selection and configuration
///
/// This is a kind a of specialized `Option<char>` for register selection.
/// Point is to keep whether the register selection has been explicitely
/// set or not while being convenient by knowing the default register name.
#[derive(Debug)]
pub struct RegisterSelection {
    selected: char,
    default_name: char,
}

impl RegisterSelection {
    pub fn new(default_name: char) -> Self {
        Self {
            selected: default_name,
            default_name,
        }
    }

    pub fn select(&mut self, name: char) {
        self.selected = name;
    }

    pub fn take(&mut self) -> Self {
        Self {
            selected: std::mem::replace(&mut self.selected, self.default_name),
            default_name: self.default_name,
        }
    }

    pub fn is_default(&self) -> bool {
        self.selected == self.default_name
    }

    pub fn name(&self) -> char {
        self.selected
    }
}

impl Default for RegisterSelection {
    fn default() -> Self {
        let default_name = '"';
        Self {
            selected: default_name,
            default_name,
        }
    }
}
