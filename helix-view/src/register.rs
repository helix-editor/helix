use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    iter,
};

use anyhow::Result;
use helix_core::NATIVE_LINE_ENDING;

use crate::{
    clipboard::{get_clipboard_provider, ClipboardProvider, ClipboardType},
    document::SCRATCH_BUFFER_NAME,
    Editor,
};

/// A key-value store for saving sets of values.
///
/// Each register corresponds to a `char`. Most chars can be used to store any set of
/// values but a few chars are "special registers". Special registers have unique
/// behaviors when read or written to:
///
/// * Black hole (`_`): all values read and written are discarded
/// * Selection indices (`#`): index number of each selection starting at 1
/// * Selection contents (`.`)
/// * Document path (`%`): filename of the current buffer
/// * System clipboard (`*`)
/// * Primary clipboard (`+`)
#[derive(Debug)]
pub struct Registers {
    /// The mapping of register to values.
    /// Values are stored in reverse order when inserted with `Registers::write`.
    /// The order is reversed again in `Registers::read`. This allows us to
    /// efficiently prepend new values in `Registers::push`.
    inner: HashMap<char, Register>,
    clipboard_provider: Box<dyn ClipboardProvider>,
    pub last_search_register: char,
}

impl Default for Registers {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            clipboard_provider: get_clipboard_provider(),
            last_search_register: '/',
        }
    }
}

