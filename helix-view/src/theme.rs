use std::{
    collections::HashMap,
    convert::TryFrom,
    path::{Path, PathBuf},
};

use anyhow::Context;
use log::warn;
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer};
use toml::Value;

pub use crate::graphics::{Color, Modifier, Style};

pub static DEFAULT_THEME: Lazy<Theme> = Lazy::new(|| {
    toml::from_slice(include_bytes!("../../theme.toml")).expect("Failed to parse default theme")
});

#[derive(Clone, Debug)]
pub struct Loader {
    user_dir: PathBuf,
    default_dir: PathBuf,
}
impl Loader {
    /// Creates a new loader that can load themes from two directories.
    pub fn new<P: AsRef<Path>>(user_dir: P, default_dir: P) -> Self {
        Self {
            user_dir: user_dir.as_ref().join("themes"),
            default_dir: default_dir.as_ref().join("themes"),
        }
    }

    /// Loads a theme first looking in the `user_dir` then in `default_dir`
    pub fn load(&self, name: &str) -> Result<Theme, anyhow::Error> {
        if name == "default" {
            return Ok(self.default());
        }
        let filename = format!("{}.toml", name);

        let user_path = self.user_dir.join(&filename);
        let path = if user_path.exists() {
            user_path
        } else {
            self.default_dir.join(filename)
        };

        let data = std::fs::read(&path)?;
        toml::from_slice(data.as_slice()).context("Failed to deserialize theme")
    }

    pub fn read_names(path: &Path) -> Vec<String> {
        std::fs::read_dir(path)
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        let entry = entry.ok()?;
                        let path = entry.path();
                        (path.extension()? == "toml")
                            .then(|| path.file_stem().unwrap().to_string_lossy().into_owned())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Lists all theme names available in default and user directory
    pub fn names(&self) -> Vec<String> {
        let mut names = Self::read_names(&self.user_dir);
        names.extend(Self::read_names(&self.default_dir));
        names
    }

    /// Returns the default theme
    pub fn default(&self) -> Theme {
        DEFAULT_THEME.clone()
    }
}

#[derive(Clone, Debug)]
pub struct Theme {
    scopes: Vec<String>,
    styles: HashMap<String, Style>,
}

impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut styles = HashMap::new();

        if let Ok(mut colors) = HashMap::<String, Value>::deserialize(deserializer) {
            // TODO: alert user of parsing failures in editor
            let palette = colors
                .remove("palette")
                .map(|value| {
                    ThemePalette::try_from(value).unwrap_or_else(|err| {
                        warn!("{}", err);
                        ThemePalette::default()
                    })
                })
                .unwrap_or_default();

            styles.reserve(colors.len());
            for (name, style_value) in colors {
                let mut style = Style::default();
                if let Err(err) = palette.parse_style(&mut style, style_value) {
                    warn!("{}", err);
                }
                styles.insert(name, style);
            }
        }

        let scopes = styles.keys().map(ToString::to_string).collect();
        Ok(Self { scopes, styles })
    }
}

impl Theme {
    pub fn get(&self, scope: &str) -> Style {
        self.try_get(scope)
            .unwrap_or_else(|| Style::default().fg(Color::Rgb(0, 0, 255)))
    }

    pub fn try_get(&self, scope: &str) -> Option<Style> {
        self.styles.get(scope).copied()
    }

    #[inline]
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    pub fn find_scope_index(&self, scope: &str) -> Option<usize> {
        self.scopes().iter().position(|s| s == scope)
    }
}

struct ThemePalette {
    palette: HashMap<String, Color>,
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self::new(HashMap::new())
    }
}

impl ThemePalette {
    pub fn new(palette: HashMap<String, Color>) -> Self {
        Self { palette }
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

        Err(format!("Theme: malformed hexcode: {}", s))
    }

    fn parse_value_as_str(value: &Value) -> Result<&str, String> {
        value
            .as_str()
            .ok_or(format!("Theme: unrecognized value: {}", value))
    }

    pub fn parse_color(&self, value: Value) -> Result<Color, String> {
        let value = Self::parse_value_as_str(&value)?;

        self.palette
            .get(value)
            .copied()
            .ok_or("")
            .or_else(|_| Self::hex_string_to_rgb(value))
    }

    pub fn parse_modifier(value: &Value) -> Result<Modifier, String> {
        value
            .as_str()
            .and_then(|s| s.parse().ok())
            .ok_or(format!("Theme: invalid modifier: {}", value))
    }

    pub fn parse_style(&self, style: &mut Style, value: Value) -> Result<(), String> {
        if let Value::Table(entries) = value {
            for (name, value) in entries {
                match name.as_str() {
                    "fg" => *style = style.fg(self.parse_color(value)?),
                    "bg" => *style = style.bg(self.parse_color(value)?),
                    "modifiers" => {
                        let modifiers = value
                            .as_array()
                            .ok_or("Theme: modifiers should be an array")?;

                        for modifier in modifiers {
                            *style = style.add_modifier(Self::parse_modifier(modifier)?);
                        }
                    }
                    _ => return Err(format!("Theme: invalid style attribute: {}", name)),
                }
            }
        } else {
            *style = style.fg(self.parse_color(value)?);
        }
        Ok(())
    }
}

impl TryFrom<Value> for ThemePalette {
    type Error = String;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let map = match value {
            Value::Table(entries) => entries,
            _ => return Ok(Self::default()),
        };

        let mut palette = HashMap::with_capacity(map.len());
        for (name, value) in map {
            let value = Self::parse_value_as_str(&value)?;
            let color = Self::hex_string_to_rgb(value)?;
            palette.insert(name, color);
        }

        Ok(Self::new(palette))
    }
}

#[test]
fn test_parse_style_string() {
    let fg = Value::String("#ffffff".to_string());

    let mut style = Style::default();
    let palette = ThemePalette::default();
    palette.parse_style(&mut style, fg).unwrap();

    assert_eq!(style, Style::default().fg(Color::Rgb(255, 255, 255)));
}

#[test]
fn test_palette() {
    use helix_core::hashmap;
    let fg = Value::String("my_color".to_string());

    let mut style = Style::default();
    let palette =
        ThemePalette::new(hashmap! { "my_color".to_string() => Color::Rgb(255, 255, 255) });
    palette.parse_style(&mut style, fg).unwrap();

    assert_eq!(style, Style::default().fg(Color::Rgb(255, 255, 255)));
}

#[test]
fn test_parse_style_table() {
    let table = toml::toml! {
        "keyword" = {
            fg = "#ffffff",
            bg = "#000000",
            modifiers = ["bold"],
        }
    };

    let mut style = Style::default();
    let palette = ThemePalette::default();
    if let Value::Table(entries) = table {
        for (_name, value) in entries {
            palette.parse_style(&mut style, value).unwrap();
        }
    }

    assert_eq!(
        style,
        Style::default()
            .fg(Color::Rgb(255, 255, 255))
            .bg(Color::Rgb(0, 0, 0))
            .add_modifier(Modifier::BOLD)
    );
}
