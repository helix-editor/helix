use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use smartstring::{LazyCompact, SmartString};

type String = SmartString<LazyCompact>;

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct Icons {
    pub mime: Mime,
    pub lsp: Lsp,
    pub diagnostic: Diagnostic,
    pub vcs: Vcs,
    pub dap: Dap,
}

// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentSymbol
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq, Clone)]
pub struct Lsp {
    enabled: bool,

    file: Option<String>,
    module: Option<String>,
    namespace: Option<String>,
    package: Option<String>,
    class: Option<String>,
    method: Option<String>,
    property: Option<String>,
    field: Option<String>,
    constructor: Option<String>,
    #[serde(rename = "enum")]
    r#enum: Option<String>,
    interface: Option<String>,
    function: Option<String>,
    variable: Option<String>,
    constant: Option<String>,
    string: Option<String>,
    number: Option<String>,
    boolean: Option<String>,
    array: Option<String>,
    object: Option<String>,
    key: Option<String>,
    null: Option<String>,
    enum_member: Option<String>,
    #[serde(rename = "struct")]
    r#struct: Option<String>,
    event: Option<String>,
    operator: Option<String>,
    type_parameter: Option<String>,
}

impl Lsp {
    #[inline]
    pub fn file(&self) -> &str {
        if self.enabled {
            return self.file.as_ref().map_or("", |file| file);
        }
        ""
    }

    #[inline]
    pub fn module(&self) -> &str {
        if self.enabled {
            return self.module.as_ref().map_or("", |module| module);
        }
        ""
    }

    #[inline]
    pub fn namespace(&self) -> &str {
        if self.enabled {
            return self.namespace.as_ref().map_or("", |namespace| namespace);
        }
        ""
    }

    #[inline]
    pub fn package(&self) -> &str {
        if self.enabled {
            return self.package.as_ref().map_or("", |package| package);
        }
        ""
    }

    #[inline]
    pub fn class(&self) -> &str {
        if self.enabled {
            return self.class.as_ref().map_or("", |class| class);
        }
        ""
    }

    #[inline]
    pub fn method(&self) -> &str {
        if self.enabled {
            return self.method.as_ref().map_or("", |method| method);
        }
        ""
    }

    #[inline]
    pub fn property(&self) -> &str {
        if self.enabled {
            return self.property.as_ref().map_or("", |property| property);
        }
        ""
    }

    #[inline]
    pub fn field(&self) -> &str {
        if self.enabled {
            return self.field.as_ref().map_or("", |field| field);
        }
        ""
    }

    #[inline]
    pub fn constructor(&self) -> &str {
        if self.enabled {
            return self
                .constructor
                .as_ref()
                .map_or("", |constructor| constructor);
        }
        ""
    }

