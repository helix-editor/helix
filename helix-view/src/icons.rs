use arc_swap::ArcSwap;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

use smartstring::{LazyCompact, SmartString};

use crate::theme::Color;

type String = SmartString<LazyCompact>;

pub static ICONS: Lazy<ArcSwap<Icons>> = Lazy::new(ArcSwap::default);

/// Centralized location for icons that can be used throughout the UI.
#[derive(Debug, Default, Deserialize, PartialEq, Eq, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Icons {
    mime: Mime,
    kind: Kind,
    diagnostic: Diagnostic,
    vcs: Vcs,
    dap: Dap,
    gutter: Gutter,
}

impl Icons {
    #[inline]
    pub fn mime(&self) -> &Mime {
        &self.mime
    }

    #[inline]
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    #[inline]
    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }

    #[inline]
    pub fn vcs(&self) -> &Vcs {
        &self.vcs
    }

    #[inline]
    pub fn dap(&self) -> &Dap {
        &self.dap
    }

    #[inline]
    pub fn gutter(&self) -> &Gutter {
        &self.gutter
    }
}

#[derive(Debug, Deserialize, Default, PartialEq, Eq, Clone)]
pub struct Kind {
    enabled: bool,

    file: Option<Icon>,
    folder: Option<Icon>,
    text: Option<Icon>,
    module: Option<Icon>,
    namespace: Option<Icon>,
    package: Option<Icon>,
    class: Option<Icon>,
    method: Option<Icon>,
    property: Option<Icon>,
    field: Option<Icon>,
    constructor: Option<Icon>,
    #[serde(rename = "enum")]
    r#enum: Option<Icon>,
    interface: Option<Icon>,
    function: Option<Icon>,
    variable: Option<Icon>,
    constant: Option<Icon>,
    string: Option<Icon>,
    number: Option<Icon>,
    boolean: Option<Icon>,
    array: Option<Icon>,
    object: Option<Icon>,
    key: Option<Icon>,
    null: Option<Icon>,
    enum_member: Option<Icon>,
    #[serde(rename = "struct")]
    r#struct: Option<Icon>,
    event: Option<Icon>,
    operator: Option<Icon>,
    type_parameter: Option<Icon>,
    color: Option<Icon>,
    keyword: Option<Icon>,
    value: Option<Icon>,
    snippet: Option<Icon>,
    reference: Option<Icon>,
    unit: Option<Icon>,

    // TODO: See what becomes of the word completion PR and its `word` completion kind.
    word: Option<Icon>,

    spellcheck: Option<Icon>,

    // WIP: Might end up in `diagnostics`.
    // For workspace indicator for `workspace-diagnostics` status-line.
    workspace: Option<Icon>,
}

impl Kind {
    #[inline]
    #[must_use]
    pub fn get(&self, kind: &str) -> Option<Icon> {
        if !self.enabled {
            return None;
        }

        let icon = match kind {
            "file" => self.file()?,
            "folder" => self.folder()?,
            "module" => self.module()?,
            "namespace" => self.namespace()?,
            "package" => self.package()?,
            "class" => self.class()?,
            "method" => self.method()?,
            "property" => self.property()?,
            "field" => self.field()?,
            "construct" => self.constructor()?,
            "enum" => self.r#enum()?,
            "interface" => self.interface()?,
            "function" => self.function()?,
            "variable" => self.variable()?,
            "constant" => self.constant()?,
            "string" => self.string()?,
            "number" => self.number()?,
            "boolean" => self.boolean()?,
            "array" => self.array()?,
            "object" => self.object()?,
            "key" => self.key()?,
            "null" => self.null()?,
            "enum_member" => self.enum_member()?,
            "struct" => self.r#struct()?,
            "event" => self.event()?,
            "operator" => self.operator()?,
            "typeparam" => self.type_parameter()?,
            "color" => self.color(),
            "keyword" => self.keyword()?,
            "value" => self.value()?,
            "snippet" => self.snippet()?,
            "reference" => self.reference()?,
            "text" => self.text()?,
            "unit" => self.unit()?,
            "word" => self.word()?,
            "spellcheck" => self.spellcheck()?,

            _ => return None,
        };

        Some(icon)
    }

    #[inline]
    pub fn file(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let file = self.file.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(file)
    }

