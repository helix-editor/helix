use arc_swap::ArcSwap;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, path::Path};

use smartstring::{LazyCompact, SmartString};

use crate::theme::Color;

type String = SmartString<LazyCompact>;

/// Centralized location for icons that can be used throughout the UI.
pub static ICONS: Lazy<ArcSwap<Icons>> = Lazy::new(ArcSwap::default);

/// Centralized location for icons that can be used throughout the UI.
///
/// ```no_run
/// use helix_view::icons::ICONS;
/// use std::path::Path;
///
/// let icons = ICONS.load();
///
/// assert_eq!("󱘗", icons.fs().from_path(Path::new("test.rs")).unwrap().glyph());
/// ```
#[derive(Debug, Default, Deserialize, PartialEq, Eq, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Icons {
    fs: Fs,
    kind: Kind,
    diagnostic: Diagnostic,
    vcs: Vcs,
    dap: Dap,
    ui: Ui,
}

impl Icons {
    /// Returns a handle to all filesystem related icons.
    ///
    /// ```no_run
    /// use helix_view::icons::ICONS;
    /// use std::path::Path;
    ///
    /// let icons = ICONS.load();
    ///
    /// assert_eq!("󱘗", icons.fs().from_path(Path::new("test.rs")).unwrap().glyph());
    /// ```
    #[inline]
    pub fn fs(&self) -> &Fs {
        &self.fs
    }

    /// Returns a handle to all symbol and completion icons.
    ///
    /// ```no_run
    /// use helix_view::icons::ICONS;
    ///
    /// let icons = ICONS.load();
    ///
    /// assert_eq!("■", icons.kind().color().glyph());
    /// assert_eq!("", icons.kind().word().unwrap().glyph());
    /// ```
    #[inline]
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    /// Returns a handle to all diagnostic related icons.
    ///
    /// ```
    /// use helix_view::icons::ICONS;
    ///
    /// let icons = ICONS.load();
    ///
    /// assert_eq!("▲", icons.diagnostic().warning());
    /// assert_eq!("■", icons.diagnostic().error());
    /// ```
    #[inline]
    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }

    /// Returns a handle to all version control related icons.
    ///
    /// ```no_run
    /// use helix_view::icons::ICONS;
    ///
    /// let icons = ICONS.load();
    ///
    /// assert_eq!("", icons.vcs().branch().unwrap());
    /// ```
    #[inline]
    pub fn vcs(&self) -> &Vcs {
        &self.vcs
    }

    /// Returns a handle to all debug related icons.
    ///
    /// ```
    /// use helix_view::icons::ICONS;
    ///
    /// let icons = ICONS.load();
    ///
    /// assert_eq!("●", icons.dap().verified());
    /// assert_eq!("◯", icons.dap().unverified());
    /// assert_eq!("▶", icons.dap().play());
    /// ```
    #[inline]
    pub fn dap(&self) -> &Dap {
        &self.dap
    }

    /// Returns a handle to all UI related icons.
    ///
    /// These icons relate to things like virtual text and statusline elements, visual elements, rather than some other
    /// well defined group.
    ///
    /// ```
    /// use helix_view::icons::ICONS;
    ///
    /// let icons = ICONS.load();
    ///
    /// assert_eq!("W", icons.ui().workspace().glyph());
    /// assert_eq!(" ", icons.ui().r#virtual().ruler());
    /// assert_eq!("│", icons.ui().statusline().separator());
    /// ```
    #[inline]
    pub fn ui(&self) -> &Ui {
        &self.ui
    }
}

macro_rules! iconmap {
    ( $( $key:literal => { glyph: $glyph:expr $(, color: $color:expr)? } ),* $(,)? ) => {{
        HashMap::from(
            [
                $(
                  (String::from($key), Icon {
                    glyph: String::from($glyph),
                    color: None $(.or( Some(Color::from_hex($color).unwrap())) )?,
                  }),
                )*
            ]
        )
    }};
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

impl From<&str> for Icon {
    fn from(icon: &str) -> Self {
        Self {
            glyph: String::from(icon),
            color: None,
        }
    }
}

impl Display for Icon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.glyph)
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
    word: Option<Icon>,
    spellcheck: Option<Icon>,
}