impl Registers {
    pub fn read<'a>(&'a self, name: char, editor: &'a Editor) -> Option<RegisterValues<'a>> {
        match name {
            '_' => Some(RegisterValues::new(iter::empty())),
            '#' => {
                let (view, doc) = current_ref!(editor);
                let selections = doc.selection(view.id).len();
                // ExactSizeIterator is implemented for Range<usize> but
                // not RangeInclusive<usize>.
                Some(RegisterValues::new(
                    (0..selections).map(|i| (i + 1).to_string().into()),
                ))
            }
            '.' => {
                let (view, doc) = current_ref!(editor);
                let text = doc.text().slice(..);
                Some(RegisterValues::new(doc.selection(view.id).fragments(text)))
            }
            '%' => {
                let doc = doc!(editor);

                let path = doc
                    .path()
                    .as_ref()
                    .map(|p| p.to_string_lossy())
                    .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into());

                Some(RegisterValues::new(iter::once(path)))
            }
            '*' | '+' => {
                let Some(register) = self.inner.get(&name) else { return None; };
                Some(register.read_from_clipboard(
                    self.clipboard_provider.as_ref(),
                    match name {
                        '+' => ClipboardType::Clipboard,
                        '*' => ClipboardType::Selection,
                        _ => unreachable!(),
                    },
                ))
            }
            _ => self.inner.get(&name).map(|register| register.value()),
        }
    }

    pub fn read_nth(&self, name: char, index: usize) -> Result<Option<RegisterValues<'_>>> {
        match name {
            '_' | '#' | '.' | '%' | '*' | '+' => Err(anyhow::anyhow!(
                "Register {name} does not allow indexed access"
            )),
            _ => Ok(self
                .inner
                .get(&name)
                .and_then(|register| register.nth(index))),
        }
    }

    pub fn push_many(&mut self, name: char, values: &Vec<String>) -> Result<()> {
        match name {
            '_' => Ok(()),
            '#' | '.' | '%' => Err(anyhow::anyhow!("Register {name} does not support writing")),
            _ => {
                if name == '*' || name == '+' {
                    self.clipboard_provider.set_contents(
                        values.join(NATIVE_LINE_ENDING.as_str()),
                        match name {
                            '+' => ClipboardType::Clipboard,
                            '*' => ClipboardType::Selection,
                            _ => unreachable!(),
                        },
                    )?;
                }
                let register = self.inner.entry(name).or_insert_with(Register::default);
                register.write(values);
                Ok(())
            }
        }
    }

    pub fn push(&mut self, name: char, value: &str) -> Result<()> {
        match name {
            '_' => Ok(()),
            '#' | '.' | '%' => Err(anyhow::anyhow!("Register {name} does not support pushing")),
            '*' | '+' => {
                let register = self.inner.entry(name).or_insert_with(Register::default);
                register
                    .save_clipboard_contents(
                        &mut self.clipboard_provider,
                        match name {
                            '+' => ClipboardType::Clipboard,
                            '*' => ClipboardType::Selection,
                            _ => unreachable!(),
                        },
                        value,
                    )
                    .map_err(|err| anyhow::anyhow!("Failed to push to register {name}: {err}"))
            }
            _ => {
                let register = self.inner.entry(name).or_insert_with(Register::default);
                register.push(value);
                Ok(())
            }
        }
    }

    pub fn first<'a>(&'a self, name: char, editor: &'a Editor) -> Option<Cow<'a, str>> {
        self.read(name, editor).and_then(|mut values| values.next())
    }

    pub fn iter_preview(&self) -> impl Iterator<Item = (char, String)> + '_ {
        self.inner
            .iter()
            .filter(|(name, _)| !matches!(name, '*' | '+'))
            .map(|(name, register)| {
                let preview = register
                    .value()
                    .next()
                    .and_then(|s| s.lines().next().map(String::from))
                    .unwrap_or("<empty>".to_string());

                (*name, preview)
            })
            .chain(
                [
                    ('_', "<empty>"),
                    ('#', "<selection indices>"),
                    ('.', "<selection contents>"),
                    ('%', "<document path>"),
                    ('+', "<system clipboard>"),
                    ('*', "<primary clipboard>"),
                ]
                .into_iter()
                .map(|(c, s)| (c, s.to_string())),
            )
    }

    pub fn clear(&mut self) {
        self.clear_clipboard(ClipboardType::Clipboard);
        self.clear_clipboard(ClipboardType::Selection);
        self.inner.clear()
    }

    pub fn remove(&mut self, name: char) -> bool {
        match name {
            '*' | '+' => {
                self.clear_clipboard(match name {
                    '+' => ClipboardType::Clipboard,
                    '*' => ClipboardType::Selection,
                    _ => unreachable!(),
                });
                self.inner.remove(&name);

                true
            }
            '_' | '#' | '.' | '%' => false,
            _ => self.inner.remove(&name).is_some(),
        }
    }

    fn clear_clipboard(&mut self, clipboard_type: ClipboardType) {
        if let Err(err) = self
            .clipboard_provider
            .set_contents("".into(), clipboard_type)
        {
            log::error!(
                "Failed to clear {} clipboard: {err}",
                match clipboard_type {
                    ClipboardType::Clipboard => "system",
                    ClipboardType::Selection => "primary",
                }
            )
        }
    }

    pub fn clipboard_provider_name(&self) -> Cow<str> {
        self.clipboard_provider.name()
    }

    pub fn preview_for(&self, name: char) -> Vec<(String, String)> {
        match name {
            '*' | '+' | '_' | '#' | '.' | '%' => Vec::default(),
            _ => self
                .inner
                .get(&name)
                .map_or_else(Vec::default, |register| register.preview()),
        }
    }
}

/// Register contents are stored as Strings in a Vec that is treated like a queue.
/// New elements are pushed on and pulled off from the end of the Vec for
/// performance and drained from the front.
#[derive(Default, Debug)]
struct Register {
    contents: Vec<String>,
    lengths: VecDeque<usize>,
}

impl Register {
    fn iter(&self) -> RegisterIterator {
        RegisterIterator::new(self)
    }

