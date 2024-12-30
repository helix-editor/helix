use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
    const DEFAULT: &'static str = "*";

    #[inline]
    pub fn file(&self) -> &str {
        self.file.as_ref().map_or(Self::DEFAULT, |file| file)
    }

    #[inline]
    pub fn module(&self) -> &str {
        self.module.as_ref().map_or(Self::DEFAULT, |module| module)
    }

    #[inline]
    pub fn namespace(&self) -> &str {
        self.namespace
            .as_ref()
            .map_or(Self::DEFAULT, |namespace| namespace)
    }

    #[inline]
    pub fn package(&self) -> &str {
        self.package
            .as_ref()
            .map_or(Self::DEFAULT, |package| package)
    }

    #[inline]
    pub fn class(&self) -> &str {
        self.class.as_ref().map_or(Self::DEFAULT, |class| class)
    }

    #[inline]
    pub fn method(&self) -> &str {
        self.method.as_ref().map_or(Self::DEFAULT, |method| method)
    }

    #[inline]
    pub fn property(&self) -> &str {
        self.property
            .as_ref()
            .map_or(Self::DEFAULT, |property| property)
    }

    #[inline]
    pub fn field(&self) -> &str {
        self.field.as_ref().map_or(Self::DEFAULT, |field| field)
    }

    #[inline]
    pub fn constructor(&self) -> &str {
        self.constructor
            .as_ref()
            .map_or(Self::DEFAULT, |constructor| constructor)
    }

    #[inline]
    pub fn r#enum(&self) -> &str {
        self.r#enum.as_ref().map_or(Self::DEFAULT, |r#enum| r#enum)
    }

    #[inline]
    pub fn interface(&self) -> &str {
        self.interface
            .as_ref()
            .map_or(Self::DEFAULT, |interface| interface)
    }

    #[inline]
    pub fn function(&self) -> &str {
        self.function
            .as_ref()
            .map_or(Self::DEFAULT, |function| function)
    }

    #[inline]
    pub fn variable(&self) -> &str {
        self.variable
            .as_ref()
            .map_or(Self::DEFAULT, |variable| variable)
    }

    #[inline]
    pub fn constant(&self) -> &str {
        self.constant
            .as_ref()
            .map_or(Self::DEFAULT, |constant| constant)
    }

    #[inline]
    pub fn string(&self) -> &str {
        self.string.as_ref().map_or(Self::DEFAULT, |string| string)
    }

    #[inline]
    pub fn number(&self) -> &str {
        self.number.as_ref().map_or(Self::DEFAULT, |number| number)
    }

    #[inline]
    pub fn boolean(&self) -> &str {
        self.boolean
            .as_ref()
            .map_or(Self::DEFAULT, |boolean| boolean)
    }

    #[inline]
    pub fn array(&self) -> &str {
        self.array.as_ref().map_or(Self::DEFAULT, |array| array)
    }

    #[inline]
    pub fn object(&self) -> &str {
        self.object.as_ref().map_or(Self::DEFAULT, |object| object)
    }

    #[inline]
    pub fn key(&self) -> &str {
        self.key.as_ref().map_or(Self::DEFAULT, |key| key)
    }

    #[inline]
    pub fn null(&self) -> &str {
        self.null.as_ref().map_or(Self::DEFAULT, |null| null)
    }

    #[inline]
    pub fn enum_member(&self) -> &str {
        self.enum_member
            .as_ref()
            .map_or(Self::DEFAULT, |enum_member| enum_member)
    }

    #[inline]
    pub fn r#struct(&self) -> &str {
        self.r#struct
            .as_ref()
            .map_or(Self::DEFAULT, |r#struct| r#struct)
    }

    #[inline]
    pub fn event(&self) -> &str {
        self.event.as_ref().map_or(Self::DEFAULT, |event| event)
    }

    #[inline]
    pub fn operator(&self) -> &str {
        self.operator
            .as_ref()
            .map_or(Self::DEFAULT, |operator| operator)
    }

    #[inline]
    pub fn type_parameter(&self) -> &str {
        self.type_parameter
            .as_ref()
            .map_or(Self::DEFAULT, |type_parameter| type_parameter)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Diagnostic {
    hint: Option<String>,
    info: Option<String>,
    warning: Option<String>,
    error: Option<String>,
}

// NOTE: #6646 can be achieved by uncommenting this or adding the symbols
// to the `map_or` default.
// impl Default for Diagnostic {
//     fn default() -> Self {
//         Self {
//             hint: Some(String::from("○")),
//             info: Some(String::from("●")),
//             warning: Some(String::from("▲")),
//             error: Some(String::from("■")),
//         }
//     }
// }

impl Diagnostic {
    const DEFAULT: &'static str = "●";

    #[inline]
    pub fn hint(&self) -> &str {
        self.hint.as_ref().map_or(Self::DEFAULT, |hint| hint)
    }

    #[inline]
    pub fn info(&self) -> &str {
        self.info.as_ref().map_or(Self::DEFAULT, |info| info)
    }

    #[inline]
    pub fn warning(&self) -> &str {
        self.warning
            .as_ref()
            .map_or(Self::DEFAULT, |warning| warning)
    }

    #[inline]
    pub fn error(&self) -> &str {
        self.error.as_ref().map_or(Self::DEFAULT, |error| error)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Vcs {
    icon: Option<String>,
}

impl Vcs {
    const DEFAULT: &'static str = "";

    #[inline]
    pub fn icon(&self) -> &str {
        self.icon
            .as_ref()
            .map_or(Self::DEFAULT, |icon| icon.as_str())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Mime {
    directory: Option<String>,
    #[serde(flatten)]
    mime: HashMap<String, String>,
}

impl Mime {
    #[inline]
    pub fn directory(&self) -> &str {
        self.directory.as_ref().map_or("", |directory| directory)
    }

    // Returns the symbol that matches the name, if any, otherwise returns the name back.
    #[inline]
    pub fn get<'name, 'mime: 'name>(&'mime self, r#type: &'name str) -> &'name str {
        self.mime.get(r#type).map_or(r#type, |mime| mime)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Dap {
    verified: Option<String>,
    unverified: Option<String>,
}

impl Dap {
    const DEFAULT_VERIFIED: &'static str = "●";
    const DEFAULT_UNVERIFIED: &'static str = "◯";

    #[inline]
    pub fn verified(&self) -> &str {
        self.verified
            .as_ref()
            .map_or(Self::DEFAULT_VERIFIED, |verified| verified)
    }

    #[inline]
    pub fn unverified(&self) -> &str {
        self.verified
            .as_ref()
            .map_or(Self::DEFAULT_UNVERIFIED, |verified| verified)
    }
}