impl Kind {
    #[inline]
    #[must_use]
    pub fn get(&self, kind: &str) -> Option<Icon> {
        if !self.enabled {
            return None;
        }

        match kind {
            "file" => self.file(),
            "folder" => self.folder(),
            "module" => self.module(),
            "namespace" => self.namespace(),
            "package" => self.package(),
            "class" => self.class(),
            "method" => self.method(),
            "property" => self.property(),
            "field" => self.field(),
            "construct" => self.constructor(),
            "enum" => self.r#enum(),
            "interface" => self.interface(),
            "function" => self.function(),
            "variable" => self.variable(),
            "constant" => self.constant(),
            "string" => self.string(),
            "number" => self.number(),
            "boolean" => self.boolean(),
            "array" => self.array(),
            "object" => self.object(),
            "key" => self.key(),
            "null" => self.null(),
            "enum_member" => self.enum_member(),
            "struct" => self.r#struct(),
            "event" => self.event(),
            "operator" => self.operator(),
            "typeparam" => self.type_parameter(),
            "color" => Some(self.color()),
            "keyword" => self.keyword(),
            "value" => self.value(),
            "snippet" => self.snippet(),
            "reference" => self.reference(),
            "text" => self.text(),
            "unit" => self.unit(),
            "word" => self.word(),
            "spellcheck" => self.spellcheck(),

            _ => None,
        }
    }

