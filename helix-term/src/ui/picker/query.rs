use std::{collections::HashMap, mem, ops::Range, sync::Arc};

#[derive(Debug)]
pub(super) struct PickerQuery {
    /// The column names of the picker.
    column_names: Box<[Arc<str>]>,
    /// The index of the primary column in `column_names`.
    /// The primary column is selected by default unless another
    /// field is specified explicitly with `%fieldname`.
    primary_column: usize,
    /// The mapping between column names and input in the query
    /// for those columns.
    inner: HashMap<Arc<str>, Arc<str>>,
    /// The byte ranges of the input text which are used as input for each column.
    /// This is calculated at parsing time for use in [Self::active_column].
    /// This Vec is naturally sorted in ascending order and ranges do not overlap.
    column_ranges: Vec<(Range<usize>, Option<Arc<str>>)>,
}

impl PartialEq<HashMap<Arc<str>, Arc<str>>> for PickerQuery {
    fn eq(&self, other: &HashMap<Arc<str>, Arc<str>>) -> bool {
        self.inner.eq(other)
    }
}

impl PickerQuery {
    pub(super) fn new<I: Iterator<Item = Arc<str>>>(
        column_names: I,
        primary_column: usize,
    ) -> Self {
        let column_names: Box<[_]> = column_names.collect();
        let inner = HashMap::with_capacity(column_names.len());
        let column_ranges = vec![(0..usize::MAX, Some(column_names[primary_column].clone()))];
        Self {
            column_names,
            primary_column,
            inner,
            column_ranges,
        }
    }

    pub(super) fn get(&self, column: &str) -> Option<&Arc<str>> {
        self.inner.get(column)
    }

    pub(super) fn parse(&mut self, input: &str) -> HashMap<Arc<str>, Arc<str>> {
        let mut fields: HashMap<Arc<str>, String> = HashMap::new();
        let primary_field = &self.column_names[self.primary_column];
        let mut escaped = false;
        let mut in_field = false;
        let mut field = None;
        let mut text = String::new();
        self.column_ranges.clear();
        self.column_ranges
            .push((0..usize::MAX, Some(primary_field.clone())));

        macro_rules! finish_field {
            () => {
                let key = field.take().unwrap_or(primary_field);

                // Trims one space from the end, enabling leading and trailing
                // spaces in search patterns, while also retaining spaces as separators
                // between column filters.
                let pat = text.strip_suffix(' ').unwrap_or(&text);

                if let Some(pattern) = fields.get_mut(key) {
                    pattern.push(' ');
                    pattern.push_str(pat);
                } else {
                    fields.insert(key.clone(), pat.to_string());
                }
                text.clear();
            };
        }

        for (idx, ch) in input.char_indices() {
            match ch {
                // Backslash escaping
                _ if escaped => {
                    // '%' is the only character that is special cased.
                    // You can escape it to prevent parsing the text that
                    // follows it as a field name.
                    if ch != '%' {
                        text.push('\\');
                    }
                    text.push(ch);
                    escaped = false;
                }
                '\\' => escaped = !escaped,
                '%' => {
                    if !text.is_empty() {
                        finish_field!();
                    }
                    let (range, _field) = self
                        .column_ranges
                        .last_mut()
                        .expect("column_ranges is non-empty");
                    range.end = idx;
                    in_field = true;
                }
                ' ' if in_field => {
                    text.clear();
                    in_field = false;
                }
                _ if in_field => {
                    text.push(ch);
                    // Go over all columns and their indices, find all that starts with field key,
                    // select a column that fits key the most.
                    field = self
                        .column_names
                        .iter()
                        .filter(|col| col.starts_with(&text))
                        // select "fittest" column
                        .min_by_key(|col| col.len());

                    // Update the column range for this column.
                    if let Some((_range, current_field)) = self
                        .column_ranges
                        .last_mut()
                        .filter(|(range, _)| range.end == usize::MAX)
                    {
                        *current_field = field.cloned();
                    } else {
                        self.column_ranges.push((idx..usize::MAX, field.cloned()));
                    }
                }
                _ => text.push(ch),
            }
        }

        if !in_field && !text.is_empty() {
            finish_field!();
        }

        let new_inner: HashMap<_, _> = fields
            .into_iter()
            .map(|(field, query)| (field, query.as_str().into()))
            .collect();

        mem::replace(&mut self.inner, new_inner)
    }

    /// Finds the column which the cursor is 'within' in the last parse.
    ///
    /// The cursor is considered to be within a column when it is placed within any
    /// of a column's text. See the `active_column_test` unit test below for examples.
    ///
    /// `cursor` is a byte index that represents the location of the prompt's cursor.
    pub fn active_column(&self, cursor: usize) -> Option<&Arc<str>> {
        let point = self
            .column_ranges
            .partition_point(|(range, _field)| cursor > range.end);

        self.column_ranges
            .get(point)
            .filter(|(range, _field)| cursor >= range.start && cursor <= range.end)
            .and_then(|(_range, field)| field.as_ref())
    }
}

