// Implementation reference: https://github.com/neovim/neovim/blob/f2906a4669a2eef6d7bf86a29648793d63c98949/runtime/autoload/provider/clipboard.vim#L68-L152

use anyhow::Result;
use std::borrow::Cow;

pub enum ClipboardType {
    Clipboard,
    Selection,
}

pub trait ClipboardProvider: std::fmt::Debug {
    fn name(&self) -> Cow<str>;
    fn get_contents(&self, clipboard_type: ClipboardType) -> Result<String>;
    fn set_contents(&mut self, contents: String, clipboard_type: ClipboardType) -> Result<()>;
}

#[cfg(not(windows))]
macro_rules! command_provider {
    (paste => $get_prg:literal $( , $get_arg:literal )* ; copy => $set_prg:literal $( , $set_arg:literal )* ; ) => {{
        log::info!(
            "Using {} to interact with the system clipboard",
            if $set_prg != $get_prg { format!("{}+{}", $set_prg, $get_prg)} else { $set_prg.to_string() }
        );
        Box::new(provider::command::Provider {
            get_cmd: provider::command::Config {
                prg: $get_prg,
                args: &[ $( $get_arg ),* ],
            },
            set_cmd: provider::command::Config {
                prg: $set_prg,
                args: &[ $( $set_arg ),* ],
            },
            get_primary_cmd: None,
            set_primary_cmd: None,
        })
    }};

    (paste => $get_prg:literal $( , $get_arg:literal )* ;
     copy => $set_prg:literal $( , $set_arg:literal )* ;
     primary_paste => $pr_get_prg:literal $( , $pr_get_arg:literal )* ;
     primary_copy => $pr_set_prg:literal $( , $pr_set_arg:literal )* ;
    ) => {{
        log::info!(
            "Using {} to interact with the system and selection (primary) clipboard",
            if $set_prg != $get_prg { format!("{}+{}", $set_prg, $get_prg)} else { $set_prg.to_string() }
        );
        Box::new(provider::command::Provider {
            get_cmd: provider::command::Config {
                prg: $get_prg,
                args: &[ $( $get_arg ),* ],
            },
            set_cmd: provider::command::Config {
                prg: $set_prg,
                args: &[ $( $set_arg ),* ],
            },
            get_primary_cmd: Some(provider::command::Config {
                prg: $pr_get_prg,
                args: &[ $( $pr_get_arg ),* ],
            }),
            set_primary_cmd: Some(provider::command::Config {
                prg: $pr_set_prg,
                args: &[ $( $pr_set_arg ),* ],
            }),
        })
    }};
}

#[cfg(windows)]
pub fn get_clipboard_provider() -> Box<dyn ClipboardProvider> {
    Box::new(provider::WindowsProvider::default())
}

#[cfg(target_os = "macos")]
pub fn get_clipboard_provider() -> Box<dyn ClipboardProvider> {
    use provider::command::exists;

    if exists("pbcopy") && exists("pbpaste") {
        command_provider! {
            paste => "pbpaste";
            copy => "pbcopy";
        }
    } else {
        Box::new(provider::NopProvider::new())
    }
}

#[cfg(target_os = "wasm32")]
pub fn get_clipboard_provider() -> Box<dyn ClipboardProvider> {
    // TODO:
    Box::new(provider::NopProvider::new())
}

#[cfg(not(any(windows, target_os = "wasm32", target_os = "macos")))]
pub fn get_clipboard_provider() -> Box<dyn ClipboardProvider> {
    use provider::command::{env_var_is_set, exists, is_exit_success};
    // TODO: support for user-defined provider, probably when we have plugin support by setting a
    // variable?

    if env_var_is_set("WAYLAND_DISPLAY") && exists("wl-copy") && exists("wl-paste") {
        command_provider! {
            paste => "wl-paste", "--no-newline";
            copy => "wl-copy", "--type", "text/plain";
            primary_paste => "wl-paste", "-p", "--no-newline";
            primary_copy => "wl-copy", "-p", "--type", "text/plain";
        }
    } else if env_var_is_set("DISPLAY") && exists("xclip") {
        command_provider! {
            paste => "xclip", "-o", "-selection", "clipboard";
            copy => "xclip", "-i", "-selection", "clipboard";
            primary_paste => "xclip", "-o";
            primary_copy => "xclip", "-i";
        }
    } else if env_var_is_set("DISPLAY") && exists("xsel") && is_exit_success("xsel", &["-o", "-b"])
    {
        // FIXME: check performance of is_exit_success
        command_provider! {
            paste => "xsel", "-o", "-b";
            copy => "xsel", "-i", "-b";
            primary_paste => "xsel", "-o";
            primary_copy => "xsel", "-i";
        }
    } else if exists("win32yank.exe") {
        command_provider! {
            paste => "win32yank.exe", "-o", "--lf";
            copy => "win32yank.exe", "-i", "--crlf";
        }
    } else if exists("termux-clipboard-set") && exists("termux-clipboard-get") {
        command_provider! {
            paste => "termux-clipboard-get";
            copy => "termux-clipboard-set";
        }
    } else if env_var_is_set("TMUX") && exists("tmux") {
        command_provider! {
            paste => "tmux", "save-buffer", "-";
            copy => "tmux", "load-buffer", "-";
        }
    } else {
        Box::new(provider::NopProvider::new())
    }
}

pub mod provider {
    use super::{ClipboardProvider, ClipboardType};
    use anyhow::Result;
    use std::borrow::Cow;

    #[cfg(not(target_os = "windows"))]
    #[derive(Debug)]
    pub struct NopProvider {
        buf: String,
        primary_buf: String,
    }