    #[inline]
    pub fn r#enum(&self) -> &str {
        if self.enabled {
            return self.r#enum.as_ref().map_or("", |r#enum| r#enum);
        }
        ""
    }

    #[inline]
    pub fn interface(&self) -> &str {
        if self.enabled {
            return self.interface.as_ref().map_or("", |interface| interface);
        }
        ""
    }

    #[inline]
    pub fn function(&self) -> &str {
        if self.enabled {
            return self.function.as_ref().map_or("", |function| function);
        }
        ""
    }

    #[inline]
    pub fn variable(&self) -> &str {
        if self.enabled {
            return self.variable.as_ref().map_or("", |variable| variable);
        }
        ""
    }

    #[inline]
    pub fn constant(&self) -> &str {
        if self.enabled {
            return self.constant.as_ref().map_or("", |constant| constant);
        }
        ""
    }

    #[inline]
    pub fn string(&self) -> &str {
        if self.enabled {
            return self.string.as_ref().map_or("", |string| string);
        }
        ""
    }

    #[inline]
    pub fn number(&self) -> &str {
        if self.enabled {
            return self.number.as_ref().map_or("", |number| number);
        }
        ""
    }

    #[inline]
    pub fn boolean(&self) -> &str {
        if self.enabled {
            return self.boolean.as_ref().map_or("", |boolean| boolean);
        }
        ""
    }

    #[inline]
    pub fn array(&self) -> &str {
        if self.enabled {
            return self.array.as_ref().map_or("", |array| array);
        }
        ""
    }

    #[inline]
    pub fn object(&self) -> &str {
        if self.enabled {
            return self.object.as_ref().map_or("", |object| object);
        }
        ""
    }

    #[inline]
    pub fn key(&self) -> &str {
        if self.enabled {
            return self.key.as_ref().map_or("", |key| key);
        }
        ""
    }

    #[inline]
    pub fn null(&self) -> &str {
        if self.enabled {
            return self.null.as_ref().map_or("󰟢", |null| null);
        }
        ""
    }

    #[inline]
    pub fn enum_member(&self) -> &str {
        if self.enabled {
            return self
                .enum_member
                .as_ref()
                .map_or("", |enum_member| enum_member);
        }
        ""
    }

    #[inline]
    pub fn r#struct(&self) -> &str {
        if self.enabled {
            return self.r#struct.as_ref().map_or("", |r#struct| r#struct);
        }
        ""
    }

    #[inline]
    pub fn event(&self) -> &str {
        if self.enabled {
            return self.event.as_ref().map_or("", |event| event);
        }
        ""
    }

    #[inline]
    pub fn operator(&self) -> &str {
        if self.enabled {
            return self.operator.as_ref().map_or("", |operator| operator);
        }
        ""
    }

    #[inline]
    pub fn type_parameter(&self) -> &str {
        if self.enabled {
            return self
                .type_parameter
                .as_ref()
                .map_or("", |type_parameter| type_parameter);
        }
        ""
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Vcs {
    enabled: bool,
    icon: Option<String>,
}

impl Vcs {
    #[inline]
    pub fn icon(&self) -> &str {
        if self.enabled {
            return self.icon.as_ref().map_or("", |icon| icon.as_str());
        }
        ""
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Mime {
    enabled: bool,
    directory: Option<String>,
    #[serde(flatten)]
    mime: HashMap<String, String>,
}

static MIMES: once_cell::sync::Lazy<HashMap<String, String>> = once_cell::sync::Lazy::new(|| {
    let mut mimes = HashMap::new();

    mimes.insert(String::from("rust"), String::from("󱘗"));
    mimes.insert(String::from("python"), String::from("󰌠"));
    mimes.insert(String::from("c"), String::from(""));
    mimes.insert(String::from("cpp"), String::from(""));
    mimes.insert(String::from("c-sharp"), String::from("󰌛"));
    mimes.insert(String::from("d"), String::from(""));
    mimes.insert(String::from("elixir"), String::from(""));
    mimes.insert(String::from("fsharp"), String::from(""));
    mimes.insert(String::from("go"), String::from("󰟓"));
    mimes.insert(String::from("haskell"), String::from("󰲒"));
    mimes.insert(String::from("java"), String::from("󰬷"));
    mimes.insert(String::from("javascript"), String::from("󰌞"));
    mimes.insert(String::from("kotlin"), String::from("󱈙"));
    mimes.insert(String::from("html"), String::from("󰌝"));
    mimes.insert(String::from("css"), String::from("󰌜"));
    mimes.insert(String::from("typescript"), String::from("󰛦"));
    mimes.insert(String::from("bash"), String::from(""));
    mimes.insert(String::from("php"), String::from("󰌟"));
    mimes.insert(String::from("powershell"), String::from("󰨊"));
    mimes.insert(String::from("dart"), String::from(""));
    mimes.insert(String::from("ruby"), String::from("󰴭"));
    mimes.insert(String::from("swift"), String::from("󰛥"));
    mimes.insert(String::from("r"), String::from("󰟔"));
    mimes.insert(String::from("groovy"), String::from(""));
    mimes.insert(String::from("scala"), String::from(""));
    mimes.insert(String::from("perl"), String::from(""));
    mimes.insert(String::from("closure"), String::from(""));
    mimes.insert(String::from("julia"), String::from(""));
    mimes.insert(String::from("zig"), String::from(""));
    mimes.insert(String::from("fortran"), String::from("󱈚"));
    mimes.insert(String::from("erlang"), String::from(""));
    mimes.insert(String::from("ocaml"), String::from(""));
    mimes.insert(String::from("crystal"), String::from(""));
    mimes.insert(String::from("svelte"), String::from(""));
    mimes.insert(String::from("gdscript"), String::from(""));
    mimes.insert(String::from("nim"), String::from(""));

    mimes.insert(String::from("docker"), String::from("󰡨"));
    mimes.insert(String::from("make"), String::from(""));
    mimes.insert(String::from("cmake"), String::from(""));
    mimes.insert(String::from("nix"), String::from(""));

    mimes.insert(String::from("text"), String::from(""));
    mimes.insert(String::from("markdown"), String::from(""));
    mimes.insert(String::from("json"), String::from("󰘦"));
    mimes.insert(String::from("toml"), String::from(""));
    mimes.insert(String::from("xml"), String::from("󰗀"));

    mimes
});

impl Mime {
    #[inline]
    pub fn directory(&self) -> &str {
        if self.enabled {
            return self.directory.as_ref().map_or("󰉋", |directory| directory);
        } else if let Some(directory) = &self.directory {
            return directory;
        }
        ""
    }

    // Returns the symbol that matches the name, if any, otherwise returns the name back.
    #[inline]
    pub fn get<'name, 'mime: 'name>(&'mime self, r#type: &'name str) -> &'name str {
        if self.enabled {
            if let Some(symbol) = self.mime.get(r#type) {
                return symbol;
            } else if let Some(symbol) = MIMES.get(r#type) {
                return symbol;
            }
        }
        r#type
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
        self.verified.as_ref().map_or("◯", |verified| verified)
    }
}
