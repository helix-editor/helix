use std::{borrow::Cow, cell::Cell, rc::Rc};

use ropey::iter::Chars;
use steel::{
    gc::unsafe_erased_pointers::CustomReference,
    rvals::Custom,
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
            slice: self.slice.byte_slice(lower..upper),
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

    // module
    // .register_fn("slice-char->byte", SRopeSlice::char_to_byte)
    // .register_fn_borrowed::<S("slice->line", SRopeSlice::line);
    // .register_fn("slice->byte-slice", SRopeSlice::byte_slice);

    // module.register_fn("slice-len-chars", SRopeSlice::len_chars);

    module
}