    #[inline]
    pub fn folder(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let folder = self.folder.clone().unwrap_or_else(|| Icon {
            glyph: String::from("󰉋"),
            color: None,
        });
        Some(folder)
    }

    #[inline]
    pub fn module(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let module = self.module.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(module)
    }

    #[inline]
    pub fn namespace(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let namespace = self.namespace.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(namespace)
    }

    #[inline]
    pub fn package(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let package = self.package.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(package)
    }

    #[inline]
    pub fn class(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let class = self.class.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(class)
    }

    #[inline]
    pub fn method(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let method = self.method.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(method)
    }

    #[inline]
    pub fn property(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let property = self.property.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(property)
    }

    #[inline]
    pub fn field(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let field = self.field.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(field)
    }

    #[inline]
    pub fn constructor(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let constructor = self.constructor.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(constructor)
    }

    #[inline]
    pub fn r#enum(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let r#enum = self.r#enum.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(r#enum)
    }

    #[inline]
    pub fn interface(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let interface = self.interface.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(interface)
    }

    #[inline]
    pub fn function(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let function = self.function.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(function)
    }

    #[inline]
    pub fn variable(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let variable = self.variable.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(variable)
    }

    #[inline]
    pub fn constant(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let constant = self.constant.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(constant)
    }

    #[inline]
    pub fn string(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let string = self.string.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(string)
    }

    #[inline]
    pub fn number(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let number = self.number.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(number)
    }

    #[inline]
    pub fn boolean(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let boolean = self.boolean.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(boolean)
    }

    #[inline]
    pub fn array(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let array = self.array.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(array)
    }

    #[inline]
    pub fn object(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let object = self.object.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(object)
    }

    #[inline]
    pub fn key(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let key = self.key.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(key)
    }

    #[inline]
    pub fn null(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let null = self.null.clone().unwrap_or_else(|| Icon {
            glyph: String::from("󰟢"),
            color: None,
        });
        Some(null)
    }

    #[inline]
    pub fn enum_member(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let enum_member = self.enum_member.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(enum_member)
    }

    #[inline]
    pub fn r#struct(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let r#struct = self.r#struct.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(r#struct)
    }

    #[inline]
    pub fn event(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let event = self.event.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(event)
    }

    #[inline]
    pub fn operator(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let operator = self.operator.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(operator)
    }

    #[inline]
    pub fn type_parameter(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let type_parameter = self.type_parameter.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(type_parameter)
    }

    // Always enabled
    #[inline]
    pub fn color(&self) -> Icon {
        self.color.clone().unwrap_or_else(|| Icon {
            glyph: String::from("■"),
            color: None,
        })
    }

    #[inline]
    pub fn keyword(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let keyword = self.keyword.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(keyword)
    }

    #[inline]
    pub fn value(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let value = self.value.clone().unwrap_or_else(|| Icon {
            glyph: String::from("󰎠"),
            color: None,
        });
        Some(value)
    }

    #[inline]
    pub fn snippet(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let snippet = self.snippet.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(snippet)
    }

    #[inline]
    pub fn reference(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let reference = self.reference.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(reference)
    }

    #[inline]
    pub fn text(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let text = self.text.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(text)
    }

    #[inline]
    pub fn unit(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let unit = self.unit.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(unit)
    }

    #[inline]
    pub fn word(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let word = self.word.clone().unwrap_or_else(|| Icon {
            glyph: String::from(""),
            color: None,
        });
        Some(word)
    }

    #[inline]
    pub fn spellcheck(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        let spellcheck = self.spellcheck.clone().unwrap_or_else(|| Icon {
            glyph: String::from("󰓆"),
            color: None,
        });
        Some(spellcheck)
    }

    /// Returns a workspace diagnostic icon.
    ///
    /// If no icon is set in the config, it will return `W` by default.
    #[inline]
    pub fn workspace(&self) -> Icon {
        self.workspace.clone().unwrap_or_else(|| Icon {
            glyph: String::from("W"),
            color: None,
        })
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Diagnostic {
    hint: Option<String>,
    info: Option<String>,
    warning: Option<String>,
    error: Option<String>,
}

impl Diagnostic {
    #[inline]
    pub fn hint(&self) -> &str {
        self.hint.as_ref().map_or("○", |hint| hint)
    }

    #[inline]
    pub fn info(&self) -> &str {
        self.info.as_ref().map_or("●", |info| info)
    }

    #[inline]
    pub fn warning(&self) -> &str {
        self.warning.as_ref().map_or("▲", |warning| warning)
    }

    #[inline]
    pub fn error(&self) -> &str {
        self.error.as_ref().map_or("■", |error| error)
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Vcs {
    enabled: bool,
    branch: Option<String>,
    added: Option<String>,
    removed: Option<String>,
    ignored: Option<String>,
    modified: Option<String>,
    renamed: Option<String>,
    conflict: Option<String>,
}

impl Vcs {
    #[inline]
    pub fn branch(&self) -> &str {
        if self.enabled {
            return self.branch.as_ref().map_or("", |branch| branch.as_str());
        }
        ""
    }

    #[inline]
    pub fn added(&self) -> &str {
        if self.enabled {
            return self.added.as_ref().map_or("", |added| added.as_str());
        }
        ""
    }

    #[inline]
    pub fn removed(&self) -> &str {
        if self.enabled {
            return self
                .removed
                .as_ref()
                .map_or("", |removed| removed.as_str());
        }
        ""
    }

    #[inline]
    pub fn ignored(&self) -> &str {
        if self.enabled {
            return self
                .ignored
                .as_ref()
                .map_or("", |ignored| ignored.as_str());
        }
        ""
    }

    #[inline]
    pub fn modified(&self) -> &str {
        if self.enabled {
            return self
                .modified
                .as_ref()
                .map_or("", |modified| modified.as_str());
        }
        ""
    }

    #[inline]
    pub fn renamed(&self) -> &str {
        if self.enabled {
            return self
                .renamed
                .as_ref()
                .map_or("", |renamed| renamed.as_str());
        }
        ""
    }

    #[inline]
    pub fn conflict(&self) -> &str {
        if self.enabled {
            return self
                .conflict
                .as_ref()
                .map_or("", |conflict| conflict.as_str());
        }
        ""
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Mime {
    enabled: bool,
    directory: Option<String>,
    #[serde(flatten)]
    mime: HashMap<String, Icon>,
}

macro_rules! mimes {
    ( $( $key:literal => { glyph: $glyph:expr $(, color: $color:expr)? } ),* $(,)? ) => {{
        let mut map = HashMap::new();
        $(
            map.insert(String::from($key), Icon {
                glyph: String::from($glyph),
                color: None $(.or( Some(Color::from_hex($color).unwrap())) )?,
            });
        )*
        map
    }};
}

static MIMES: once_cell::sync::Lazy<HashMap<String, Icon>> = once_cell::sync::Lazy::new(|| {
    mimes! {
    // Language name
        "git-commit" => {glyph: "", color: "#f15233" },
        "git-rebase" => {glyph: "", color: "#f15233" },
        "git-config" => {glyph: "", color: "#f15233" },
        "helm" => {glyph: "", color: "#277a9f" },
        "nginx" => {glyph: "", color: "#019639" },
        "docker" => {glyph: "󰡨", color: "#0096e6" },
        "docker-compose" => {glyph: "󰡨", color: "#0096e6" },
        "text" => { glyph: "" },

    // Exact
        "README.md" => { glyph: "" },
        "LICENSE" => { glyph: "󰗑", color: "#e7a933" },
        "CHANGELOG.md" => { glyph: "", color: "#7bab43" },
        ".gitignore" => { glyph: "", color: "#f15233" },
        ".gitattributes" => { glyph: "", color: "#f15233" },
        ".editorconfig" => { glyph: "" },
        ".env" => { glyph: "" },
        ".dockerignore" => {glyph: "󰡨", color: "#0096e6" },

    // Extension
        "rs" => {glyph: "󱘗", color: "#fab387" },
        "py" => {glyph: "󰌠", color: "#ffd94a" },
        "c" => {glyph: "", color: "#b0c4de" },
        "cpp" => {glyph: "", color: "#0288d1" },
        "cs" => {glyph: "", color: "#512bd4" },
        "d" => {glyph: "", color: "#b03931" },
        "ex" => {glyph: "", color: "#71567d" },
        "fs" => {glyph: "", color: "#2fb9da" },
        "go" => {glyph: "󰟓", color: "#00acd8" },
        "hs" => {glyph: "󰲒", color: "#5e5089" },
        "java" => {glyph: "󰬷", color: "#f58217" },
        "js" => {glyph: "󰌞", color: "#f0dc4e" },
        "ts" => {glyph: "󰛦", color: "#3179c7" },
        "kt" => {glyph: "󱈙", color: "#8a48fc" },
        "html" => {glyph: "󰌝", color: "#f15c29" },
        "css" => {glyph: "󰌜", color: "#9479b6" },
        "scss" => {glyph: "󰟬", color: "#d06599" },
        "sh" => {glyph: "" },
        "bash" => {glyph: "" },
        "php" => {glyph: "󰌟", color: "#777bb3" },
        "ps1" => {glyph: "󰨊", color: "#2670be" },
        "dart" => {glyph: "", color: "#2db7f6" },
        "ruby" => {glyph: "󰴭", color: "#d30000" },
        "swift" => {glyph: "󰛥", color: "#fba03d" },
        "r" => {glyph: "󰟔", color: "#236abd" },
        "groovy" => {glyph: "", color: "#4298b8" },
        "scala" => {glyph: "", color: "#db3331" },
        "pl" => {glyph: "", color: "#006894" },
        "clj" => {glyph: "", color: "#91b4ff" },
        "jl" => {glyph: "", color: "#cb3c33" },
        "zig" => {glyph: "", color: "#f7a41d" },
        "f" => {glyph: "󱈚", color: "#734f96" },
        "erl" => {glyph: "", color: "#a90432" },
        "ml" => {glyph: "", color: "#f29000" },
        "cr" => {glyph: "" },
        "svelte" => {glyph: "", color: "#ff5620" },
        "gd" => {glyph: "", color: "#478cbf" },
        "nim" => {glyph: "", color: "#efc743" },
        "jsx" => {glyph: "", color: "#61dafb" },
        "tsx" => {glyph: "", color: "#61dafb" },
        "twig" => {glyph: "", color: "#a8bf21" },
        "lua" => {glyph: "", color: "#74c7ec" },
        "vue" => {glyph: "", color: "#40b884" },
        "lisp" => {glyph: "" },
        "elm" => {glyph: "", color: "#5b6379" },
        "res" => {glyph: "", color: "#ef5350" },
        "sol" => {glyph: "" },
        "vala" => {glyph: "", color: "#a972e4" },
        "scm" => {glyph: "", color: "#d53d32" },
        "v" => {glyph: "", color: "#5e87c0" },
        "prisma" => {glyph: "" },
        "ada" => {glyph: "", color: "#195c19" },
        "astro" => {glyph: "", color: "#ed45cf" },
        "m" => {glyph: "", color: "#ed8012" },
        "rst" => {glyph: "", color: "#74aada" },
        "cl" => {glyph: "" },
        "njk" => {glyph: "", color: "#53a553" },
        "jinja" => {glyph: "" },
        "bicep" => {glyph: "", color: "#529ab7" },
        "wat" => {glyph: "", color: "#644fef" },
        "md" => {glyph: "" },
        "make" => {glyph: "" },
        "cmake" => {glyph: "", color: "#3eae2b" },
        "nix" => {glyph: "", color: "#4f73bd" },
        "awk" => {glyph: "" },
        "ll" => {glyph: "", color: "#09627d" },
        "regex" => {glyph: "" },
        "gql" => {glyph: "", color: "#e534ab" },
        "typst" => {glyph: "", color: "#5bc0af" },
        "json" => {glyph: "" },
        "toml" => {glyph: "", color: "#a8403e" },
        "xml" => {glyph: "󰗀", color: "#8bc34a" },
        "tex" => {glyph: "", color: "#008080" },
        "todotxt" => {glyph: "", color: "#7cb342" },
        "svg" => {glyph: "󰜡", color: "#ffb300" },
        "png" => {glyph: "", color: "#26a69a" },
        "jpeg" => {glyph: "", color: "#26a69a" },
        "jpg" => {glyph: "", color: "#26a69a" },
        "lock" => {glyph: "", color: "#70797d" },
        "Dockerfile" => {glyph: "󰡨", color: "#0096e6" },
        "csv" => {glyph: "", color: "#1abb54" },
        "ipynb" => {glyph: "", color: "#f47724" },
        "ttf" => {glyph: "", color: "#144cb7" },
        "exe" => {glyph: "" },
    }
});

impl Mime {
    #[inline]
    pub fn directory(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }

        let dir = self.directory.as_ref().map_or("󰉋", |dir| dir.as_str());

        Some(dir)
    }

    // Returns the icon that matches the name, if any, otherwise returns the name back.
    #[inline]
    pub fn get<'b, 'a: 'b>(
        &'a self,
        path: Option<&'b PathBuf>,
        name: Option<&'b str>,
    ) -> Option<&'b Icon> {
        if !self.enabled {
            return None;
        }

        if let Some(path) = path {
            // Search for fully specified name first so that custom icons,
            // for example for `README.md` or `docker-compose.yaml`, can
            // take precedence over any extension it make have.
            if let Some(name) = path.file_name() {
                // Search config options first
                if let Some(icon) = self.mime.get(name.to_str()?) {
                    return Some(icon);
                }

                // Then built-in
                if let Some(icon) = MIMES.get(name.to_str()?) {
                    return Some(icon);
                }
            }

            // Try to search for icons based off of the extension.
            if let Some(name) = path.extension() {
                // Search config options first
                if let Some(icon) = self.mime.get(name.to_str()?) {
                    return Some(icon);
                }

                // Then built-in
                if let Some(icon) = MIMES.get(name.to_str()?) {
                    return Some(icon);
                }
            }
        }

        if let Some(name) = name {
            // Search config options first
            if let Some(icon) = self.mime.get(name) {
                return Some(icon);
            }

            // Then built-in
            if let Some(icon) = MIMES.get(name) {
                return Some(icon);
            }
        }

        // If icons are enabled but there is no matching found, default to the `text` icon.
        //
        // Check user configured first
        if let Some(icon) = self.mime.get("text") {
            return Some(icon);
        }

        // Then built-in
        MIMES.get("text")
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Dap {
    verified: Option<String>,
    unverified: Option<String>,
}

impl Dap {
    #[inline]
    pub fn verified(&self) -> &str {
        self.verified.as_ref().map_or("●", |verified| verified)
    }

    #[inline]
    pub fn unverified(&self) -> &str {
        self.unverified
            .as_ref()
            .map_or("◯", |unverified| unverified)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Gutter {
    added: Option<String>,
    modified: Option<String>,
    deleted: Option<String>,
}

impl Gutter {
    #[inline]
    pub fn added(&self) -> &str {
        self.added.as_ref().map_or("▍", |added| added)
    }

    #[inline]
    pub fn modified(&self) -> &str {
        self.modified.as_ref().map_or("▍", |modified| modified)
    }

    #[inline]
    pub fn deleted(&self) -> &str {
        self.deleted.as_ref().map_or("▔", |deleted| deleted)
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Icon {
    glyph: String,
    color: Option<Color>,
}

impl Icon {
    pub fn glyph(&self) -> &str {
        self.glyph.as_str()
    }

    pub const fn color(&self) -> Option<Color> {
        self.color
    }
}

impl<'de> Deserialize<'de> for Icon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(IconVisitor)
    }
}

struct IconVisitor;

impl<'de> serde::de::Visitor<'de> for IconVisitor {
    type Value = Icon;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a string glyph or a map with 'glyph' and optional 'color'"
        )
    }

    fn visit_str<E>(self, glyph: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Icon {
            glyph: String::from(glyph),
            color: None,
        })
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut glyph = None;
        let mut color = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "glyph" => {
                    if glyph.is_some() {
                        return Err(serde::de::Error::duplicate_field("glyph"));
                    }
                    glyph = Some(map.next_value::<String>()?);
                }
                "color" => {
                    if color.is_some() {
                        return Err(serde::de::Error::duplicate_field("color"));
                    }
                    color = Some(map.next_value::<String>()?);
                }
                _ => return Err(serde::de::Error::unknown_field(&key, &["glyph", "color"])),
            }
        }

        let glyph = glyph.ok_or_else(|| serde::de::Error::missing_field("glyph"))?;

        let color = if let Some(hex) = color {
            let color = Color::from_hex(&hex).ok_or_else(|| {
                serde::de::Error::custom(format!("`{hex} is not a valid color code`"))
            })?;
            Some(color)
        } else {
            None
        };

        Ok(Icon { glyph, color })
    }
}
