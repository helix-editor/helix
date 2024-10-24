#[cfg(feature = "steel")]
pub mod steel_implementations {

    use std::borrow::Cow;

    use steel::{
        gc::ShareableMut,
        rvals::{as_underlying_type, Custom, SteelString},
        steel_vm::{builtin::BuiltInModule, register_fn::RegisterFn},
        SteelVal,
    };

    use helix_stdx::rope::RopeSliceExt;

    use crate::syntax::{AutoPairConfig, SoftWrap};

    impl steel::rvals::Custom for crate::Position {}
    impl steel::rvals::Custom for crate::Selection {}
    impl steel::rvals::Custom for AutoPairConfig {}
    impl steel::rvals::Custom for SoftWrap {}

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

        pub fn to_string(&self) -> String {
            self.to_slice().to_string()
        }

        pub fn len_chars(&self) -> usize {
            self.to_slice().len_chars()
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

        pub fn trimmed_starts_with(&self, pat: SteelString) -> bool {
            let maybe_owned = Cow::from(self.to_slice());

            maybe_owned.trim_start().starts_with(pat.as_str())
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

        module
            .register_fn("string->rope", SteelRopeSlice::from_string)
            .register_fn("rope->slice", SteelRopeSlice::slice)
            .register_fn("rope-char->byte", SteelRopeSlice::char_to_byte)
            .register_fn("rope->byte-slice", SteelRopeSlice::byte_slice)
            .register_fn("rope->line", SteelRopeSlice::line)
            .register_fn("rope->string", SteelRopeSlice::to_string)
            .register_fn("rope-len-chars", SteelRopeSlice::len_chars)
            .register_fn("rope-char-ref", SteelRopeSlice::get_char)
            .register_fn("rope-len-lines", SteelRopeSlice::len_lines)
            .register_fn("rope-starts-with?", SteelRopeSlice::starts_with)
            .register_fn("rope-ends-with?", SteelRopeSlice::ends_with)
            .register_fn("rope-trim-start", SteelRopeSlice::trim_start);

        module
    }
}
