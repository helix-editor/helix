use self::context_register::{context_register_read, context_register_write, CONTEXT_REGISTERS};
use crate::{info::Info, register::Register, Editor};
use std::borrow::Cow;

pub mod context_register;

// NOTE: Picking from history currently only supported for "normal" registers.
pub trait EditorRegisters {
    // Write:
    fn register_push_values(&mut self, register: Register, values: Vec<String>);
    fn register_push_value(&mut self, register: Register, value: String);
    // Read:
    fn register_newest_values(&self, register: &Register) -> Option<Cow<[String]>>;
    fn register_newest_value(&self, register: &Register) -> Option<Cow<str>>;
    fn register_values(&self, register: &Register) -> Option<Cow<[String]>>;
    // Delete:
    fn register_clear(&mut self);
    fn register_remove(&mut self, register: &Register);
    // Meta:
    fn register_size(&self, register: &Register) -> Option<usize>;
    fn register_select_newest(&mut self, register: &Register, index: usize) -> Result<(), usize>;
}

impl EditorRegisters for Editor {
    fn register_push_values(&mut self, register: Register, values: Vec<String>) {
        if CONTEXT_REGISTERS.contains(&register) {
            return context_register_write(self, &register, values);
        }

        self.registers.push_values(register, values)
    }

    fn register_push_value(&mut self, register: Register, value: String) {
        if CONTEXT_REGISTERS.contains(&register) {
            return context_register_write(self, &register, vec![value]);
        }

        self.registers.push_singular(register, value)
    }

    fn register_newest_values(&self, register: &Register) -> Option<Cow<[String]>> {
        if CONTEXT_REGISTERS.contains(register) {
            Some(context_register_read(self, register))
        } else {
            self.registers
                .newest_values(register)
                .map(|slice| slice.into())
        }
    }

    fn register_newest_value(&self, register: &Register) -> Option<Cow<str>> {
        if CONTEXT_REGISTERS.contains(register) {
            context_register_read(self, register)
                .first()
                .map(|value| value.to_owned().into())
        } else {
            self.registers
                .newest_value(register)
                .map(|value| value.into())
        }
    }

    fn register_values(&self, register: &Register) -> Option<Cow<[String]>> {
        if CONTEXT_REGISTERS.contains(register) {
            Some(context_register_read(self, register))
        } else {
            self.registers.values(register).map(|values| values.into())
        }
    }

    fn register_clear(&mut self) {
        self.registers.clear()
    }

    fn register_remove(&mut self, register: &Register) {
        let status_message: String;

        if CONTEXT_REGISTERS.contains(register) {
            status_message = format!("Context register {} can't be removed.", register)
        } else {
            match self.registers.remove(register) {
                true => status_message = format!("Register {} removed", register),
                false => status_message = format!("Register {} not found", register),
            }
        }

        self.set_status(status_message);
    }

    fn register_size(&self, register: &Register) -> Option<usize> {
        if CONTEXT_REGISTERS.contains(register) {
            Some(context_register_read(self, register).len())
        } else {
            self.registers.size(register)
        }
    }

    fn register_select_newest(&mut self, register: &Register, index: usize) -> Result<(), usize> {
        assert!(!CONTEXT_REGISTERS.contains(register));
        self.registers.set_newest(register, index)
    }
}

pub trait EditorRegisterDisplay {
    fn registers_newest_values_display(&self) -> Vec<(String, String)>;
    fn registers_newest_values_info(&self) -> Info;
    fn registers_listed_info(&self) -> Info;
    fn registers_history_info(&self, register: &Register) -> Info;
}

impl EditorRegisterDisplay for Editor {
    fn registers_newest_values_display(&self) -> Vec<(String, String)> {
        let mut registers_display = self.registers.newest_values_display();
        for context_register in CONTEXT_REGISTERS {
            registers_display.push((
                context_register.to_string(),
                context_register_read(self, &context_register)
                    .first()
                    .map(|value| value.to_owned())
                    .unwrap_or_default(),
            ))
        }

        registers_display
    }

    fn registers_newest_values_info(&self) -> Info {
        prepare_infobox("Registers", self.registers_newest_values_display())
    }

    fn registers_listed_info(&self) -> Info {
        prepare_infobox("Select register", self.registers.listed_info_body())
    }

    fn registers_history_info(&self, register: &Register) -> Info {
        prepare_infobox(
            format!("Register: {}", register),
            self.registers.register_history_info_body(register),
        )
    }
}

fn prepare_infobox<S: AsRef<str>>(title: S, body: Vec<(String, String)>) -> Info {
    let body: Vec<(String, String)> = body
        .into_iter()
        .map(|(key, value)| {
            let mut line_iter = value.lines();
            let Some(first_line) = line_iter.next() else {
                    return (key, String::new())
                };

            let mut truncated_value: Option<Cow<str>> = None;

            const MAX_WIDTH: usize = 30;
            if first_line.chars().count() > MAX_WIDTH {
                truncated_value = Some(
                    first_line
                        .chars()
                        .take(MAX_WIDTH)
                        .collect::<String>()
                        .into(),
                )
            } else if line_iter.next().is_some() {
                truncated_value = Some(first_line.into())
            }

            if let Some(truncated_value) = truncated_value {
                return (key, format!("{}...", truncated_value));
            }

            (key, first_line.to_string())
        })
        .collect();

    Info::new(title.as_ref(), &body)
}
