use std::collections::HashMap;

use mlua::{Lua, Result, Table, Value};

use crate::types::{get_opt, table_to_string_vec};

/// What the user set for theme configuration.
#[derive(Debug, Clone)]
pub enum ThemeConfig {
    /// `si.theme.set("name")`
    Named(String),
    /// `si.theme.adaptive({ light = "...", dark = "...", fallback = "..." })`
    Adaptive {
        light: String,
        dark: String,
        fallback: Option<String>,
    },
    /// `si.theme.define("name", { ... })` — custom inline theme, auto-selected.
    Custom { name: String, spec: ThemeSpec },
}

/// A custom theme specification defined inline in Lua.
#[derive(Debug, Clone)]
pub struct ThemeSpec {
    pub inherits: Option<String>,
    pub palette: HashMap<String, String>,
    pub styles: HashMap<String, StyleSpec>,
}

/// A style specification for a single scope (e.g. `"ui.background"`).
#[derive(Debug, Clone)]
pub struct StyleSpec {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub underline: Option<UnderlineSpec>,
    pub modifiers: Vec<String>,
}

/// Underline style specification.
#[derive(Debug, Clone)]
pub struct UnderlineSpec {
    pub style: Option<String>,
    pub color: Option<String>,
}

/// Register `si.theme.set()`, `si.theme.adaptive()`, `si.theme.define()`.
pub fn register_theme_api(lua: &Lua, theme_table: &Table) -> Result<()> {
    // si.theme.set(name)
    theme_table.set(
        "set",
        lua.create_function(|lua, name: String| {
            lua.set_named_registry_value("si_theme", name)?;
            lua.set_named_registry_value("si_theme_type", "named")?;
            Ok(())
        })?,
    )?;

    // si.theme.adaptive({ light, dark, fallback })
    theme_table.set(
        "adaptive",
        lua.create_function(|lua, opts: Table| {
            lua.set_named_registry_value("si_theme_adaptive", opts)?;
            lua.set_named_registry_value("si_theme_type", "adaptive")?;
            Ok(())
        })?,
    )?;

    // si.theme.define(name, spec)
    theme_table.set(
        "define",
        lua.create_function(|lua, (name, spec): (String, Table)| {
            lua.set_named_registry_value("si_theme_define_name", name)?;
            lua.set_named_registry_value("si_theme_define_spec", spec)?;
            lua.set_named_registry_value("si_theme_type", "custom")?;
            Ok(())
        })?,
    )?;

    // Initialize as no theme set.
    lua.set_named_registry_value("si_theme_type", "none")?;

    Ok(())
}

/// Extract theme config from the Lua registry after script execution.
pub fn extract_theme_config(lua: &Lua) -> Result<Option<ThemeConfig>> {
    let theme_type: String = lua.named_registry_value("si_theme_type")?;

    match theme_type.as_str() {
        "none" => Ok(None),

        "named" => {
            let name: String = lua.named_registry_value("si_theme")?;
            Ok(Some(ThemeConfig::Named(name)))
        }

        "adaptive" => {
            let opts: Table = lua.named_registry_value("si_theme_adaptive")?;
            let light: String = opts.get("light")?;
            let dark: String = opts.get("dark")?;
            let fallback = get_opt::<String>(lua, &opts, "fallback");
            Ok(Some(ThemeConfig::Adaptive {
                light,
                dark,
                fallback,
            }))
        }

        "custom" => {
            let name: String = lua.named_registry_value("si_theme_define_name")?;
            let spec_table: Table = lua.named_registry_value("si_theme_define_spec")?;
            let spec = extract_theme_spec(lua, &spec_table)?;
            Ok(Some(ThemeConfig::Custom { name, spec }))
        }

        _ => Ok(None),
    }
}

