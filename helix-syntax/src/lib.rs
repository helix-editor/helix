use enum_iterator::IntoEnumIterator;
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
        #[derive(Clone, Debug, IntoEnumIterator, PartialEq)]
        pub enum LANG {
            $(
                $camel,
            )*
        }
    };
}

#[macro_export]
macro_rules! mk_get_language {
    ( $( ($camel:ident, $name:ident) ),* ) => {
        pub fn get_language(lang: &LANG) -> Language {
            unsafe {
                match lang {
                    $(
                        LANG::$camel => $name(),
                    )*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! mk_get_language_name {
    ( $( $camel:ident ),* ) => {
        pub fn get_language_name(lang: &LANG) -> &'static str {
            match lang {
                $(
                    LANG::$camel => stringify!($camel),
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
    // (Cpp, tree_sitter_cpp),
    (Css, tree_sitter_css),
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