#[cfg(test)]
mod test {
    use helix_core::hashmap;

    use super::*;

    #[test]
    fn parse_query_test() {
        let mut query = PickerQuery::new(
            [
                "primary".into(),
                "field1".into(),
                "field2".into(),
                "another".into(),
                "anode".into(),
            ]
            .into_iter(),
            0,
        );

        // Basic field splitting
        query.parse("hello world");
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => "hello world".into(),
            )
        );
        query.parse("hello %field1 world %field2 !");
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => "hello".into(),
                "field1".into() => "world".into(),
                "field2".into() => "!".into(),
            )
        );
        query.parse("%field1 abc %field2 def xyz");
        assert_eq!(
            query,
            hashmap!(
                "field1".into() => "abc".into(),
                "field2".into() => "def xyz".into(),
            )
        );

        // Trailing space is trimmed
        query.parse("hello ");
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => "hello".into(),
            )
        );

        // Unknown fields are trimmed.
        query.parse("hello %foo");
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => "hello".into(),
            )
        );

        // Multiple words in a field
        query.parse("hello %field1 a b c");
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => "hello".into(),
                "field1".into() => "a b c".into(),
            )
        );

        // Escaping
        query.parse(r#"hello\ world"#);
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => r#"hello\ world"#.into(),
            )
        );
        query.parse(r#"hello \%field1 world"#);
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => "hello %field1 world".into(),
            )
        );
        query.parse(r#"%field1 hello\ world"#);
        assert_eq!(
            query,
            hashmap!(
                "field1".into() => r#"hello\ world"#.into(),
            )
        );
        query.parse(r#"hello %field1 a\"b"#);
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => "hello".into(),
                "field1".into() => r#"a\"b"#.into(),
            )
        );
        query.parse(r#"%field1 hello\ world"#);
        assert_eq!(
            query,
            hashmap!(
                "field1".into() => r#"hello\ world"#.into(),
            )
        );
        query.parse(r#"\bfoo\b"#);
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => r#"\bfoo\b"#.into(),
            )
        );
        query.parse(r#"\\n"#);
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => r#"\\n"#.into(),
            )
        );

        // Only the prefix of a field is required.
        query.parse("hello %anot abc");
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => "hello".into(),
                "another".into() => "abc".into(),
            )
        );
        // The shortest matching the prefix is selected.
        query.parse("hello %ano abc");
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => "hello".into(),
                "anode".into() => "abc".into()
            )
        );
        // Multiple uses of a column are concatenated with space separators.
        query.parse("hello %field1 xyz %fie abc");
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => "hello".into(),
                "field1".into() => "xyz abc".into()
            )
        );
        query.parse("hello %fie abc");
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => "hello".into(),
                "field1".into() => "abc".into()
            )
        );
        // The primary column can be explicitly qualified.
        query.parse("hello %fie abc %prim world");
        assert_eq!(
            query,
            hashmap!(
                "primary".into() => "hello world".into(),
                "field1".into() => "abc".into()
            )
        );
    }

    #[test]
    fn active_column_test() {
        fn active_column<'a>(query: &'a mut PickerQuery, input: &str) -> Option<&'a str> {
            let cursor = input.find('|').expect("cursor must be indicated with '|'");
            let input = input.replace('|', "");
            query.parse(&input);
            query.active_column(cursor).map(AsRef::as_ref)
        }

        let mut query = PickerQuery::new(
            ["primary".into(), "foo".into(), "bar".into()].into_iter(),
            0,
        );

        assert_eq!(active_column(&mut query, "|"), Some("primary"));
        assert_eq!(active_column(&mut query, "hello| world"), Some("primary"));
        assert_eq!(active_column(&mut query, "|%foo hello"), Some("primary"));
        assert_eq!(active_column(&mut query, "%foo|"), Some("foo"));
        assert_eq!(active_column(&mut query, "%|"), None);
        assert_eq!(active_column(&mut query, "%baz|"), None);
        assert_eq!(active_column(&mut query, "%quiz%|"), None);
        assert_eq!(active_column(&mut query, "%foo hello| world"), Some("foo"));
        assert_eq!(active_column(&mut query, "%foo hello world|"), Some("foo"));
        assert_eq!(active_column(&mut query, "%foo| hello world"), Some("foo"));
        assert_eq!(active_column(&mut query, "%|foo hello world"), Some("foo"));
        assert_eq!(active_column(&mut query, "%f|oo hello world"), Some("foo"));
        assert_eq!(active_column(&mut query, "hello %f|oo world"), Some("foo"));
        assert_eq!(
            active_column(&mut query, "hello %f|oo world %bar !"),
            Some("foo")
        );
        assert_eq!(
            active_column(&mut query, "hello %foo wo|rld %bar !"),
            Some("foo")
        );
        assert_eq!(
            active_column(&mut query, "hello %foo world %bar !|"),
            Some("bar")
        );
    }
}
