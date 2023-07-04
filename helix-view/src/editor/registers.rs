use self::context_register::{
    context_register_read, context_register_write, CONTEXT_REGISTERS, WRITABLE_CONTEXT_REGISTERS,
};

use crate::{register::Register, Editor};
use std::borrow::Cow;

pub mod context_register;

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
}

impl EditorRegisters for Editor {
    fn register_push_values(&mut self, register: Register, values: Vec<String>) {
        if WRITABLE_CONTEXT_REGISTERS.contains(&register) {
            context_register_write(self, &register, values)
        } else {
            self.registers.push_values(register, values)
        }
    }

    fn register_push_value(&mut self, register: Register, value: String) {
        if WRITABLE_CONTEXT_REGISTERS.contains(&register) {
            context_register_write(self, &register, vec![value])
        } else {
            self.registers.push_singular(register, value)
        }
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
}
