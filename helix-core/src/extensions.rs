#[cfg(feature = "steel")]
pub mod steel_implementations {

    use std::borrow::Cow;

    use steel::{
        gc::ShareableMut,
        rvals::{as_underlying_type, AsRefSteelVal, Custom, SteelString},
        steel_vm::{
            builtin::{BuiltInModule, MarkdownDoc},
            register_fn::RegisterFn,
        },
        SteelVal,
    };

    use helix_stdx::rope::RopeSliceExt;

    use crate::syntax::config::{AutoPairConfig, SoftWrap};

    impl steel::rvals::Custom for crate::Position {}
    impl steel::rvals::Custom for crate::Selection {}
    impl steel::rvals::Custom for AutoPairConfig {}
    impl steel::rvals::Custom for SoftWrap {}

    #[allow(unused)]
    pub struct RopeyError(ropey::Error);

    impl steel::rvals::Custom for RopeyError {}

    impl From<ropey::Error> for RopeyError {
        fn from(value: ropey::Error) -> Self {
            Self(value)
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum RangeKind {
        Char,
        Byte,
    }

    #[derive(Clone, PartialEq, Eq)]
    pub struct SteelRopeSlice {
        text: crate::Rope,
        start: usize,
        end: usize,
        kind: RangeKind,
    }

    impl Custom for SteelRopeSlice {
        // `equal?` on two ropes should return true if they are the same
        fn equality_hint(&self, other: &dyn steel::rvals::CustomType) -> bool {
            if let Some(other) = as_underlying_type::<SteelRopeSlice>(other) {
                self == other
            } else {
                false
            }
        }

        fn equality_hint_general(&self, other: &steel::SteelVal) -> bool {
            match other {
                SteelVal::StringV(s) => self.to_slice() == s.as_str(),
                SteelVal::Custom(c) => Self::equality_hint(&self, c.read().as_ref()),

                _ => false,
            }
        }

        fn fmt(&self) -> Option<std::result::Result<String, std::fmt::Error>> {
            Some(Ok(format!("#<Rope:\"{}\">", self.to_slice())))
        }
    }

    impl SteelRopeSlice {
        pub fn from_string(string: SteelString) -> Self {
            Self {
                text: crate::Rope::from_str(string.as_str()),
                start: 0,
                end: string.len(),
                kind: RangeKind::Char,
            }
        }

        pub fn new(rope: crate::Rope) -> Self {
            let end = rope.len_chars();
            Self {
                text: rope,
                start: 0,
                end,
                kind: RangeKind::Char,
            }
        }

        fn to_slice(&self) -> crate::RopeSlice<'_> {
            match self.kind {
                RangeKind::Char => self.text.slice(self.start..self.end),
                RangeKind::Byte => self.text.byte_slice(self.start..self.end),
            }
        }

        pub fn insert_str(&self, char_idx: usize, text: SteelString) -> Result<Self, RopeyError> {
            let slice = self.to_slice();
            let mut rope = ropey::Rope::from(slice);
            rope.try_insert(char_idx, &text)?;
            Ok(Self::new(rope))
        }

        pub fn insert_char(&self, char_idx: usize, c: char) -> Result<Self, RopeyError> {
            let slice = self.to_slice();
            let mut rope = ropey::Rope::from(slice);
            rope.try_insert_char(char_idx, c)?;
            Ok(Self::new(rope))
        }

        pub fn try_line_to_char(&self, line: usize) -> Result<usize, RopeyError> {
            self.to_slice().try_line_to_char(line).map_err(RopeyError)
        }

        pub fn try_line_to_byte(&self, line: usize) -> Result<usize, RopeyError> {
            self.to_slice().try_line_to_byte(line).map_err(RopeyError)
        }

        pub fn try_char_to_line(&self, line: usize) -> Result<usize, RopeyError> {
            self.to_slice().try_char_to_line(line).map_err(RopeyError)
        }

        pub fn try_byte_to_line(&self, line: usize) -> Result<usize, RopeyError> {
            self.to_slice().try_byte_to_line(line).map_err(RopeyError)
        }

        pub fn line(mut self, cursor: usize) -> Result<Self, RopeyError> {
            match self.kind {
                RangeKind::Char => {
                    let slice = self.text.get_slice(self.start..self.end).ok_or_else(|| {
                        RopeyError(ropey::Error::CharIndexOutOfBounds(self.start, self.end))
                    })?;

                    // Move the start range, to wherever this lines up
                    let index = slice.try_line_to_char(cursor)?;

                    let line = slice.line(cursor);

                    self.start += index;
                    self.end = self.start + line.len_chars();

                    Ok(self)
                }
                RangeKind::Byte => {
                    let slice =
                        self.text
                            .get_byte_slice(self.start..self.end)
                            .ok_or_else(|| {
                                RopeyError(ropey::Error::ByteIndexOutOfBounds(self.start, self.end))
                            })?;

                    // Move the start range, to wherever this lines up
                    let index = slice.try_line_to_byte(cursor)?;
                    let line = slice.line(cursor);

                    self.start += index;
                    self.end = self.start + line.len_bytes();

                    Ok(self)
                }
            }
        }

        pub fn slice(mut self, lower: usize, upper: usize) -> Result<Self, RopeyError> {
            match self.kind {
                RangeKind::Char => {
                    self.end = self.start + upper;
                    self.start += lower;

                    // Just check that this is legal
                    self.text.get_slice(self.start..self.end).ok_or_else(|| {
                        RopeyError(ropey::Error::CharIndexOutOfBounds(self.start, self.end))
                    })?;

                    Ok(self)
                }
                RangeKind::Byte => {
                    self.start = self.text.try_byte_to_char(self.start)? + lower;
                    self.end = self.start + (upper - lower);

                    self.text
                        .get_byte_slice(self.start..self.end)
                        .ok_or_else(|| {
                            RopeyError(ropey::Error::ByteIndexOutOfBounds(self.start, self.end))
                        })?;

                    self.kind = RangeKind::Char;
                    Ok(self)
                }
            }
        }

        pub fn byte_slice(mut self, lower: usize, upper: usize) -> Result<Self, RopeyError> {
            match self.kind {
                RangeKind::Char => {
                    self.start = self.text.try_char_to_byte(self.start)? + lower;
                    self.end = self.start + (upper - lower);
                    self.kind = RangeKind::Byte;

                    // Just check that this is legal
                    self.text.get_slice(self.start..self.end).ok_or_else(|| {
                        RopeyError(ropey::Error::CharIndexOutOfBounds(self.start, self.end))
                    })?;

                    Ok(self)
                }
                RangeKind::Byte => {
                    self.start += lower;
                    self.end = self.start + (upper - lower);

                    self.text
                        .get_byte_slice(self.start..self.end)
                        .ok_or_else(|| {
                            RopeyError(ropey::Error::ByteIndexOutOfBounds(self.start, self.end))
                        })?;

                    Ok(self)
                }
            }
        }

        pub fn char_to_byte(&self, pos: usize) -> Result<usize, RopeyError> {
            Ok(self.to_slice().try_char_to_byte(pos)?)
        }

        pub fn byte_to_char(&self, pos: usize) -> Result<usize, RopeyError> {
            Ok(self.to_slice().try_byte_to_char(pos)?)
        }

        pub fn to_string(&self) -> String {
            self.to_slice().to_string()
        }

        pub fn len_chars(&self) -> usize {
            self.to_slice().len_chars()
        }

        pub fn len_bytes(&self) -> usize {
            self.to_slice().len_bytes()
        }

        pub fn get_char(&self, index: usize) -> Option<char> {
            self.to_slice().get_char(index)
        }

        pub fn len_lines(&self) -> usize {
            self.to_slice().len_lines()
        }

        pub fn trim_start(mut self) -> Self {
            let slice = self.to_slice();

            for (idx, c) in slice.chars().enumerate() {
                if !c.is_whitespace() {
                    match self.kind {
                        RangeKind::Char => {
                            self.start += idx;
                        }
                        RangeKind::Byte => {
                            self.start += slice.char_to_byte(idx);
                        }
                    }

                    break;
                }
            }

            self
        }

        pub fn starts_with(&self, pat: SteelString) -> bool {
            self.to_slice().starts_with(pat.as_str())
        }

        pub fn ends_with(&self, pat: SteelString) -> bool {
            self.to_slice().ends_with(pat.as_str())
        }
    }

