use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    str,
};

use anyhow::{anyhow, Result};
use helix_core::hashmap;
use helix_loader::merge_toml_values;
use log::warn;
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer};
use toml::{map::Map, Value};

use crate::graphics::UnderlineStyle;
pub use crate::graphics::{Color, Modifier, Style};

pub static DEFAULT_THEME_DATA: Lazy<Value> = Lazy::new(|| {
    let bytes = include_bytes!("../../theme.toml");
    toml::from_str(str::from_utf8(bytes).unwrap()).expect("Failed to parse base default theme")
});

pub static BASE16_DEFAULT_THEME_DATA: Lazy<Value> = Lazy::new(|| {
    let bytes = include_bytes!("../../base16_theme.toml");
    toml::from_str(str::from_utf8(bytes).unwrap()).expect("Failed to parse base 16 default theme")
});

pub static DEFAULT_THEME: Lazy<Theme> = Lazy::new(|| Theme {
    name: "default".into(),
    ..Theme::from(DEFAULT_THEME_DATA.clone())
});

pub static BASE16_DEFAULT_THEME: Lazy<Theme> = Lazy::new(|| Theme {
    name: "base16_default".into(),
    ..Theme::from(BASE16_DEFAULT_THEME_DATA.clone())
});

#[derive(Clone, Debug)]
pub struct Loader {
    /// Theme directories to search from highest to lowest priority
    theme_dirs: Vec<PathBuf>,
}
impl Loader {
    /// Creates a new loader that can load themes from multiple directories.
    ///
    /// The provided directories should be ordered from highest to lowest priority.
    /// The directories will have their "themes" subdirectory searched.
    pub fn new(dirs: &[PathBuf]) -> Self {
        Self {
            theme_dirs: dirs.iter().map(|p| p.join("themes")).collect(),
        }
    }

    /// Loads a theme searching directories in priority order.
    pub fn load(&self, name: &str) -> Result<Theme> {
        if name == "default" {
            return Ok(self.default());
        }
        if name == "base16_default" {
            return Ok(self.base16_default());
        }

        let mut visited_paths = HashSet::new();
        let theme = self.load_theme(name, &mut visited_paths).map(Theme::from)?;

        Ok(Theme {
            name: name.into(),
            ..theme
        })
    }