    fn value(&self) -> RegisterValues<'_> {
        self.iter().next().map_or_else(
            || RegisterValues::new(iter::empty()),
            |items| RegisterValues::new(items.into_iter()),
        )
    }

    fn nth(&self, index: usize) -> Option<RegisterValues<'_>> {
        let index = index.min(self.lengths.len().saturating_sub(1));
        let mut it = self.iter();
        it.nth(index)
            .map(|items| RegisterValues::new(items.into_iter()))
    }

    fn pop_oldest(&mut self) -> Option<RegisterValues<'_>> {
        match self.lengths.pop_front() {
            Some(oldest_length) => Some(RegisterValues::new(
                self.contents.drain(0..oldest_length).rev().map(Cow::from),
            )),
            None => {
                if !self.contents.is_empty() {
                    log::debug!("Register has no lengths but some contents!!");
                }
                None
            }
        }
    }

    // TODO: expose this as something like registers.pop() -> Option<RegisterValues>
    // to then be exposed by a command.
    #[allow(dead_code)]
    fn pop_recent(&mut self) -> Option<RegisterValues<'_>> {
        match self.lengths.pop_back() {
            Some(recent_length) => {
                let end = self.contents.len();
                let start = end - recent_length;

                Some(RegisterValues::new(
                    self.contents.drain(start..end).rev().map(Cow::from),
                ))
            }
            None => {
                if !self.contents.is_empty() {
                    log::debug!("Register has no lengths but some contents!!");
                }
                None
            }
        }
    }

    fn write(&mut self, values: &Vec<String>) {
        if self.lengths.len() >= 16 {
            // Limit the number of captured elements to 16.
            self.pop_oldest();
        }
        self.contents.extend(values.iter().rev().cloned());
        self.lengths.push_back(values.len());
    }

    fn push(&mut self, value: &str) {
        if self.lengths.len() >= 16 {
            // Limit the number of captured elements to 16.
            self.pop_oldest();
        }
        self.contents.push(value.to_string());
        self.lengths.push_back(1);
    }

    fn read_from_clipboard<'a>(
        &'a self,
        provider: &dyn ClipboardProvider,
        clipboard_type: ClipboardType,
    ) -> RegisterValues<'a> {
        match provider.get_contents(clipboard_type) {
            Ok(contents) => {
                // If we're pasting the same values that we just yanked, re-use
                // the saved values. This allows pasting multiple selections
                // even when yanked to a clipboard.

                if contents_are_saved(&self.contents, &contents) {
                    return self.value();
                }
                RegisterValues::new(iter::once(contents.into()))
            }
            Err(err) => {
                log::error!(
                    "Failed to read {} clipboard: {err}",
                    match clipboard_type {
                        ClipboardType::Clipboard => "system",
                        ClipboardType::Selection => "primary",
                    }
                );

                RegisterValues::new(iter::empty())
            }
        }
    }

    fn save_clipboard_contents(
        &mut self,
        clipboard_provider: &mut Box<dyn ClipboardProvider>,
        clipboard_type: ClipboardType,
        value: &str,
    ) -> Result<()> {
        let contents = clipboard_provider.get_contents(clipboard_type)?;

        if contents_are_saved(&self.contents, &contents) {
            let mut value = value.to_string();
            self.push(&value);
            if !contents.is_empty() {
                value.push_str(NATIVE_LINE_ENDING.as_str());
            }
            value.push_str(&contents);

            clipboard_provider.set_contents(value, clipboard_type)?;
        }
        Ok(())
    }

    /// Return the first string of each element of contents.
    fn preview(&self) -> Vec<(String, String)> {
        if self.lengths.is_empty() {
            return vec![];
        }

        // Yes, this could use the iter() definition. However, that builds
        // vectors we don't need. Is this worth the maintainence? Not sure.
        let mut previous = 0;
        self.lengths
            .iter()
            .rev()
            .enumerate()
            .map(|(index, count)| {
                let mut remaining = self.contents.iter().rev().skip(previous);
                previous += count;
                // As with the comment in next(), these unwraps are safe
                // because each is guaranteed to have a value.
                let first_value = remaining.next().unwrap();
                let line = first_value.lines().next().unwrap();
                // Safe unwrap. Can never be more than 16 values.
                let ch = char::from_digit(index as u32, 16).unwrap();
                (ch.to_string(), String::from(line))
            })
            .collect()
    }
}

fn contents_are_saved(saved_values: &[String], mut contents: &str) -> bool {
    let line_ending = NATIVE_LINE_ENDING.as_str();
    let mut values = saved_values.iter().rev();

    match values.next() {
        Some(first) if contents.starts_with(first) => {
            contents = &contents[first.len()..];
        }
        None if contents.is_empty() => return true,
        _ => return false,
    }

    for value in values {
        if contents.starts_with(line_ending) && contents[line_ending.len()..].starts_with(value) {
            contents = &contents[line_ending.len() + value.len()..];
        } else {
            return false;
        }
    }

    true
}

