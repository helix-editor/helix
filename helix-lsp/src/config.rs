use std::collections::HashMap;

use anyhow::bail;
use helix_config::{options, List, Map, String, Ty, Value};

use crate::lsp;

// TODO: differentiating between Some(null) and None is not really practical
// since the distinction is lost on a roundtrip through config::Value.
// Probably better to change our code to treat null the way we currently
// treat None
options! {
    struct LanguageServerConfig {
        /// The name or path of the language server binary to execute. Binaries must be in `$PATH`
        command: Option<String> = None,
        /// A list of arguments to pass to the language server binary
        #[read = deref]
        args: List<String> = List::default(),
        /// Any environment variables that will be used when starting the language server
        environment: Map<String> = Map::default(),
        /// LSP initialization options
        #[name = "config"]
        server_config: Option<Box<serde_json::Value>> = None,
        /// LSP initialization options
        #[read = copy]
        timeout: u64 = 20,
        // TODO: merge
        /// LSP formatting options
        #[name = "config.format"]
        #[read = fold(HashMap::new(), fold_format_config, FormatConfig)]
        format: Map<FormattingProperty> = Map::default()
    }
}

type FormatConfig = HashMap<std::string::String, lsp::FormattingProperty>;

fn fold_format_config(config: &Map<FormattingProperty>, mut res: FormatConfig) -> FormatConfig {
    for (k, v) in config.iter() {
        res.entry(k.to_string()).or_insert_with(|| v.0.clone());
    }
    res
}

// damn orphan rules :/
#[derive(Debug, PartialEq, Clone)]
struct FormattingProperty(lsp::FormattingProperty);

impl Ty for FormattingProperty {
    fn from_value(val: Value) -> anyhow::Result<Self> {
        match val {
            Value::Int(_) => Ok(FormattingProperty(lsp::FormattingProperty::Number(
                i32::from_value(val)?,
            ))),
            Value::Bool(val) => Ok(FormattingProperty(lsp::FormattingProperty::Bool(val))),
            Value::String(val) => Ok(FormattingProperty(lsp::FormattingProperty::String(val))),
            _ => bail!("expected a string, boolean or integer"),
        }
    }

    fn to_value(&self) -> Value {
        match self.0 {
            lsp::FormattingProperty::Bool(val) => Value::Bool(val),
            lsp::FormattingProperty::Number(val) => Value::Int(val as _),
            lsp::FormattingProperty::String(ref val) => Value::String(val.clone()),
        }
    }
}