fn extract_theme_spec(lua: &Lua, table: &Table) -> Result<ThemeSpec> {
    let inherits = get_opt::<String>(lua, table, "inherits");

    let mut palette = HashMap::new();
    if let Some(p) = get_opt::<Table>(lua, table, "palette") {
        for pair in p.pairs::<String, String>() {
            let (k, v) = pair?;
            palette.insert(k, v);
        }
    }

    let mut styles = HashMap::new();
    for pair in table.pairs::<String, Value>() {
        let (key, value) = pair?;
        if key == "inherits" || key == "palette" {
            continue;
        }
        if let Value::Table(style_table) = value {
            styles.insert(key, extract_style_spec(lua, &style_table)?);
        }
    }

    Ok(ThemeSpec {
        inherits,
        palette,
        styles,
    })
}

fn extract_style_spec(lua: &Lua, table: &Table) -> Result<StyleSpec> {
    let fg = get_opt::<String>(lua, table, "fg");
    let bg = get_opt::<String>(lua, table, "bg");
    let modifiers = match get_opt::<Table>(lua, table, "modifiers") {
        Some(mods) => table_to_string_vec(&mods)?,
        None => Vec::new(),
    };
    let underline = get_opt::<Table>(lua, table, "underline").map(|ul| UnderlineSpec {
        style: get_opt::<String>(lua, &ul, "style"),
        color: get_opt::<String>(lua, &ul, "color"),
    });

    Ok(StyleSpec {
        fg,
        bg,
        underline,
        modifiers,
    })
}

/// Convert a `ThemeSpec` to a `toml::Value` matching the TOML theme file format.
///
/// This allows custom themes defined in Lua to be loaded through the existing
/// theme parsing pipeline.
pub fn theme_spec_to_toml(spec: &ThemeSpec) -> toml::Value {
    let mut map = toml::map::Map::new();

    if let Some(inherits) = &spec.inherits {
        map.insert(
            "inherits".to_string(),
            toml::Value::String(inherits.clone()),
        );
    }

    if !spec.palette.is_empty() {
        let mut palette_map = toml::map::Map::new();
        for (k, v) in &spec.palette {
            palette_map.insert(k.clone(), toml::Value::String(v.clone()));
        }
        map.insert("palette".to_string(), toml::Value::Table(palette_map));
    }

    for (scope, style) in &spec.styles {
        map.insert(scope.clone(), style_spec_to_toml(style));
    }

    toml::Value::Table(map)
}