/// Iterator to walk the register contents.
///
/// index is the most recently accessed element.
/// previous is the total number of strings read from contents so far. This is
/// used to find the next one.
struct RegisterIterator<'a> {
    register: &'a Register,
    index: usize,
    previous: usize,
}

impl<'a> RegisterIterator<'a> {
    fn new(register: &'a Register) -> Self {
        Self {
            register,
            index: 0,
            previous: 0,
        }
    }
}

impl<'a> Iterator for RegisterIterator<'a> {
    type Item = Vec<Cow<'a, str>>;

    /// The mental model of how this works is:
    ///     let count = self.register.lengths[self.index];
    ///     let values = self.register.contents[self.previous..(self.previous + self.count)];
    /// But this is not how it actually works because the data structure is read
    /// from its tail.
    ///
    /// Retrieving the next element is done by first retrieving the length of
    /// the element from the lengths. This is done by index lookup. Then the
    /// number of previously read strings are skipped. Since an element can be
    /// more than one string the contents cannot be directly accessed. Finally
    /// the number of strings specified from lengths are returned.
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.register.lengths.len() {
            return None;
        }
        let count = self.register.lengths.iter().rev().nth(self.index).unwrap();
        let remaining = self.register.contents.iter().rev().skip(self.previous);
        let values = remaining.take(*count).map(Cow::from).collect();
        self.previous += count;
        self.index += 1;
        Some(values)
    }
}

// This is a wrapper of an iterator that can return either owned or borrowed values.
// Regular registers can return borrowed values while some special registers need
// to return owned values.
pub struct RegisterValues<'a> {
    iter: Box<dyn Iterator<Item = Cow<'a, str>> + 'a>,
}

impl<'a> RegisterValues<'a> {
    fn new(iter: impl Iterator<Item = Cow<'a, str>> + 'a) -> Self {
        Self {
            iter: Box::new(iter),
        }
    }
}

