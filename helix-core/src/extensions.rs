#[cfg(feature = "steel")]
pub mod steel_implementations {

    use std::{borrow::Cow, cell::Cell, rc::Rc};

    use ropey::iter::Chars;
    use smallvec::SmallVec;
    use steel::{
        gc::unsafe_erased_pointers::CustomReference,
        rvals::{Custom, SteelString},
        steel_vm::{
            builtin::BuiltInModule, register_fn::RegisterFn, register_fn::RegisterFnBorrowed,
        },
    };

    impl steel::rvals::Custom for crate::Position {}
    impl steel::rvals::Custom for crate::Selection {}

    #[derive(Clone, Copy, Debug)]
    enum SliceKind {
        Normal(usize, usize),
        Byte(usize, usize),
        Line(usize),
    }

    #[derive(Clone)]
    pub struct SteelRopeSlice {
        text: crate::Rope,
        ranges: SmallVec<[SliceKind; 5]>,
    }

    impl Custom for SteelRopeSlice {}

    impl SteelRopeSlice {
        pub fn from_string(string: SteelString) -> Self {
            Self {
                text: crate::Rope::from_str(string.as_str()),
                ranges: SmallVec::default(),
            }
        }

        pub fn new(rope: crate::Rope) -> Self {
            Self {
                text: rope,
                ranges: SmallVec::default(),
            }
        }

        fn to_slice(&self) -> crate::RopeSlice<'_> {
            let mut slice = self.text.slice(..);

            for range in &self.ranges {
                match range {
                    SliceKind::Normal(l, r) => slice = slice.slice(l..r),
                    SliceKind::Byte(l, r) => slice = slice.byte_slice(l..r),
                    SliceKind::Line(index) => slice = slice.line(*index),
                }
            }

            slice
        }

        pub fn slice(mut self, lower: usize, upper: usize) -> Self {
            self.ranges.push(SliceKind::Normal(lower, upper));
            self
        }

        pub fn char_to_byte(&self, pos: usize) -> usize {
            self.to_slice().char_to_byte(pos)
        }

        pub fn byte_slice(mut self, lower: usize, upper: usize) -> Self {
            self.ranges.push(SliceKind::Byte(lower, upper));
            self
        }

        pub fn line(mut self, cursor: usize) -> Self {
            self.ranges.push(SliceKind::Line(cursor));
            self
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

        pub fn trimmed_starts_with(&self, pat: SteelString) -> bool {
            let maybe_owned = Cow::from(self.to_slice());

            maybe_owned.trim_start().starts_with(pat.as_str())
        }

        // pub fn as_cow(&'a self) -> SRopeSliceCowStr<'a> {
        //     SRopeSliceCowStr(std::borrow::Cow::from(self.slice))
        // }
    }

    pub fn rope_module() -> BuiltInModule {
        let mut module = BuiltInModule::new("helix/core/text");

        module
            .register_fn("string->slice", SteelRopeSlice::from_string)
            .register_fn("slice->slice", SteelRopeSlice::slice)
            .register_fn("slice-char->byte", SteelRopeSlice::char_to_byte)
            .register_fn("slice->byte-slice", SteelRopeSlice::byte_slice)
            .register_fn("slice->line", SteelRopeSlice::line)
            .register_fn("slice->string", SteelRopeSlice::to_string)
            .register_fn("slice-len-chars", SteelRopeSlice::len_chars)
            .register_fn("slice-char-ref", SteelRopeSlice::get_char)
            .register_fn("slice-len-lines", SteelRopeSlice::len_lines)
            .register_fn(
                "slice-trim-and-starts-with?",
                SteelRopeSlice::trimmed_starts_with,
            );

        module
    }
}
