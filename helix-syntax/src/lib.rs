use serde::{Deserialize, Serialize};
use tree_sitter::Language;

#[macro_export]
macro_rules! mk_extern {
    ( $( $name:ident ),* ) => {
        $(
            extern "C" { pub fn $name() -> Language; }
        )*
    };
}

#[macro_export]
macro_rules! mk_enum {
    ( $( $camel:ident ),* ) => {
        #[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "lowercase")]
        pub enum Lang {
            $(
                $camel,
            )*
        }
    };
}

#[macro_export]
macro_rules! mk_get_language {
    ( $( ($camel:ident, $name:ident) ),* ) => {
        #[must_use]
        pub fn get_language(lang: Lang) -> Language {
            unsafe {
                match lang {
                    $(
                        Lang::$camel => $name(),
                    )*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! mk_get_language_name {
    ( $( $camel:ident ),* ) => {
        #[must_use]
        pub const fn get_language_name(lang: Lang) -> &'static str {
            match lang {
                $(
                    Lang::$camel => stringify!($camel),
                )*
            }
        }
    };
}

#[macro_export]
macro_rules! mk_langs {
    ( $( ($camel:ident, $name:ident) ),* ) => {
        mk_extern!($( $name ),*);
        mk_enum!($( $camel ),*);
        mk_get_language!($( ($camel, $name) ),*);
        mk_get_language_name!($( $camel ),*);
    };
}

mk_langs!(
    // 1) Name for enum
    // 2) tree-sitter function to call to get a Language
    (Agda, tree_sitter_agda),
    (Bash, tree_sitter_bash),
    (C, tree_sitter_c),
    (CSharp, tree_sitter_c_sharp),
    (Cpp, tree_sitter_cpp),
    (Css, tree_sitter_css),
    (Elixir, tree_sitter_elixir),
    (Go, tree_sitter_go),
    (Haskell, tree_sitter_haskell),
    (Html, tree_sitter_html),
    (Java, tree_sitter_java),
    (Javascript, tree_sitter_javascript),
    (Json, tree_sitter_json),
    (Julia, tree_sitter_julia),
    (Php, tree_sitter_php),
    (Python, tree_sitter_python),
    (Ruby, tree_sitter_ruby),
    (Rust, tree_sitter_rust),
    (Scala, tree_sitter_scala),
    (Swift, tree_sitter_swift),
    (Toml, tree_sitter_toml),
    (Tsx, tree_sitter_tsx),
    (Typescript, tree_sitter_typescript)
);