fn style_spec_to_toml(style: &StyleSpec) -> toml::Value {
    let mut map = toml::map::Map::new();

    if let Some(fg) = &style.fg {
        map.insert("fg".to_string(), toml::Value::String(fg.clone()));
    }
    if let Some(bg) = &style.bg {
        map.insert("bg".to_string(), toml::Value::String(bg.clone()));
    }
    if !style.modifiers.is_empty() {
        let mods: Vec<toml::Value> = style
            .modifiers
            .iter()
            .map(|m| toml::Value::String(m.clone()))
            .collect();
        map.insert("modifiers".to_string(), toml::Value::Array(mods));
    }
    if let Some(underline) = &style.underline {
        let mut ul_map = toml::map::Map::new();
        if let Some(s) = &underline.style {
            ul_map.insert("style".to_string(), toml::Value::String(s.clone()));
        }
        if let Some(c) = &underline.color {
            ul_map.insert("color".to_string(), toml::Value::String(c.clone()));
        }
        map.insert("underline".to_string(), toml::Value::Table(ul_map));
    }

    toml::Value::Table(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_set() {
        let config = crate::load_config_from_str(
            r#"
            si.theme.set("onedark")
        "#,
        )
        .unwrap();
        match config.theme {
            Some(ThemeConfig::Named(name)) => assert_eq!(name, "onedark"),
            other => panic!("expected Named theme, got {:?}", other),
        }
    }

    #[test]
    fn test_theme_adaptive() {
        let config = crate::load_config_from_str(
            r#"
            si.theme.adaptive({
                light = "solarized_light",
                dark = "onedark",
                fallback = "onedark",
            })
        "#,
        )
        .unwrap();
        match config.theme {
            Some(ThemeConfig::Adaptive {
                light,
                dark,
                fallback,
            }) => {
                assert_eq!(light, "solarized_light");
                assert_eq!(dark, "onedark");
                assert_eq!(fallback, Some("onedark".into()));
            }
            other => panic!("expected Adaptive theme, got {:?}", other),
        }
    }

    #[test]
    fn test_theme_adaptive_no_fallback() {
        let config = crate::load_config_from_str(
            r#"
            si.theme.adaptive({
                light = "solarized_light",
                dark = "onedark",
            })
        "#,
        )
        .unwrap();
        match config.theme {
            Some(ThemeConfig::Adaptive { fallback, .. }) => {
                assert!(fallback.is_none());
            }
            other => panic!("expected Adaptive theme, got {:?}", other),
        }
    }

    #[test]
    fn test_theme_define() {
        let config = crate::load_config_from_str(
            r##"
            si.theme.define("my_theme", {
                inherits = "onedark",
                ["ui.background"] = { bg = "#000000" },
                ["ui.cursor"] = { bg = "#e94560", modifiers = { "bold" } },
                palette = { red = "#ff0000" },
            })
        "##,
        )
        .unwrap();
        match config.theme {
            Some(ThemeConfig::Custom { name, spec }) => {
                assert_eq!(name, "my_theme");
                assert_eq!(spec.inherits, Some("onedark".into()));
                assert!(spec.styles.contains_key("ui.background"));
                assert!(spec.styles.contains_key("ui.cursor"));
                assert_eq!(spec.styles["ui.background"].bg, Some("#000000".into()));
                assert_eq!(
                    spec.styles["ui.cursor"].modifiers,
                    vec!["bold".to_string()]
                );
                assert!(spec.palette.contains_key("red"));
                assert_eq!(spec.palette["red"], "#ff0000");
            }
            other => panic!("expected Custom theme, got {:?}", other),
        }
    }

    #[test]
    fn test_theme_define_with_underline() {
        let config = crate::load_config_from_str(
            r##"
            si.theme.define("ul_theme", {
                ["diagnostic.warning"] = {
                    underline = { style = "curl", color = "#ff9900" },
                },
            })
        "##,
        )
        .unwrap();
        match config.theme {
            Some(ThemeConfig::Custom { spec, .. }) => {
                let style = &spec.styles["diagnostic.warning"];
                let ul = style.underline.as_ref().unwrap();
                assert_eq!(ul.style, Some("curl".into()));
                assert_eq!(ul.color, Some("#ff9900".into()));
            }
            other => panic!("expected Custom theme, got {:?}", other),
        }
    }

    #[test]
    fn test_no_theme_returns_none() {
        let config = crate::load_config_from_str("-- no theme").unwrap();
        assert!(config.theme.is_none());
    }

    #[test]
    fn test_last_theme_call_wins() {
        let config = crate::load_config_from_str(
            r#"
            si.theme.set("first")
            si.theme.set("second")
        "#,
        )
        .unwrap();
        match config.theme {
            Some(ThemeConfig::Named(name)) => assert_eq!(name, "second"),
            other => panic!("expected Named theme, got {:?}", other),
        }
    }

    #[test]
    fn test_define_overrides_set() {
        let config = crate::load_config_from_str(
            r##"
            si.theme.set("onedark")
            si.theme.define("custom", { ["keyword"] = { fg = "#ff0000" } })
        "##,
        )
        .unwrap();
        assert!(matches!(config.theme, Some(ThemeConfig::Custom { .. })));
    }

    #[test]
    fn test_theme_spec_to_toml() {
        let spec = ThemeSpec {
            inherits: Some("parent".into()),
            palette: HashMap::from([("red".into(), "#ff0000".into())]),
            styles: HashMap::from([(
                "keyword".into(),
                StyleSpec {
                    fg: Some("#e94560".into()),
                    bg: None,
                    underline: None,
                    modifiers: vec!["bold".into()],
                },
            )]),
        };
        let toml_val = theme_spec_to_toml(&spec);
        let table = toml_val.as_table().unwrap();
        assert_eq!(
            table.get("inherits").unwrap().as_str().unwrap(),
            "parent"
        );
        let palette = table.get("palette").unwrap().as_table().unwrap();
        assert_eq!(palette.get("red").unwrap().as_str().unwrap(), "#ff0000");
        let keyword = table.get("keyword").unwrap().as_table().unwrap();
        assert_eq!(keyword.get("fg").unwrap().as_str().unwrap(), "#e94560");
    }
}
