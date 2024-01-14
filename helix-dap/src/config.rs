use anyhow::bail;
use helix_config::*;
use serde::{Deserialize, Serialize};

options! {
    struct DebugAdapterConfig {
        #[name = "debugger.name"]
        name: Option<String> = None,
        #[name = "debugger.transport"]
        #[read = copy]
        transport: Transport = Transport::Stdio,
        #[name = "debugger.command"]
        #[read = deref]
        command: String = "",
        #[name = "debugger.args"]
        #[read = deref]
        args: List<String> = List::default(),
        #[name = "debugger.port-arg"]
        #[read = deref]
        port_arg: String = "",
        #[name = "debugger.templates"]
        #[read = deref]
        templates: List<DebugTemplate> = List::default(),
        #[name = "debugger.quirks.absolut-path"]
        #[read = copy]
        absolut_path: bool = false,
        #[name = "terminal.command"]
        terminal_command: Option<String> = get_terminal_provider().map(|term| term.command),
        #[name = "terminal.args"]
        #[read = deref]
        terminal_args: List<String> = get_terminal_provider().map(|term| term.args.into_boxed_slice()).unwrap_or_default(),
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Transport {
    Stdio,
    Tcp,
}

impl Ty for Transport {
    fn from_value(val: Value) -> anyhow::Result<Self> {
        match &*String::from_value(val)? {
            "stdio" => Ok(Transport::Stdio),
            "tcp" => Ok(Transport::Tcp),
            val => bail!("expected 'stdio' or 'tcp' (got {val:?})"),
        }
    }
    fn to_value(&self) -> Value {
        match self {
            Transport::Stdio => "stdio".into(),
            Transport::Tcp => "tcp".into(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DebugArgumentValue {
    String(String),
    Array(Vec<String>),
    Boolean(bool),
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AdvancedCompletion {
    pub name: Option<String>,
    pub completion: Option<String>,
    pub default: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", untagged)]
pub enum DebugConfigCompletion {
    Named(String),
    Advanced(AdvancedCompletion),
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DebugTemplate {
    pub name: String,
    pub request: String,
    pub completion: Vec<DebugConfigCompletion>,
    pub args: Map<DebugArgumentValue>,
}

// TODO: integrate this better with the new config system (less nesting)
// the best way to do that is probably a rewrite. I think these templates
// are probably overkill here. This may be easier to solve by moving the logic
// to scheme
config_serde_adapter!(DebugTemplate);

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct TerminalConfig {
    pub command: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[cfg(windows)]
pub fn get_terminal_provider() -> Option<TerminalConfig> {
    use helix_config::env::binary_exists;

    if binary_exists("wt") {
        return Some(TerminalConfig {
            command: "wt".into(),
            args: vec![
                "new-tab".into(),
                "--title".into(),
                "DEBUG".into(),
                "cmd".into(),
                "/C".into(),
            ],
        });
    }

    Some(TerminalConfig {
        command: "conhost".into(),
        args: vec!["cmd".into(), "/C".into()],
    })
}

#[cfg(not(any(windows, target_os = "wasm32")))]
fn get_terminal_provider() -> Option<TerminalConfig> {
    use helix_config::env::{binary_exists, env_var_is_set};

    if env_var_is_set("TMUX") && binary_exists("tmux") {
        return Some(TerminalConfig {
            command: "tmux".into(),
            args: vec!["split-window".into()],
        });
    }

    if env_var_is_set("WEZTERM_UNIX_SOCKET") && binary_exists("wezterm") {
        return Some(TerminalConfig {
            command: "wezterm".into(),
            args: vec!["cli".into(), "split-pane".into()],
        });
    }

    None
}
