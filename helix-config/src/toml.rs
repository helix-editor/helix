use crate::{Map, OptionManager, OptionRegistry, Value};

/// Inserts the config declaration from a map deserialized from toml into
/// options manager. Returns an error if any of the config options are
/// invalid. The conversion may not be exactly one-to-one to retain backwards
/// compatibility
pub fn read_toml_config(
    config_entries: Map<Value>,
    options: &OptionManager,
    registry: &OptionRegistry,
) -> anyhow::Result<()> {
    let mut buf = String::new();
    for (key, val) in config_entries {
        if matches!(val, Value::Map(_)) {
            buf.push_str(&key);
            visit(&mut buf, val, options, registry)?;
            buf.clear();
        } else {
            visit(&mut key.to_string(), val, options, registry)?;
        }
    }
    Ok(())
}

fn visit(
    path: &mut String,
    val: Value,
    options: &OptionManager,
    registry: &OptionRegistry,
) -> anyhow::Result<()> {
    match &**path {
        // don't descend
        "auto-format" => {
            // treat as unset
            if Value::Bool(true) == val {
                return Ok(());
            }
        }
        "auto-pairs" => return options.set("auto-pairs", val, registry),
        "environment" => return options.set("environment", val, registry),
        "config" => return options.set("config", val, registry),
        "gutters" if matches!(val, Value::List(_)) => {
            return options.set("gutters.layout", val, registry);
        }
        "whitespace.render" if matches!(val, Value::String(_)) => {
            return options.set("whitespace.render.default", val, registry);
        }
        "language-servers" => {
            // merge list/map of language servers but if "only" and "except" are specified overwrite
            return options.append("language-servers", val, registry, 0);
        }
        _ => (),
    };
    if let Value::Map(val) = val {
        let old_path_len = path.len();
        for (key, val) in val.into_iter() {
            path.push('.');
            path.push_str(&key);
            visit(path, val, options, registry)?;
            path.truncate(old_path_len);
        }
        Ok(())
    } else {
        options.set(&**path, val, registry)
    }
}