    /// Recursively load a theme, merging with any inherited parent themes.
    ///
    /// The paths that have been visited in the inheritance hierarchy are tracked
    /// to detect and avoid cycling.
    ///
    /// It is possible for one file to inherit from another file with the same name
    /// so long as the second file is in a themes directory with lower priority.
    /// However, it is not recommended that users do this as it will make tracing
    /// errors more difficult.
    fn load_theme(&self, name: &str, visited_paths: &mut HashSet<PathBuf>) -> Result<Value> {
        let path = self.path(name, visited_paths)?;

        let theme_toml = self.load_toml(path)?;

        let inherits = theme_toml.get("inherits");

        let theme_toml = if let Some(parent_theme_name) = inherits {
            let parent_theme_name = parent_theme_name.as_str().ok_or_else(|| {
                anyhow!(
                    "Theme: expected 'inherits' to be a string: {}",
                    parent_theme_name
                )
            })?;

            let parent_theme_toml = match parent_theme_name {
                // load default themes's toml from const.
                "default" => DEFAULT_THEME_DATA.clone(),
                "base16_default" => BASE16_DEFAULT_THEME_DATA.clone(),
                _ => self.load_theme(parent_theme_name, visited_paths)?,
            };

            self.merge_themes(parent_theme_toml, theme_toml)
        } else {
            theme_toml
        };

        Ok(theme_toml)
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

    // merge one theme into the parent theme
    fn merge_themes(&self, parent_theme_toml: Value, theme_toml: Value) -> Value {
        let parent_palette = parent_theme_toml.get("palette");
        let palette = theme_toml.get("palette");

        // handle the table separately since it needs a `merge_depth` of 2
        // this would conflict with the rest of the theme merge strategy
        let palette_values = match (parent_palette, palette) {
            (Some(parent_palette), Some(palette)) => {
                merge_toml_values(parent_palette.clone(), palette.clone(), 2)
            }
            (Some(parent_palette), None) => parent_palette.clone(),
            (None, Some(palette)) => palette.clone(),
            (None, None) => Map::new().into(),
        };

        // add the palette correctly as nested table
        let mut palette = Map::new();
        palette.insert(String::from("palette"), palette_values);

        // merge the theme into the parent theme
        let theme = merge_toml_values(parent_theme_toml, theme_toml, 1);
        // merge the before specially handled palette into the theme
        merge_toml_values(theme, palette.into(), 1)
    }

    // Loads the theme data as `toml::Value`
    fn load_toml(&self, path: PathBuf) -> Result<Value> {
        let data = std::fs::read_to_string(path)?;
        let value = toml::from_str(&data)?;

        Ok(value)
    }

    /// Returns the path to the theme with the given name
    ///
    /// Ignores paths already visited and follows directory priority order.
    fn path(&self, name: &str, visited_paths: &mut HashSet<PathBuf>) -> Result<PathBuf> {
        let filename = format!("{}.toml", name);

        let mut cycle_found = false; // track if there was a path, but it was in a cycle
        self.theme_dirs
            .iter()
            .find_map(|dir| {
                let path = dir.join(&filename);
                if !path.exists() {
                    None
                } else if visited_paths.contains(&path) {
                    // Avoiding cycle, continuing to look in lower priority directories
                    cycle_found = true;
                    None
                } else {
                    visited_paths.insert(path.clone());
                    Some(path)
                }
            })
            .ok_or_else(|| {
                if cycle_found {
                    anyhow!("Theme: cycle found in inheriting: {}", name)
                } else {
                    anyhow!("Theme: file not found for: {}", name)
                }
            })
    }

    pub fn default_theme(&self, true_color: bool) -> Theme {
        if true_color {
            self.default()
        } else {
            self.base16_default()
        }
    }

    /// Returns the default theme
    pub fn default(&self) -> Theme {
        DEFAULT_THEME.clone()
    }

    /// Returns the alternative 16-color default theme
    pub fn base16_default(&self) -> Theme {
        BASE16_DEFAULT_THEME.clone()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Theme {
    name: String,

    // UI styles are stored in a HashMap
    styles: HashMap<String, Style>,
    // tree-sitter highlight styles are stored in a Vec to optimize lookups
    scopes: Vec<String>,
    highlights: Vec<Style>,
}

impl From<Value> for Theme {
    fn from(value: Value) -> Self {
        if let Value::Table(table) = value {
            let (styles, scopes, highlights) = build_theme_values(table);

            Self {
                styles,
                scopes,
                highlights,
                ..Default::default()
            }
        } else {
            warn!("Expected theme TOML value to be a table, found {:?}", value);
            Default::default()
        }
    }
}

impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = Map::<String, Value>::deserialize(deserializer)?;

        let (styles, scopes, highlights) = build_theme_values(values);

        Ok(Self {
            styles,
            scopes,
            highlights,
            ..Default::default()
        })
    }
}

fn build_theme_values(
    mut values: Map<String, Value>,
) -> (HashMap<String, Style>, Vec<String>, Vec<Style>) {
    let mut styles = HashMap::new();
    let mut scopes = Vec::new();
    let mut highlights = Vec::new();

    // TODO: alert user of parsing failures in editor
    let palette = values
        .remove("palette")
        .map(|value| {
            ThemePalette::try_from(value).unwrap_or_else(|err| {
                warn!("{}", err);
                ThemePalette::default()
            })
        })
        .unwrap_or_default();
    // remove inherits from value to prevent errors
    let _ = values.remove("inherits");
    styles.reserve(values.len());
    scopes.reserve(values.len());
    highlights.reserve(values.len());
    for (name, style_value) in values {
        let mut style = Style::default();
        if let Err(err) = palette.parse_style(&mut style, style_value) {
            warn!("{}", err);
        }

        // these are used both as UI and as highlights
        styles.insert(name.clone(), style);
        scopes.push(name);
        highlights.push(style);
    }

    (styles, scopes, highlights)
}

impl Theme {
    #[inline]
    pub fn highlight(&self, index: usize) -> Style {
        self.highlights[index]
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get(&self, scope: &str) -> Style {
        self.try_get(scope).unwrap_or_default()
    }

    /// Get the style of a scope, falling back to dot separated broader
    /// scopes. For example if `ui.text.focus` is not defined in the theme,
    /// `ui.text` is tried and then `ui` is tried.
    pub fn try_get(&self, scope: &str) -> Option<Style> {
        std::iter::successors(Some(scope), |s| Some(s.rsplit_once('.')?.0))
            .find_map(|s| self.styles.get(s).copied())
    }

    /// Get the style of a scope, without falling back to dot separated broader
    /// scopes. For example if `ui.text.focus` is not defined in the theme, it
    /// will return `None`, even if `ui.text` is.
    pub fn try_get_exact(&self, scope: &str) -> Option<Style> {
        self.styles.get(scope).copied()
    }

    #[inline]
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    pub fn find_scope_index_exact(&self, scope: &str) -> Option<usize> {
        self.scopes().iter().position(|s| s == scope)
    }

    pub fn find_scope_index(&self, mut scope: &str) -> Option<usize> {
        loop {
            if let Some(highlight) = self.find_scope_index_exact(scope) {
                return Some(highlight);
            }
            if let Some(new_end) = scope.rfind('.') {
                scope = &scope[..new_end];
            } else {
                return None;
            }
        }
    }

    pub fn is_16_color(&self) -> bool {
        self.styles.iter().all(|(_, style)| {
            [style.fg, style.bg]
                .into_iter()
                .all(|color| !matches!(color, Some(Color::Rgb(..))))
        })
    }
}

struct ThemePalette {
    palette: HashMap<String, Color>,
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self {
            palette: hashmap! {
                "reset".to_string() => Color::Reset,
                "black".to_string() => Color::Black,
                "red".to_string() => Color::Red,
                "green".to_string() => Color::Green,
                "yellow".to_string() => Color::Yellow,
                "blue".to_string() => Color::Blue,
                "magenta".to_string() => Color::Magenta,
                "cyan".to_string() => Color::Cyan,
                "gray".to_string() => Color::Gray,
                "light-red".to_string() => Color::LightRed,
                "light-green".to_string() => Color::LightGreen,
                "light-yellow".to_string() => Color::LightYellow,
                "light-blue".to_string() => Color::LightBlue,
                "light-magenta".to_string() => Color::LightMagenta,
                "light-cyan".to_string() => Color::LightCyan,
                "light-gray".to_string() => Color::LightGray,
                "white".to_string() => Color::White,
            },
        }
    }
}

impl ThemePalette {
    pub fn new(palette: HashMap<String, Color>) -> Self {
        let ThemePalette {
            palette: mut default,
        } = ThemePalette::default();

        default.extend(palette);
        Self { palette: default }
    }

