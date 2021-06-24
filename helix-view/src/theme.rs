use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Context;
use log::warn;
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer};
use toml::Value;

pub use crate::graphics::{Color, Modifier, Style};

/// Color theme for syntax highlighting.

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
                        if let Ok(entry) = entry {
                            let path = entry.path();
                            if let Some(ext) = path.extension() {
                                if ext != "toml" {
                                    return None;
                                }
                                return Some(
                                    entry
                                        .file_name()
                                        .to_string_lossy()
                                        .trim_end_matches(".toml")
                                        .to_owned(),
                                );
                            }
                        }
                        None
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

        if let Ok(colors) = HashMap::<String, Value>::deserialize(deserializer) {
            // scopes.reserve(colors.len());
            styles.reserve(colors.len());
            for (name, style_value) in colors {
                let mut style = Style::default();
                parse_style(&mut style, style_value);
                // scopes.push(name);
                styles.insert(name, style);
            }
        }

        let scopes = styles.keys().map(ToString::to_string).collect();
        Ok(Self { scopes, styles })
    }
}

fn parse_style(style: &mut Style, value: Value) {
    //TODO: alert user of parsing failures
    if let Value::Table(entries) = value {
        for (name, value) in entries {
            match name.as_str() {
                "fg" => {
                    if let Some(color) = parse_color(value) {
                        *style = style.fg(color);
                    }
                }
                "bg" => {
                    if let Some(color) = parse_color(value) {
                        *style = style.bg(color);
                    }
                }
                "modifiers" => {
                    if let Value::Array(arr) = value {
                        for modifier in arr.iter().filter_map(parse_modifier) {
                            *style = style.add_modifier(modifier);
                        }
                    }
                }
                _ => (),
            }
        }
    } else if let Some(color) = parse_color(value) {
        *style = style.fg(color);
    }
}

fn hex_string_to_rgb(s: &str) -> Option<(u8, u8, u8)> {
    if s.starts_with('#') && s.len() >= 7 {
        if let (Ok(red), Ok(green), Ok(blue)) = (
            u8::from_str_radix(&s[1..3], 16),
            u8::from_str_radix(&s[3..5], 16),
            u8::from_str_radix(&s[5..7], 16),
        ) {
            Some((red, green, blue))
        } else {
            None
        }
    } else {
        None
    }
}

fn parse_color(value: Value) -> Option<Color> {
    if let Value::String(s) = value {
        if let Some((red, green, blue)) = hex_string_to_rgb(&s) {
            Some(Color::Rgb(red, green, blue))
        } else {
            warn!("malformed hexcode in theme: {}", s);
            None
        }
    } else {
        warn!("unrecognized value in theme: {}", value);
        None
    }
}

fn parse_modifier(value: &Value) -> Option<Modifier> {
    if let Value::String(s) = value {
        match s.as_str() {
            "bold" => Some(Modifier::BOLD),
            "dim" => Some(Modifier::DIM),
            "italic" => Some(Modifier::ITALIC),
            "underlined" => Some(Modifier::UNDERLINED),
            "slow_blink" => Some(Modifier::SLOW_BLINK),
            "rapid_blink" => Some(Modifier::RAPID_BLINK),
            "reversed" => Some(Modifier::REVERSED),
            "hidden" => Some(Modifier::HIDDEN),
            "crossed_out" => Some(Modifier::CROSSED_OUT),
            _ => {
                warn!("unrecognized modifier in theme: {}", s);
                None
            }
        }
    } else {
        warn!("unrecognized modifier in theme: {}", value);
        None
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
}

#[test]
fn test_parse_style_string() {
    let fg = Value::String("#ffffff".to_string());

    let mut style = Style::default();
    parse_style(&mut style, fg);

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
    if let Value::Table(entries) = table {
        for (_name, value) in entries {
            parse_style(&mut style, value);
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
