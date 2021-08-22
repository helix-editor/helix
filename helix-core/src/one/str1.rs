use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Str1<T>(T);

impl<T: Deref<Target = str>> Deref for Str1<T> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Deref<Target = str> + DerefMut> DerefMut for Str1<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> AsRef<T> for Str1<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for Str1<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> From<T> for Str1<T> {
    fn from(t: T) -> Self {
        Str1(t)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Str1View<'a>(pub char, pub &'a str);

impl<T: AsRef<str>> Str1<T> {
    pub fn new(t: T) -> Option<Str1<T>> {
        if t.as_ref().is_empty() {
            None
        } else {
            Some(Str1(t))
        }
    }

    pub fn view(&self) -> Str1View<'_> {
        let mut chars = self.0.as_ref().chars();
        let head = chars.next().unwrap();
        let tail = chars.as_str();
        Str1View(head, tail)
    }

    pub fn head(&self) -> char {
        self.0.as_ref().chars().next().unwrap()
    }

    pub fn tail(&self) -> char {
        self.0.as_ref().chars().rev().next().unwrap()
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

pub fn str1<T, U: From<T>>(s: T) -> Str1<U> {
    Str1(s.into())
}

#[cfg(test)]
mod tests {
    use crate::Tendril1;

    use super::*;

    #[test]
    #[allow(unused_variables)]
    fn smoke() {
        use crate::Tendril;

        let s = Str1::new("interesting").unwrap().chars();
        let s = Str1::new(Tendril::from("wow")).unwrap();

        fn hello(s: Tendril1) {}

        let s: &str = "hello";
        hello(str1(s));
        hello(str1("interesting"));
        hello(Str1::new("hello".into()).unwrap());
    }
}