    pub fn string_to_rgb(s: &str) -> Result<Color, String> {
        if s.starts_with('#') {
            Self::hex_string_to_rgb(s)
        } else {
            Self::ansi_string_to_rgb(s)
        }
    }

    fn ansi_string_to_rgb(s: &str) -> Result<Color, String> {
        if let Ok(index) = s.parse::<u8>() {
            return Ok(Color::Indexed(index));
        }
        Err(format!("Theme: malformed ANSI: {}", s))
    }

    fn hex_string_to_rgb(s: &str) -> Result<Color, String> {
        if s.len() >= 7 {
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
            .or_else(|_| Self::string_to_rgb(value))
    }

    pub fn parse_modifier(value: &Value) -> Result<Modifier, String> {
        value
            .as_str()
            .and_then(|s| s.parse().ok())
            .ok_or(format!("Theme: invalid modifier: {}", value))
    }

    pub fn parse_underline_style(value: &Value) -> Result<UnderlineStyle, String> {
        value
            .as_str()
            .and_then(|s| s.parse().ok())
            .ok_or(format!("Theme: invalid underline style: {}", value))
    }

    pub fn parse_style(&self, style: &mut Style, value: Value) -> Result<(), String> {
        if let Value::Table(entries) = value {
            for (name, mut value) in entries {
                match name.as_str() {
                    "fg" => *style = style.fg(self.parse_color(value)?),
                    "bg" => *style = style.bg(self.parse_color(value)?),
                    "underline" => {
                        let table = value
                            .as_table_mut()
                            .ok_or("Theme: underline must be table")?;
                        if let Some(value) = table.remove("color") {
                            *style = style.underline_color(self.parse_color(value)?);
                        }
                        if let Some(value) = table.remove("style") {
                            *style = style.underline_style(Self::parse_underline_style(&value)?);
                        }

                        if let Some(attr) = table.keys().next() {
                            return Err(format!("Theme: invalid underline attribute: {attr}"));
                        }
                    }
                    "modifiers" => {
                        let modifiers = value
                            .as_array()
                            .ok_or("Theme: modifiers should be an array")?;

                        for modifier in modifiers {
                            if modifier
                                .as_str()
                                .map_or(false, |modifier| modifier == "underlined")
                            {
                                *style = style.underline_style(UnderlineStyle::Line);
                            } else {
                                *style = style.add_modifier(Self::parse_modifier(modifier)?);
                            }
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
            let color = Self::string_to_rgb(value)?;
            palette.insert(name, color);
        }

        Ok(Self::new(palette))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        for (_name, value) in table {
            palette.parse_style(&mut style, value).unwrap();
        }

        assert_eq!(
            style,
            Style::default()
                .fg(Color::Rgb(255, 255, 255))
                .bg(Color::Rgb(0, 0, 0))
                .add_modifier(Modifier::BOLD)
        );
    }
}
