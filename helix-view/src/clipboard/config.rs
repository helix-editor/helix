use std::collections::HashMap;
use std::ffi::OsStr;

use log::warn;
use serde::{Deserialize, Serialize};

use super::{provider, ClipboardProvider};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(from = "CustomClipboardConfig")]
pub struct ClipboardConfig {
    providers: HashMap<String, ClipboardProviderSpec>,
    order: Vec<String>,
}

#[derive(Deserialize)]
struct CustomClipboardConfig {
    #[serde(rename = "provider")]
    providers: HashMap<String, ClipboardProviderSpec>,
    order: Vec<String>,
}

impl From<CustomClipboardConfig> for ClipboardConfig {
    fn from(cfg: CustomClipboardConfig) -> Self {
        let mut providers = Self::default().providers;
        providers.extend(cfg.providers);
        let order = cfg.order;
        Self { providers, order }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
struct ClipboardProviderSpec {
    copy: Vec<String>,
    paste: Vec<String>,
    primary_copy: Option<Vec<String>>,
    primary_paste: Option<Vec<String>>,
    #[serde(default)]
    env_vars: Vec<String>,
    #[serde(default)]
    test_commands: Vec<Vec<String>>,
}

macro_rules! command_provider_spec {
    (
        copy: $($get:literal),+ ;
        paste: $($set:literal),+ ;
        $(
            primary_copy: $($pget:literal),+ ;
            primary_paste: $($pset:literal),+ ;
        )?
        $( env: $($env:literal),+ ; )?
        $( test: $($test:literal),+ ; )*
    ) => {
        #[allow(clippy::needless_update)]
        ClipboardProviderSpec {
            copy: vec![$($get.to_owned()),+],
            paste: vec![$($set.to_owned()),+],
            $(
                primary_copy: Some(vec![$($pget.to_owned()),+]),
                primary_paste: Some(vec![$($pset.to_owned()),+]),
            )?
            $( env_vars: vec![$($env.to_owned()),+], )?
            test_commands: vec![$(vec![$($test.to_owned()),+]),*],
            ..Default::default()
        }
    };
}

macro_rules! clipboard_config {
    ($(
        $(#[$attr:meta])*
        $name:literal {
            $($body:tt)*
        }
    )*) => {
        ClipboardConfig {
            providers: HashMap::from([$(
                $(#[$attr])*
                ($name.to_owned(), command_provider_spec! { $($body)* })
            ),*]),
            order: vec![$(
                $(#[$attr])*
                $name.to_owned()
            ),*],
        }
    };
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        let mut cfg = clipboard_config! {
            #[cfg(unix)]
            "wl-clipboard" {
                copy: "wl-copy", "--type", "text/plain";
                paste: "wl-paste", "--no-newline";
                primary_copy: "wl-copy", "-p", "--type", "text/plain";
                primary_paste: "wl-paste", "-p", "--no-newline";
                env: "WAYLAND_DISPLAY";
            }

            #[cfg(unix)]
            "xclip" {
                copy: "xclip", "-i", "-selection", "clipboard";
                paste: "xclip", "-o", "-selection", "clipboard";
                primary_copy: "xclip", "-i";
                primary_paste: "xclip", "-o";
                env: "DISPLAY";
            }

            #[cfg(unix)]
            "xsel" {
                copy: "xsel", "-o", "-b";
                paste: "xsel", "-i", "-b";
                primary_copy: "xsel", "-o";
                primary_paste: "xsel", "-i";
                env: "DISPLAY";
                test: "xsel", "-o", "-b";
            }

            "lemonade" {
                copy: "lemonade", "copy";
                paste: "lemonade", "paste";
            }

            #[cfg(unix)]
            "doit" {
                copy: "doitclient", "wclip";
                paste: "doitclient", "wclip", "-r";
            }

            #[cfg(any(windows, target_os = "linux"))] // this is a godsend on WSL
            "win32yank" {
                copy: "win32yank.exe", "-i";
                paste: "win32yank.exe", "-o";
            }

            #[cfg(unix)]
            "tmux" {
                copy: "tmux", "load-buffer", "-";
                paste: "tmux", "save-buffer", "-";
                env: "TMUX";
            }

            #[cfg(target_os = "linux")]
            "termux" {
                copy: "termux-clipboard-set";
                paste: "termux-clipboard-get";
            }

            #[cfg(target_os = "macos")]
            "pbcopy+pbpaste" {
                copy: "pbcopy";
                paste: "pbpaste";
            }
        };
        if cfg!(windows) {
            cfg.order.push("clipboard-win".to_owned());
        }
        cfg
    }
}

impl ClipboardConfig {
    pub fn get_provider(&self) -> Box<dyn ClipboardProvider> {
        for name in &self.order {
            #[cfg(windows)]
            if name == "clipboard-win" {
                return Box::new(provider::WindowsProvider);
            }
            if let Some(spec) = self.providers.get(name) {
                if spec.is_available() {
                    return Box::new(provider::CommandProvider::from(spec));
                }
            } else {
                warn!("No clipboard provider named {name}");
                continue;
            }
        }

        Box::new(provider::NopProvider::new())
    }
}

impl ClipboardProviderSpec {
    pub fn is_available(&self) -> bool {
        macro_rules! check {
            ($cond:expr) => {
                if !$cond {
                    return false;
                }
            };
        }

        check!(!self.copy.is_empty() && exists(&self.copy[0]));
        check!(!self.paste.is_empty() && exists(&self.paste[0]));

        if let Some(copy) = &self.primary_copy {
            check!(!copy.is_empty() && exists(&copy[0]));
        }
        if let Some(paste) = &self.primary_paste {
            check!(!paste.is_empty() && exists(&paste[0]));
        }
        for var in &self.env_vars {
            check!(env_var_is_set(var));
        }
        for cmd in &self.test_commands {
            // FIXME: check performance of is_exit_success
            check!(!cmd.is_empty() && is_exit_success(&cmd[0], &cmd[1..]))
        }

        true
    }
}

impl From<&ClipboardProviderSpec> for provider::CommandProvider {
    fn from(spec: &ClipboardProviderSpec) -> Self {
        let cmd = |argv: &[String]| provider::CommandConfig {
            prg: argv[0].clone(),
            args: argv[1..].to_vec(),
        };

        provider::CommandProvider {
            get_cmd: cmd(&spec.copy),
            set_cmd: cmd(&spec.paste),
            get_primary_cmd: spec.primary_copy.as_deref().map(cmd),
            set_primary_cmd: spec.primary_paste.as_deref().map(cmd),
        }
    }
}

fn exists(executable_name: &str) -> bool {
    which::which(executable_name).is_ok()
}

fn env_var_is_set(env_var_name: &str) -> bool {
    std::env::var_os(env_var_name).is_some()
}

fn is_exit_success(program: impl AsRef<OsStr>, args: &[impl AsRef<OsStr>]) -> bool {
    std::process::Command::new(program)
        .args(args)
        .output()
        .ok()
        .and_then(|out| out.status.success().then(|| ())) // TODO: use then_some when stabilized
        .is_some()
}