    #[cfg(not(target_os = "windows"))]
    impl NopProvider {
        pub fn new() -> Self {
            log::warn!(
                "No clipboard provider found! Yanking and pasting will be internal to Helix"
            );
            Self {
                buf: String::new(),
                primary_buf: String::new(),
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    impl Default for NopProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(not(target_os = "windows"))]
    impl ClipboardProvider for NopProvider {
        fn name(&self) -> Cow<str> {
            Cow::Borrowed("none")
        }

        fn get_contents(&self, clipboard_type: ClipboardType) -> Result<String> {
            let value = match clipboard_type {
                ClipboardType::Clipboard => self.buf.clone(),
                ClipboardType::Selection => self.primary_buf.clone(),
            };

            Ok(value)
        }

        fn set_contents(&mut self, content: String, clipboard_type: ClipboardType) -> Result<()> {
            match clipboard_type {
                ClipboardType::Clipboard => self.buf = content,
                ClipboardType::Selection => self.primary_buf = content,
            }
            Ok(())
        }
    }

    #[cfg(target_os = "windows")]
    #[derive(Default, Debug)]
    pub struct WindowsProvider;

    #[cfg(target_os = "windows")]
    impl ClipboardProvider for WindowsProvider {
        fn name(&self) -> Cow<str> {
            log::info!("Using clipboard-win to interact with the system clipboard");
            Cow::Borrowed("clipboard-win")
        }

        fn get_contents(&self, clipboard_type: ClipboardType) -> Result<String> {
            match clipboard_type {
                ClipboardType::Clipboard => {
                    let contents = clipboard_win::get_clipboard(clipboard_win::formats::Unicode)?;
                    Ok(contents)
                }
                ClipboardType::Selection => Ok(String::new()),
            }
        }

        fn set_contents(&mut self, contents: String, clipboard_type: ClipboardType) -> Result<()> {
            match clipboard_type {
                ClipboardType::Clipboard => {
                    clipboard_win::set_clipboard(clipboard_win::formats::Unicode, contents)?;
                }
                ClipboardType::Selection => {}
            };
            Ok(())
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub mod command {
        use super::*;
        use anyhow::{bail, Context as _, Result};

        pub fn exists(executable_name: &str) -> bool {
            which::which(executable_name).is_ok()
        }

        #[cfg(not(windows))]
        pub fn env_var_is_set(env_var_name: &str) -> bool {
            std::env::var_os(env_var_name).is_some()
        }

        #[cfg(not(any(windows, target_os = "macos")))]
        pub fn is_exit_success(program: &str, args: &[&str]) -> bool {
            std::process::Command::new(program)
                .args(args)
                .output()
                .ok()
                .and_then(|out| out.status.success().then(|| ())) // TODO: use then_some when stabilized
                .is_some()
        }

        #[derive(Debug)]
        pub struct Config {
            pub prg: &'static str,
            pub args: &'static [&'static str],
        }

        impl Config {
            fn execute(&self, input: Option<&str>, pipe_output: bool) -> Result<Option<String>> {
                use std::io::Write;
                use std::process::{Command, Stdio};

                let stdin = input.map(|_| Stdio::piped()).unwrap_or_else(Stdio::null);
                let stdout = pipe_output.then(Stdio::piped).unwrap_or_else(Stdio::null);

                let mut child = Command::new(self.prg)
                    .args(self.args)
                    .stdin(stdin)
                    .stdout(stdout)
                    .stderr(Stdio::null())
                    .spawn()?;

                if let Some(input) = input {
                    let mut stdin = child.stdin.take().context("stdin is missing")?;
                    stdin
                        .write_all(input.as_bytes())
                        .context("couldn't write in stdin")?;
                }

                // TODO: add timer?
                let output = child.wait_with_output()?;

                if !output.status.success() {
                    bail!("clipboard provider {} failed", self.prg);
                }

                if pipe_output {
                    Ok(Some(String::from_utf8(output.stdout)?))
                } else {
                    Ok(None)
                }
            }
        }

        #[derive(Debug)]
        pub struct Provider {
            pub get_cmd: Config,
            pub set_cmd: Config,
            pub get_primary_cmd: Option<Config>,
            pub set_primary_cmd: Option<Config>,
        }

        impl ClipboardProvider for Provider {
            fn name(&self) -> Cow<str> {
                if self.get_cmd.prg != self.set_cmd.prg {
                    Cow::Owned(format!("{}+{}", self.get_cmd.prg, self.set_cmd.prg))
                } else {
                    Cow::Borrowed(self.get_cmd.prg)
                }
            }

            fn get_contents(&self, clipboard_type: ClipboardType) -> Result<String> {
                match clipboard_type {
                    ClipboardType::Clipboard => Ok(self
                        .get_cmd
                        .execute(None, true)?
                        .context("output is missing")?),
                    ClipboardType::Selection => {
                        if let Some(cmd) = &self.get_primary_cmd {
                            return cmd.execute(None, true)?.context("output is missing");
                        }

                        Ok(String::new())
                    }
                }
            }

            fn set_contents(&mut self, value: String, clipboard_type: ClipboardType) -> Result<()> {
                let cmd = match clipboard_type {
                    ClipboardType::Clipboard => &self.set_cmd,
                    ClipboardType::Selection => {
                        if let Some(cmd) = &self.set_primary_cmd {
                            cmd
                        } else {
                            return Ok(());
                        }
                    }
                };
                cmd.execute(Some(&value), false).map(|_| ())
            }
        }
    }
}
