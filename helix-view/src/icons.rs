use helix_loader::merge_toml_values;
use log::warn;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::{path::PathBuf, str};
use toml::Value;

use crate::graphics::{Color, Style};
use crate::Theme;

pub static BLANK_ICON: Icon = Icon {
    icon_char: ' ',
    style: None,
};

/// The style of an icon can either be defined by the TOML file, or by the theme.
/// We need to remember that in order to reload the icons colors when the theme changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconStyle {
    Custom(Style),
    Default(Style),
}

impl Default for IconStyle {
    fn default() -> Self {
        IconStyle::Default(Style::default())
    }
}

impl From<IconStyle> for Style {
    fn from(icon_style: IconStyle) -> Self {
        match icon_style {
            IconStyle::Custom(style) => style,
            IconStyle::Default(style) => style,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Icon {
    #[serde(rename = "icon")]
    pub icon_char: char,
    #[serde(default)]
    #[serde(deserialize_with = "icon_color_to_style", rename = "color")]
    pub style: Option<IconStyle>,
}

impl Icon {
    /// Loads a given style if the icon style is undefined or based on a default value
    pub fn with_default_style(&mut self, style: Style) {
        if self.style.is_none() || matches!(self.style, Some(IconStyle::Default(_))) {
            self.style = Some(IconStyle::Default(style));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Icons {
    pub name: String,
    pub mime_type: Option<HashMap<String, Icon>>,
    pub diagnostic: Diagnostic,
    pub symbol_kind: Option<HashMap<String, Icon>>,
    pub breakpoint: Breakpoint,
    pub diff: Diff,
    pub ui: Option<HashMap<String, Icon>>,
}

impl Icons {
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set theme defined styles to diagnostic icons
    pub fn set_diagnostic_icons_base_style(&mut self, theme: &Theme) {
        self.diagnostic.error.with_default_style(theme.get("error"));
        self.diagnostic.info.with_default_style(theme.get("info"));
        self.diagnostic.hint.with_default_style(theme.get("hint"));
        self.diagnostic
            .warning
            .with_default_style(theme.get("warning"));
    }

    /// Set theme defined styles to symbol-kind icons
    pub fn set_symbolkind_icons_base_style(&mut self, theme: &Theme) {
        let style = theme
            .try_get("symbolkind")
            .unwrap_or_else(|| theme.get("keyword"));
        if let Some(symbol_kind_icons) = &mut self.symbol_kind {
            for (_, icon) in symbol_kind_icons.iter_mut() {
                icon.with_default_style(style);
            }
        }
    }

    /// Set the default style for all icons
    pub fn reset_styles(&mut self) {
        if let Some(mime_type_icons) = &mut self.mime_type {
            for (_, icon) in mime_type_icons.iter_mut() {
                icon.style = Some(IconStyle::Default(Style::default()));
            }
        }
        if let Some(symbol_kind_icons) = &mut self.symbol_kind {
            for (_, icon) in symbol_kind_icons.iter_mut() {
                icon.style = Some(IconStyle::Default(Style::default()));
            }
        }
        if let Some(ui_icons) = &mut self.ui {
            for (_, icon) in ui_icons.iter_mut() {
                icon.style = Some(IconStyle::Default(Style::default()));
            }
        }
        self.diagnostic.error.style = Some(IconStyle::Default(Style::default()));
        self.diagnostic.warning.style = Some(IconStyle::Default(Style::default()));
        self.diagnostic.hint.style = Some(IconStyle::Default(Style::default()));
        self.diagnostic.info.style = Some(IconStyle::Default(Style::default()));
    }

    pub fn icon_from_filetype<'a>(&'a self, filetype: &str) -> Option<&'a Icon> {
        if let Some(mime_type_icons) = &self.mime_type {
            mime_type_icons.get(filetype)
        } else {
            None
        }
    }

    /// Try to return a reference to an appropriate icon for the specified file path, with a default "file" icon if none is found.
    /// If no such "file" icon is available, return `None`.
    pub fn icon_from_path<'a>(&'a self, filepath: Option<&PathBuf>) -> Option<&'a Icon> {
        self.mime_type
            .as_ref()
            .and_then(|mime_type_icons| {
                filepath?
                    .extension()
                    .or(filepath?.file_name())
                    .map(|extension_or_filename| extension_or_filename.to_str())?
                    .and_then(|extension_or_filename| mime_type_icons.get(extension_or_filename))
            })
            .or_else(|| self.ui.as_ref().and_then(|ui_icons| ui_icons.get("file")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Diagnostic {
    pub error: Icon,
    pub warning: Icon,
    pub info: Icon,
    pub hint: Icon,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Breakpoint {
    pub verified: Icon,
    pub unverified: Icon,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Diff {
    pub added: Icon,
    pub deleted: Icon,
    pub modified: Icon,
}

fn icon_color_to_style<'de, D>(deserializer: D) -> Result<Option<IconStyle>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let mut style = Style::default();
    if !s.is_empty() {
        match hex_string_to_rgb(&s) {
            Ok(c) => {
                style = style.fg(c);
            }
            Err(e) => {
                log::error!("{}", e);
            }
        };
        Ok(Some(IconStyle::Custom(style)))
    } else {
        Ok(None)
    }
}

pub fn hex_string_to_rgb(s: &str) -> Result<Color, String> {
    if s.starts_with('#') && s.len() >= 7 {
        if let (Ok(red), Ok(green), Ok(blue)) = (
            u8::from_str_radix(&s[1..3], 16),
            u8::from_str_radix(&s[3..5], 16),
            u8::from_str_radix(&s[5..7], 16),
        ) {
            return Ok(Color::Rgb(red, green, blue));
        }
    }
    Err(format!("Icon color: malformed hexcode: {}", s))
}

pub struct Loader {
    /// Icons directories to search from highest to lowest priority
    icons_dirs: Vec<PathBuf>,
}

pub static DEFAULT_ICONS_DATA: Lazy<Value> = Lazy::new(|| {
    let bytes = include_bytes!("../../icons.toml");
    toml::from_str(str::from_utf8(bytes).unwrap()).expect("Failed to parse base 16 default theme")
});

pub static DEFAULT_ICONS: Lazy<Icons> = Lazy::new(|| Icons {
    name: "default".into(),
    ..Icons::from(DEFAULT_ICONS_DATA.clone())
});

impl Loader {
    /// Creates a new loader that can load icons flavors from two directories.
    pub fn new(dirs: &[PathBuf]) -> Self {
        Self {
            icons_dirs: dirs.iter().map(|p| p.join("icons")).collect(),
        }
    }

    /// Loads icons flavors first looking in the `user_dir` then in `default_dir`.
    /// The `theme` is needed in order to load default styles for diagnostic icons.
    pub fn load(
        &self,
        name: &str,
        theme: &Theme,
        true_color: bool,
    ) -> Result<Icons, anyhow::Error> {
        if name == "default" {
            return Ok(self.default(theme));
        }

        let mut visited_paths = HashSet::new();
        let default_icons = HashMap::from([("default", &DEFAULT_ICONS_DATA)]);
        let mut icons = helix_loader::load_inheritable_toml(
            name,
            &self.icons_dirs,
            &mut visited_paths,
            &default_icons,
            Self::merge_icons,
        )
        .map(Icons::from)?;

        // Remove all styles when there is no truecolor support.
        // Not classy, but less cumbersome than trying to pass a parameter to a deserializer.
        if !true_color {
            icons.reset_styles();
        } else {
            icons.set_diagnostic_icons_base_style(theme);
            icons.set_symbolkind_icons_base_style(theme);
        }

        Ok(Icons {
            name: name.into(),
            ..icons
        })
    }

    fn merge_icons(parent: Value, child: Value) -> Value {
        merge_toml_values(parent, child, 3)
    }

    /// Returns the default icon flavor.
    /// The `theme` is needed in order to load default styles for diagnostic icons.
    pub fn default(&self, theme: &Theme) -> Icons {
        let mut icons = DEFAULT_ICONS.clone();
        icons.set_diagnostic_icons_base_style(theme);
        icons.set_symbolkind_icons_base_style(theme);
        icons
    }
}

impl From<Value> for Icons {
    fn from(value: Value) -> Self {
        if let Value::Table(mut table) = value {
            // remove inherits from value to prevent errors
            table.remove("inherits");
            let toml_str = table.to_string();
            match toml::from_str(&toml_str) {
                Ok(icons) => icons,
                Err(e) => {
                    log::error!("Failed to load icons, falling back to default: {}\n", e);
                    DEFAULT_ICONS.clone()
                }
            }
        } else {
            warn!("Expected icons TOML value to be a table, found {:?}", value);
            DEFAULT_ICONS.clone()
        }
    }
}
