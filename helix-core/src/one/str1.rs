use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Str1<T>(T);

impl<T> Deref for Str1<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Str1<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0 
    }
}

impl<T: AsRef<str>> Str1<T> {
    pub fn new(t: T) -> Option<Str1<T>> {
        if t.as_ref().is_empty() {
            None
        } else {
            Some(Str1(t))
        }
    }

    pub fn head(&self) -> char {
        self.0.as_ref().chars().next().unwrap()
    }

    pub fn tail(&self) -> char {
        self.0.as_ref().chars().rev().next().unwrap()
    }

    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}