impl<'a> Iterator for RegisterValues<'a> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{self, Rng};

    #[test]
    fn test_all_register_values_are_retrieved_with_iter_preview() {
        let mut registers = Registers::default();
        let mut lines = Vec::new();

        for register in 'a'..='d' {
            let preview_line = format!("This is register {register}");
            let line = format!("{preview_line}\nThis is its second line not seen.");
            assert!(registers.push(register, &line).is_ok());
            lines.push((register, preview_line));
        }

        lines.extend_from_slice(&[
            ('_', String::from("<empty>")),
            ('#', String::from("<selection indices>")),
            ('.', String::from("<selection contents>")),
            ('%', String::from("<document path>")),
            ('+', String::from("<system clipboard>")),
            ('*', String::from("<primary clipboard>")),
        ]);
        let mut values: Vec<_> = registers.iter_preview().collect();
        // Sorting is needed because the HashMap returns arbitrary ordering.
        lines.sort();
        values.sort();
        assert_eq!(values, lines);
    }

    #[test]
    fn test_register_remembers_first_element_pushed() {
        let mut register = Register::default();
        register.push("this is a test");
        let values: Vec<_> = register.value().collect();
        assert_eq!(values, vec!["this is a test"]);
    }

    #[test]
    fn test_register_remembers_second_element_pushed() {
        let mut register = Register::default();
        register.push("this is a test");
        register.push("this is a second test");
        let values: Vec<_> = register.value().collect();
        assert_eq!(values, vec!["this is a second test"]);
        let values: Vec<_> = register.nth(1).unwrap().collect();
        assert_eq!(values, vec!["this is a test"]);
    }

    #[test]
    fn test_register_remembers_first_element_written() {
        let mut register = Register::default();
        let expected = vec![
            "This is the first line.".to_string(),
            "This is the second line.".to_string(),
            "This is the third line".to_string(),
        ];
        register.write(&expected);
        let values: Vec<_> = register.value().collect();
        assert_eq!(values, expected);
    }

    #[test]
    fn test_register_remembers_third_element_written() {
        let mut register = Register::default();
        let first = vec!["Junk line 1.".to_string(), "Junk line 2.".to_string()];
        register.write(&first);
        let second = vec![
            "More Junk line 1.".to_string(),
            "More Junk line 2.".to_string(),
            "More Junk line 3.".to_string(),
            "More Junk line 4.".to_string(),
        ];
        register.write(&second);
        let expected = vec![
            "This is the first line.".to_string(),
            "This is the second line.".to_string(),
            "This is the third line".to_string(),
        ];
        register.write(&expected);
        let values: Vec<_> = register.value().collect();
        assert_eq!(values, expected);
        let first_thing_pushed: Vec<_> = register.nth(2).unwrap().collect();
        assert_eq!(first_thing_pushed, first);
    }

    #[test]
    fn test_register_returns_correct_element_after_removing_most_recent() {
        let mut register = Register::default();
        let first = vec!["Junk line 1.".to_string(), "Junk line 2.".to_string()];
        register.write(&first);
        let second = vec![
            "More Junk line 1.".to_string(),
            "More Junk line 2.".to_string(),
            "More Junk line 3.".to_string(),
            "More Junk line 4.".to_string(),
        ];
        register.write(&second);
        let third = vec![
            "This is the first line.".to_string(),
            "This is the second line.".to_string(),
            "This is the third line".to_string(),
        ];
        register.write(&third);
        let popped: Vec<_> = register.pop_recent().unwrap().collect();
        assert_eq!(popped, third);
        let values: Vec<_> = register.value().collect();
        assert_eq!(values, second);
    }

    #[test]
    fn test_register_remembers_mixed_elements_written() {
        let mut register = Register::default();
        let first = vec!["Junk line 1.".to_string(), "Junk line 2.".to_string()];
        register.write(&first);
        let second = "More Junk line 1.";
        register.push(second);
        let expected = vec![
            "This is the first line.".to_string(),
            "This is the second line.".to_string(),
            "This is the third line".to_string(),
        ];
        register.write(&expected);
        let values: Vec<_> = register.value().collect();
        assert_eq!(values, expected);
        let first_thing_pushed: Vec<_> = register.nth(2).unwrap().collect();
        assert_eq!(first_thing_pushed, first);
        let second_thing_pushed: Vec<_> = register.nth(1).unwrap().collect();
        assert_eq!(second_thing_pushed, vec![second]);
    }

    #[test]
    fn test_register_only_holds_16_elements() {
        let mut register = Register::default();
        for i in 1..=16 {
            let item = format!("This is the {i} thing");
            register.push(&item);
        }
        assert_eq!(register.lengths.len(), 16);
        register.push("This is the last thing");
        assert_eq!(register.lengths.len(), 16);
        let oldest: Vec<_> = register.nth(15).unwrap().collect();
        assert_eq!(oldest, vec!["This is the 2 thing"]);
        let values: Vec<_> = register.value().collect();
        assert_eq!(values, vec!["This is the last thing"]);
    }

    #[test]
    fn test_register_only_holds_16_writes() {
        let mut register = Register::default();
        let mut rng = rand::thread_rng();
        for i in 1..=17 {
            let mut values = vec![format!("This is the {i} thing")];
            let end = rng.gen_range(1..6);
            for n in 0..end {
                let next = format!("This is next {} thing", n * i);
                values.push(next);
            }
            register.write(&values);
        }
        assert_eq!(register.lengths.len(), 16);
        let oldest: Vec<_> = register.nth(15).unwrap().collect();
        let oldest_initial = oldest.first().unwrap();
        assert_eq!(oldest_initial, "This is the 2 thing");
        let values: Vec<_> = register.value().collect();
        assert_eq!(values.len(), *(register.lengths.back().unwrap()));
        assert_eq!(values.first().unwrap(), "This is the 17 thing");
    }

    #[test]
    fn test_register_preview() {
        let mut register = Register::default();
        let mut expected = Vec::new();
        for i in 1..=16 {
            let line = format!("This is the {i} thing");
            let mut values = vec![format!("{}\nFoo", line)];
            let index = char::from_digit(16 - i, 16).unwrap();
            expected.push((index.to_string(), line.clone()));
            // Extra data to demonstrate that only the first line is read.
            for n in 0..3 {
                let next = format!("This is next {} thing", n * i);
                values.push(next);
            }
            register.write(&values);
        }
        // Elements are showed newest first.
        expected.reverse();

        let values: Vec<_> = register.preview();
        assert_eq!(values, expected);
    }
}