    pub fn rope_module() -> BuiltInModule {
        let mut module = BuiltInModule::new("helix/core/text");

        macro_rules! register_value {
            ($name:expr, $func:expr, $doc:expr) => {
                module.register_fn($name, $func);
                module.register_doc($name, MarkdownDoc(Cow::Borrowed($doc)));
            };
        }

        register_value!(
            "Rope?",
            |value: SteelVal| SteelRopeSlice::as_ref(&value).is_ok(),
            "Check if the given value is a rope"
        );

        register_value!(
            "string->rope",
            SteelRopeSlice::from_string,
            r#"Converts a string into a rope.

```scheme
(string->rope value) -> Rope?
```

* value : string?
            "#
        );

        register_value!(
            "rope->slice",
            SteelRopeSlice::slice,
            r#"Take a slice from using character indices from the rope.
Returns a new rope value.

```scheme
(rope->slice rope start end) -> Rope?
```

* rope : Rope?
* start: (and positive? int?)
* end: (and positive? int?)
"#
        );

        register_value!(
            "rope-char->byte",
            SteelRopeSlice::char_to_byte,
            r#"Convert the character offset into a byte offset for a given rope"#
        );

        register_value!(
            "rope-char->byte",
            SteelRopeSlice::byte_to_char,
            r#"Convert the byte offset into a character offset for a given rope"#
        );

        register_value!(
            "rope-line->char",
            SteelRopeSlice::try_line_to_char,
            r#"Convert the given line index to a character offset for a given rope

```scheme
(rope-line->char rope line-offset) -> int?
```

* rope : Rope?
* line-offset: int?
            "#
        );

        register_value!(
            "rope-line->byte",
            SteelRopeSlice::try_line_to_byte,
            r#"Convert the given line index to a byte offset for a given rope

```scheme
(rope-line->byte rope line-offset) -> int?
```

* rope : Rope?
* line-offset: int?
            "#
        );

        register_value!(
            "rope-char->line",
            SteelRopeSlice::try_char_to_line,
            r#"Convert the given character offset to a line offset for a given rope

```scheme
(rope-char->line rope char-index) -> int?
```

* rope : Rope?
* char-index : int?

            "#
        );

        register_value!(
            "rope-byte->line",
            SteelRopeSlice::try_byte_to_line,
            r#"Convert the given byte offset to a line offset for a given rope

```scheme
(rope-byte->line rope byte-index) -> int?
```

* rope : Rope?
* byte-index : int?

            "#
        );

        register_value!(
            "rope->byte-slice",
            SteelRopeSlice::byte_slice,
            r#"Take a slice of this rope using byte offsets

```scheme
(rope->byte-slice rope start end) -> Rope?
```

* rope: Rope?
* start: (and positive? int?)
* end: (and positive? int?)
"#
        );

        register_value!(
            "rope->line",
            SteelRopeSlice::line,
            r#"Get the line at the given line index. Returns a rope.

```scheme
(rope->line rope index) -> Rope?

```

* rope : Rope?
* index : (and positive? int?)
"#
        );

        register_value!(
            "rope->string",
            SteelRopeSlice::to_string,
            "Convert the given rope to a string"
        );

        register_value!(
            "rope-len-chars",
            SteelRopeSlice::len_chars,
            "Get the length of the rope in characters"
        );
        register_value!(
            "rope-len-bytes",
            SteelRopeSlice::len_chars,
            "Get the length of the rope in bytes"
        );

        register_value!(
            "rope-char-ref",
            SteelRopeSlice::get_char,
            "Get the character at the given index"
        );

        register_value!(
            "rope-len-lines",
            SteelRopeSlice::len_lines,
            "Get the number of lines in the rope"
        );

        register_value!(
            "rope-starts-with?",
            SteelRopeSlice::starts_with,
            "Check if the rope starts with a given pattern"
        );

        register_value!(
            "rope-ends-with?",
            SteelRopeSlice::ends_with,
            "Check if the rope ends with a given pattern"
        );

        register_value!(
            "rope-trim-start",
            SteelRopeSlice::trim_start,
            "Remove the leading whitespace from the given rope"
        );

        register_value!(
            "rope-insert-string",
            SteelRopeSlice::insert_str,
            "Insert a string at the given index into the rope"
        );

        register_value!(
            "rope-insert-char",
            SteelRopeSlice::insert_char,
            "Insert a character at the given index"
        );

        module
    }
}
