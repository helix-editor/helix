// NOTE: currently not making use of folds, injections, locals, tags.
// (fd --hidden --glob *.scm --exec basename {} \; | sort | uniq)
/// Helper functions for probing Tree-sitter language support in Helix
#[derive(Copy, Clone)]
pub enum TsFeature {
    Highlight,
    TextObject,
    AutoIndent,
}

impl TsFeature {
    pub fn all() -> &'static [Self] {
        &[Self::Highlight, Self::TextObject, Self::AutoIndent]
    }

    pub fn runtime_filename(&self) -> &'static str {
        match *self {
            Self::Highlight => "highlights.scm",
            Self::TextObject => "textobjects.scm",
            Self::AutoIndent => "indents.scm",
        }
    }

    pub fn long_title(&self) -> &'static str {
        match *self {
            Self::Highlight => "Syntax Highlighting",
            Self::TextObject => "Treesitter Textobjects",
            Self::AutoIndent => "Auto Indent",
        }
    }

    pub fn short_title(&self) -> &'static str {
        match *self {
            Self::Highlight => "Highlight",
            Self::TextObject => "Textobject",
            Self::AutoIndent => "Indent",
        }
    }
}