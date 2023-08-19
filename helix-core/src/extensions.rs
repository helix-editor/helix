use std::{borrow::Cow, cell::Cell, rc::Rc};

use ropey::iter::Chars;
use smallvec::SmallVec;
use steel::{
    gc::unsafe_erased_pointers::CustomReference,
    rvals::{Custom, SteelString},
    steel_vm::{builtin::BuiltInModule, register_fn::RegisterFn, register_fn::RegisterFnBorrowed},
};

impl steel::rvals::Custom for crate::Position {}
impl steel::rvals::Custom for crate::Selection {}

pub struct SRopeSlice<'a> {
    slice: crate::RopeSlice<'a>,
}

steel::custom_reference!(SRopeSlice<'a>);
impl<'a> CustomReference for SRopeSlice<'a> {}

// impl Custom for SRopeSlice<'static> {}

pub struct SRopeSliceCowStr<'a>(Cow<'a, str>);
steel::custom_reference!(SRopeSliceCowStr<'a>);
impl<'a> CustomReference for SRopeSliceCowStr<'a> {}

struct CharIter<'a>(Chars<'a>);

impl<'a> SRopeSlice<'a> {
    pub fn new(slice: crate::RopeSlice<'a>) -> Self {
        Self { slice }
    }

    pub fn char_to_byte(&self, pos: usize) -> usize {
        self.slice.char_to_byte(pos)
    }

    pub fn byte_slice(&'a self, lower: usize, upper: usize) -> SRopeSlice<'a> {
        SRopeSlice {
            slice: self.slice.byte_slice(lower..upper),
        }
    }

    pub fn line(&'a self, cursor: usize) -> SRopeSlice<'a> {
        SRopeSlice {
            slice: self.slice.line(cursor),
        }
    }

    pub fn as_cow(&'a self) -> SRopeSliceCowStr<'a> {
        SRopeSliceCowStr(std::borrow::Cow::from(self.slice))
    }

    pub fn to_string(&self) -> String {
        self.slice.to_string()
    }

    pub fn len_chars(&'a self) -> usize {
        self.slice.len_chars()
    }

    pub fn slice(&'a self, lower: usize, upper: usize) -> SRopeSlice<'a> {
        SRopeSlice {
            slice: self.slice.slice(lower..upper),
        }
    }

    pub fn get_char(&'a self, index: usize) -> Option<char> {
        self.slice.get_char(index)
    }
}

// RegisterFn::<
//     _,
//     steel::steel_vm::register_fn::MarkerWrapper7<(
//         Context<'_>,
//         helix_view::Editor,
//         helix_view::Editor,
//         Context<'static>,
//     )>,
//     helix_view::Editor,
// >::register_fn(&mut engine, "cx-editor!", get_editor);

pub fn rope_slice_module() -> BuiltInModule {
    let mut module = BuiltInModule::new("helix/core/text");

    // (SELF, ARG, SELFSTAT, RET, RETSTAT)

    RegisterFnBorrowed::<
        _,
        steel::steel_vm::register_fn::MarkerWrapper9<(
            SRopeSlice<'_>,
            usize,
            SRopeSlice<'static>,
            SRopeSlice<'_>,
            SRopeSlice<'static>,
        )>,
        SRopeSlice,
    >::register_fn_borrowed(&mut module, "slice->line", SRopeSlice::line);

    // TODO: Note the difficulty of the lifetime params here
    module.register_fn("slice->string", SRopeSlice::to_string);
    module.register_fn("slice-char->byte", SRopeSlice::char_to_byte);

    // module
    // .register_fn("slice-char->byte", SRopeSlice::char_to_byte)
    // .register_fn_borrowed::<S("slice->line", SRopeSlice::line);
    // .register_fn("slice->byte-slice", SRopeSlice::byte_slice);

    // module.register_fn("slice-len-chars", SRopeSlice::len_chars);

    module
}

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

// #[derive(Clone)]
// pub struct SteelRopeString {
//     slice: SteelRopeSlice,
//     operations: SmallVec<[StringOperation; 5]>,
// }

// #[derive(Clone)]
// enum StringOperation {
//     TrimStart,
//     StartsWith(SteelString),
// }

// impl SteelRopeString {
//     pub fn evaluate(&self) -> SteelVal {
//         todo!()
//     }
// }

impl Custom for SteelRopeSlice {}
// impl Custom for SteelRopeString {}

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