    #[inline]
    pub fn file(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.file.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn folder(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.folder.clone().or_else(|| Some(Icon::from("󰉋")))
    }

    #[inline]
    pub fn module(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.module.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn namespace(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.namespace.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn package(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.package.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn class(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.class.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn method(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.method.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn property(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.property.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn field(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.field.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn constructor(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.constructor.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn r#enum(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.r#enum.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn interface(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.interface.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn function(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.function.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn variable(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.variable.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn constant(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.constant.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn string(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.string.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn number(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.number.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn boolean(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.boolean.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn array(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.array.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn object(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.object.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn key(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.key.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn null(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.null.clone().or_else(|| Some(Icon::from("󰟢")))
    }

    #[inline]
    pub fn enum_member(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.enum_member.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn r#struct(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.r#struct.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn event(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.event.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn operator(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.operator.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn type_parameter(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.type_parameter
            .clone()
            .or_else(|| Some(Icon::from("")))
    }

    // Always enabled
    #[inline]
    pub fn color(&self) -> Icon {
        self.color.clone().unwrap_or_else(|| Icon::from("■"))
    }

    #[inline]
    pub fn keyword(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.keyword.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn value(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.value.clone().or_else(|| Some(Icon::from("󰎠")))
    }

    #[inline]
    pub fn snippet(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.snippet.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn reference(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.reference.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn text(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.text.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn unit(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.unit.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn word(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.word.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn spellcheck(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.spellcheck.clone().or_else(|| Some(Icon::from("󰓆")))
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
        self.hint.as_deref().unwrap_or("○")
    }

    #[inline]
    pub fn info(&self) -> &str {
        self.info.as_deref().unwrap_or("●")
    }

    #[inline]
    pub fn warning(&self) -> &str {
        self.warning.as_deref().unwrap_or("▲")
    }

    #[inline]
    pub fn error(&self) -> &str {
        self.error.as_deref().unwrap_or("■")
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
    pub fn branch(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.branch.as_deref().or(Some(""))
    }

    #[inline]
    pub fn added(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.added.as_deref().or(Some(""))
    }

    #[inline]
    pub fn removed(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.removed.as_deref().or(Some(""))
    }

    #[inline]
    pub fn ignored(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.ignored.as_deref().or(Some(""))
    }

    #[inline]
    pub fn modified(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.modified.as_deref().or(Some(""))
    }

    #[inline]
    pub fn renamed(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.renamed.as_deref().or(Some(""))
    }

    #[inline]
    pub fn conflict(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.conflict.as_deref().or(Some(""))
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Fs {
    enabled: bool,
    directory: Option<String>,
    #[serde(rename = "directory-open")]
    directory_open: Option<String>,
    #[serde(flatten)]
    mime: HashMap<String, Icon>,
}

static MIMES: once_cell::sync::Lazy<HashMap<String, Icon>> = once_cell::sync::Lazy::new(|| {
    iconmap! {
        // Language name
        "git-commit" => {glyph: "", color: "#f15233" },
        "git-rebase" => {glyph: "", color: "#f15233" },
        "git-config" => {glyph: "", color: "#f15233" },
        "helm" => {glyph: "", color: "#277a9f" },
        "nginx" => {glyph: "", color: "#019639" },
        "text" => { glyph: "" },

        // Exact
        "README.md" => { glyph: "" },
        "LICENSE" => { glyph: "󰗑", color: "#e7a933" },
        "LICENSE-MIT" => { glyph: "󰗑", color: "#e7a933" },
        "LICENSE-APACHE" => { glyph: "󰗑", color: "#e7a933" },
        "LICENSE-GPL" => { glyph: "󰗑", color: "#e7a933" },
        "LICENSE-AGPL" => { glyph: "󰗑", color: "#e7a933" },
        "CHANGELOG.md" => { glyph: "", color: "#7bab43" },
        "CODE_OF_CONDUCT.md" => { glyph: "", color: "#f7769d" },
        ".gitignore" => { glyph: "", color: "#f15233" },
        ".gitattributes" => { glyph: "", color: "#f15233" },
        ".git-blame-ignore-revs" => { glyph: "", color: "#f15233" },
        ".gitmodules" => { glyph: "", color: "#f15233" },
        ".editorconfig" => { glyph: "" },
        ".dockerignore" => {glyph: "󰡨", color: "#0096e6" },
        ".ignore" => {glyph: "󰈉" },
        "docker-compose.yaml" => {glyph: "󰡨", color: "#0096e6" },
        "compose.yaml" => {glyph: "󰡨", color: "#0096e6" },
        "Makefile" => {glyph: "" },
        ".prettierrc" => {glyph: "" },
        ".prettierignore" => {glyph: "" },
        "Dockerfile" => {glyph: "󰡨", color: "#0096e6" },
        ".env" => { glyph: "" },
        ".envrc" => { glyph: "" },
        ".mailmap" => { glyph: "" },
        ".vimrc" => { glyph: "", color: "#007f00" },

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
        "nu" => {glyph: "" },
        "zsh" => {glyph: "" },
        "fish" => {glyph: "" },
        "cmd" => {glyph: "" },
        "elv" => {glyph: "" },
        "php" => {glyph: "󰌟", color: "#777bb3" },
        "ps1" => {glyph: "󰨊", color: "#03a9f4" },
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
        "json" => {glyph: "", color: "#f9a825" },
        "toml" => {glyph: "", color: "#a8403e" },
        "xml" => {glyph: "󰗀", color: "#8bc34a" },
        "tex" => {glyph: "", color: "#008080" },
        "todotxt" => {glyph: "", color: "#7cb342" },
        "svg" => {glyph: "󰜡", color: "#ffb300" },
        "png" => {glyph: "", color: "#26a69a" },
        "jpeg" => {glyph: "", color: "#26a69a" },
        "jpg" => {glyph: "", color: "#26a69a" },
        "ico" => {glyph: "", color: "#26a69a" },
        "lock" => {glyph: "", color: "#70797d" },
        "csv" => {glyph: "", color: "#1abb54" },
        "ipynb" => {glyph: "", color: "#f47724" },
        "ttf" => {glyph: "", color: "#144cb7" },
        "exe" => {glyph: "" },
        "bin" => {glyph: "" },
        "bzl" => {glyph: "", color: "#76d275" },
        "sql" => {glyph: "", color: "#ffca28" },
        "db" => {glyph: "", color: "#ffca28" },
        "yaml" => { glyph: "", color: "#cc1018" },
        "yml" => { glyph: "", color: "#cc1018" },
        "conf" => { glyph: "" },
        "ron" => { glyph: "" },
        "hbs" => { glyph: "" },
        "desktop" => { glyph: "" },
        "xlsx" => { glyph: "󱎏", color: "#01ac47" },
        "wxs" => { glyph: "" },
        "vim" => { glyph: "", color: "#007f00" },
    }
});

impl Fs {
    /// Returns the icon for a folder/directory if enabled.
    ///
    /// This takes a `bool` that signified if the returned icon should be an open variant or not.
    #[inline]
    pub fn directory(&self, is_open: bool) -> Option<&str> {
        if !self.enabled {
            return None;
        }

        if is_open {
            self.directory_open.as_deref().or(Some("󰝰"))
        } else {
            self.directory.as_deref().or(Some("󰉋"))
        }
    }

    /// Returns an icon that matches an exact name or extension if enabled.
    ///
    /// If there is no match, and is enabled, it will return `None`.
    #[inline]
    pub fn from_name<'a>(&'a self, name: &str) -> Option<&'a Icon> {
        if !self.enabled {
            return None;
        }

        self.mime.get(name).or_else(|| MIMES.get(name))
    }

    /// Returns an icon that matches an exact name or extension if enabled.
    ///
    /// If there is no match, and is enabled, it will return with the default `text` icon.
    #[inline]
    pub fn from_path<'b, 'a: 'b>(&'a self, path: &'b Path) -> Option<&'b Icon> {
        self.__from_path_or_lang(Some(path), None)
    }

    /// Returns an icon that matches an exact name or extension if enabled.
    ///
    /// If there is no match, and is enabled, or if there is `None` passed in, it will
    /// return with the default `text` icon.
    #[inline]
    pub fn from_optional_path<'b, 'a: 'b>(&'a self, path: Option<&'b Path>) -> Option<&'b Icon> {
        self.__from_path_or_lang(path, None)
    }

    /// Returns an icon that matches an exact name, extension, or language, if enabled.
    ///
    /// If there is no match, and is enabled, it will return with the default `text` icon.
    #[inline]
    pub fn from_path_or_lang<'b, 'a: 'b>(
        &'a self,
        path: &'b Path,
        lang: &'b str,
    ) -> Option<&'b Icon> {
        self.__from_path_or_lang(Some(path), Some(lang))
    }

    /// Returns an icon that matches an exact name, extension, or language, if enabled.
    ///
    /// If there is no match, and is enabled, or if there is `None` passed in and there is not language match, it will
    /// return with the default `text` icon.
    #[inline]
    pub fn from_optional_path_or_lang<'b, 'a: 'b>(
        &'a self,
        path: Option<&'b Path>,
        lang: &'b str,
    ) -> Option<&'b Icon> {
        self.__from_path_or_lang(path, Some(lang))
    }

    fn __from_path_or_lang<'b, 'a: 'b>(
        &'a self,
        path: Option<&'b Path>,
        lang: Option<&'b str>,
    ) -> Option<&'b Icon> {
        if !self.enabled {
            return None;
        }

        // Search via some part of the path.
        if let Some(path) = path {
            // Search for fully specified name first so that custom icons,
            // for example for `README.md` or `docker-compose.yaml`, can
            // take precedence over any extension it may have.
            if let Some(Some(name)) = path.file_name().map(|name| name.to_str()) {
                // Search config options first, then built-in.
                if let Some(icon) = self.mime.get(name).or_else(|| MIMES.get(name)) {
                    return Some(icon);
                }
            }

            // Try to search for icons based off of the extension.
            if let Some(Some(ext)) = path.extension().map(|ext| ext.to_str()) {
                // Search config options first, then built-in.
                if let Some(icon) = self.mime.get(ext).or_else(|| MIMES.get(ext)) {
                    return Some(icon);
                }
            }
        }

        // Try to search via lang name.
        if let Some(lang) = lang {
            // Search config options first, then built-in.
            if let Some(icon) = self.mime.get(lang).or_else(|| MIMES.get(lang)) {
                return Some(icon);
            }
        }

        // If icons are enabled but there is no matching found, default to the `text` icon.
        // Check user configured first, then built-in.
        self.mime.get("text").or_else(|| MIMES.get("text"))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Dap {
    verified: Option<String>,
    unverified: Option<String>,
    play: Option<String>,
}

impl Dap {
    #[inline]
    pub fn verified(&self) -> &str {
        self.verified.as_deref().unwrap_or("●")
    }

    #[inline]
    pub fn unverified(&self) -> &str {
        self.unverified.as_deref().unwrap_or("◯")
    }

    #[inline]
    pub fn play(&self) -> &str {
        self.play.as_deref().unwrap_or("▶")
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Ui {
    workspace: Option<Icon>,
    gutter: Gutter,
    #[serde(rename = "virtual")]
    r#virtual: Virtual,
    statusline: Statusline,
}

impl Ui {
    /// Returns a workspace diagnostic icon.
    ///
    /// If no icon is set in the config, it will return `W` by default.
    #[inline]
    pub fn workspace(&self) -> Icon {
        self.workspace.clone().unwrap_or_else(|| Icon::from("W"))
    }

    #[inline]
    pub fn gutter(&self) -> &Gutter {
        &self.gutter
    }

    #[inline]
    pub fn r#virtual(&self) -> &Virtual {
        &self.r#virtual
    }

    #[inline]
    pub fn statusline(&self) -> &Statusline {
        &self.statusline
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Gutter {
    added: Option<String>,
    modified: Option<String>,
    removed: Option<String>,
}

impl Gutter {
    #[inline]
    pub fn added(&self) -> &str {
        self.added.as_deref().unwrap_or("▍")
    }

    #[inline]
    pub fn modified(&self) -> &str {
        self.modified.as_deref().unwrap_or("▍")
    }

    #[inline]
    pub fn removed(&self) -> &str {
        self.removed.as_deref().unwrap_or("▔")
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Virtual {
    // Whitespace
    space: Option<String>,
    nbsp: Option<String>,
    nnbsp: Option<String>,
    tab: Option<String>,
    newline: Option<String>,
    tabpad: Option<String>,

    // Soft-wrap
    wrap: Option<String>,

    // Indentation guide
    indentation: Option<String>,

    // Ruler
    ruler: Option<String>,
}

impl Virtual {
    #[inline]
    pub fn space(&self) -> &str {
        // Default: U+00B7
        self.space.as_deref().unwrap_or("·")
    }

    #[inline]
    pub fn nbsp(&self) -> &str {
        // Default: U+237D
        self.nbsp.as_deref().unwrap_or("⍽")
    }

    #[inline]
    pub fn nnbsp(&self) -> &str {
        // Default: U+2423
        self.nnbsp.as_deref().unwrap_or("␣")
    }

    #[inline]
    pub fn tab(&self) -> &str {
        // Default: U+2192
        self.tab.as_deref().unwrap_or("→")
    }

    #[inline]
    pub fn newline(&self) -> &str {
        // Default: U+23CE
        self.newline.as_deref().unwrap_or("⏎")
    }

    #[inline]
    pub fn tabpad(&self) -> &str {
        // Default: U+23CE
        self.tabpad.as_deref().unwrap_or(" ")
    }

    #[inline]
    pub fn wrap(&self) -> &str {
        // Default: U+21AA
        self.wrap.as_deref().unwrap_or("↪")
    }

    #[inline]
    pub fn indentation(&self) -> &str {
        // Default: U+254E
        self.indentation.as_deref().unwrap_or("╎")
    }

    #[inline]
    pub fn ruler(&self) -> &str {
        // TODO: Default: U+00A6: ¦
        self.ruler.as_deref().unwrap_or(" ")
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Statusline {
    separator: Option<String>,
}

impl Statusline {
    #[inline]
    pub fn separator(&self) -> &str {
        self.separator.as_deref().unwrap_or("│")
    }
}
