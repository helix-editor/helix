// Implementation reference: https://github.com/neovim/neovim/blob/f2906a4669a2eef6d7bf86a29648793d63c98949/runtime/autoload/provider/clipboard.vim#L68-L152

use anyhow::Result;
use helix_stdx::nonempty::NonEmptyVec;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Clone, Copy, Debug)]
pub enum ClipboardType {
    Clipboard,
    Selection,
}

pub trait ClipboardProvider: std::fmt::Debug {
    fn name(&self) -> Cow<str>;
    fn get_contents(&self, clipboard_type: ClipboardType) -> Result<String>;
    fn set_contents(&mut self, contents: String, clipboard_type: ClipboardType) -> Result<()>;
}

impl dyn ClipboardProvider {
    pub fn from_string(string: &str) -> Result<Box<dyn ClipboardProvider>> {
        let config: ClipboardConfig = serde_json::from_str(string)?;
        Ok(config.get_provider())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClipboardConfig {
    #[cfg(windows)]
    Windows,
    #[cfg(target_os = "macos")]
    #[serde(rename = "macos")]
    MacOS,
    #[cfg(not(any(windows, target_arch = "wasm32", target_os = "macos")))]
    Wayland,
    #[cfg(not(any(windows, target_arch = "wasm32", target_os = "macos")))]
    #[serde(rename = "xclip")]
    XClip,
    #[cfg(not(any(windows, target_arch = "wasm32", target_os = "macos")))]
    #[serde(rename = "xsel")]
    XSel,
    #[cfg(not(any(windows, target_arch = "wasm32", target_os = "macos")))]
    Win32Yank,
    #[cfg(not(windows))]
    Termux,
    #[cfg(not(windows))]
    Tmux,
    #[cfg(feature = "term")]
    #[cfg(not(windows))]
    Term,
    #[cfg(not(windows))]
    Custom(CustomClipboardConfig),
    None,
}

impl Default for ClipboardConfig {
    #[cfg(windows)]
    fn default() -> Self {
        Self::Windows
    }

    #[cfg(target_os = "macos")]
    fn default() -> Self {
        use helix_stdx::env::{binary_exists, env_var_is_set};

        if env_var_is_set("TMUX") && binary_exists("tmux") {
            Self::Tmux
        } else if binary_exists("pbcopy") && binary_exists("pbpaste") {
            Self::MacOS
        } else {
            Self::Term
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn default() -> Box<dyn ClipboardProvider> {
        Self::None
    }

    #[cfg(not(any(windows, target_arch = "wasm32", target_os = "macos")))]
    fn default() -> Self {
        use command_provider::command::is_exit_success;
        use helix_stdx::env::{binary_exists, env_var_is_set};

        if env_var_is_set("WAYLAND_DISPLAY")
            && binary_exists("wl-copy")
            && binary_exists("wl-paste")
        {
            Self::Wayland
        } else if env_var_is_set("DISPLAY") && binary_exists("xclip") {
            Self::XClip
        } else if env_var_is_set("DISPLAY")
            && binary_exists("xsel")
            && is_exit_success("xsel", &["-o", "-b"])
        {
            // FIXME: check performance of is_exit_success
            Self::XSel
        } else if binary_exists("win32yank.exe") {
            Self::Win32Yank
        } else if binary_exists("termux-clipboard-set") && binary_exists("termux-clipboard-get") {
            Self::Termux
        } else if env_var_is_set("TMUX") && binary_exists("tmux") {
            Self::Tmux
        } else {
            Self::Term
        }
    }
}

#[cfg(not(windows))]
macro_rules! command_provider {
    (name => $name:literal ; paste => $get_prg:literal $( , $get_arg:literal )* ; copy => $set_prg:literal $( , $set_arg:literal )* ; ) => {{
        log::debug!(
            "Using {} to interact with the system clipboard",
            if $set_prg != $get_prg { format!("{}+{}", $set_prg, $get_prg)} else { $set_prg.to_string() }
        );
        Box::new(command_provider::command::Provider {
            name: $name.to_owned(),
            get_cmd: command_provider::command::CommandConfig {
                prg: $get_prg.to_string(),
                args: vec![ $( $get_arg.to_string() ),* ],
            },
            set_cmd: command_provider::command::CommandConfig {
                prg: $set_prg.to_string(),
                args: vec![ $( $set_arg.to_string() ),* ],
            },
            get_primary_cmd: None,
            set_primary_cmd: None,
        })
    }};

    (name => $name:literal ;
     paste => $get_prg:literal $( , $get_arg:literal )* ;
     copy => $set_prg:literal $( , $set_arg:literal )* ;
     primary_paste => $pr_get_prg:literal $( , $pr_get_arg:literal )* ;
     primary_copy => $pr_set_prg:literal $( , $pr_set_arg:literal )* ;
    ) => {{
        log::debug!(
            "Using {} to interact with the system and selection (primary) clipboard",
            if $set_prg != $get_prg { format!("{}+{}", $set_prg, $get_prg)} else { $set_prg.to_string() }
        );
        Box::new(command_provider::command::Provider {
            name: $name.to_owned(),
            get_cmd: command_provider::command::CommandConfig {
                prg: $get_prg.to_string(),
                args: vec![ $( $get_arg.to_string() ),* ],
            },
            set_cmd: command_provider::command::CommandConfig {
                prg: $set_prg.to_string(),
                args: vec![ $( $set_arg.to_string() ),* ],
            },
            get_primary_cmd: Some(command_provider::command::CommandConfig {
                prg: $pr_get_prg.to_string(),
                args: vec![ $( $pr_get_arg.to_string() ),* ],
            }),
            set_primary_cmd: Some(command_provider::command::CommandConfig {
                prg: $pr_set_prg.to_string(),
                args: vec![ $( $pr_set_arg.to_string() ),* ],
            }),
        })
    }};
}

impl ClipboardConfig {
    pub fn get_provider(&self) -> Box<dyn ClipboardProvider> {
        match self {
            #[cfg(target_os = "windows")]
            ClipboardConfig::Windows => Box::new(win_provider::WindowsProvider),
            #[cfg(target_os = "macos")]
            ClipboardConfig::MacOS => command_provider! {
                name => "MacOS";
                paste => "pbpaste";
                copy => "pbcopy";
            },
            #[cfg(not(any(windows, target_arch = "wasm32", target_os = "macos")))]
            ClipboardConfig::Wayland => command_provider! {
                name => "Wayland";
                paste => "wl-paste", "--no-newline";
                copy => "wl-copy", "--type", "text/plain";
                primary_paste => "wl-paste", "-p", "--no-newline";
                primary_copy => "wl-copy", "-p", "--type", "text/plain";
            },
            #[cfg(not(any(windows, target_arch = "wasm32", target_os = "macos")))]
            ClipboardConfig::XClip => command_provider! {
                name => "XClip";
                paste => "xclip", "-o", "-selection", "clipboard";
                copy => "xclip", "-i", "-selection", "clipboard";
                primary_paste => "xclip", "-o";
                primary_copy => "xclip", "-i";
            },
            #[cfg(not(any(windows, target_arch = "wasm32", target_os = "macos")))]
            ClipboardConfig::XSel => command_provider! {
                name => "XSel";
                paste => "xclip", "-o", "-selection", "clipboard";
                copy => "xclip", "-i", "-selection", "clipboard";
                primary_paste => "xclip", "-o";
                primary_copy => "xclip", "-i";
            },
            #[cfg(not(windows))]
            ClipboardConfig::Termux => command_provider! {
                name => "Termux";
                paste => "termux-clipboard-get";
                copy => "termux-clipboard-set";
            },
            #[cfg(not(any(windows, target_arch = "wasm32", target_os = "macos")))]
            ClipboardConfig::Win32Yank => command_provider! {
                name => "Win32Yank";
                paste => "win32yank.exe", "-o", "--lf";
                copy => "win32yank.exe", "-i", "--crlf";
            },
            #[cfg(not(windows))]
            ClipboardConfig::Tmux => command_provider! {
                name => "Tmux";
                paste => "tmux", "save-buffer", "-";
                copy => "tmux", "save-buffer", "-";
            },
            #[cfg(feature = "term")]
            #[cfg(not(windows))]
            ClipboardConfig::Term => Box::new(term_provider::TermProvider::new()),
            #[cfg(not(windows))]
            ClipboardConfig::Custom(cust) => Box::new(command_provider::command::Provider {
                name: "Custom configuration".to_string(),
                get_cmd: command_provider::command::CommandConfig {
                    prg: cust.paste.head().clone(),
                    args: cust.paste.tail().clone(),
                },
                set_cmd: command_provider::command::CommandConfig {
                    prg: cust.copy.head().clone(),
                    args: cust.copy.tail().clone(),
                },
                get_primary_cmd: Some(command_provider::command::CommandConfig {
                    prg: cust.copy.head().clone(),
                    args: cust.copy.tail().clone(),
                }),
                set_primary_cmd: Some(command_provider::command::CommandConfig {
                    prg: cust.copy.head().clone(),
                    args: cust.copy.tail().clone(),
                }),
            }),
            ClipboardConfig::None => Box::<NoneProvider>::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct CustomClipboardConfig {
    copy: NonEmptyVec<String>,
    paste: NonEmptyVec<String>,
    primary_copy: Option<NonEmptyVec<String>>,
    primary_paste: Option<NonEmptyVec<String>>,
}

#[derive(Debug, Default)]
pub struct NoneProvider {
    buf: String,
    primary_buf: String,
}

impl ClipboardProvider for NoneProvider {
    fn name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed("None (internal to helix)")
    }

    fn get_contents(&self, clipboard_type: ClipboardType) -> Result<String> {
        // This is the same noop if term is enabled or not.
        // We don't use the get side of OSC 52 as it isn't often enabled, it's a security hole,
        // and it would require this to be async to listen for the response
        let value = match clipboard_type {
            ClipboardType::Clipboard => self.buf.clone(),
            ClipboardType::Selection => self.primary_buf.clone(),
        };

        Ok(value)
    }

    fn set_contents(&mut self, content: String, clipboard_type: ClipboardType) -> Result<()> {
        // Set our internal variables to use in get_content regardless of using OSC 52
        match clipboard_type {
            ClipboardType::Clipboard => self.buf = content,
            ClipboardType::Selection => self.primary_buf = content,
        }
        Ok(())
    }
}

#[cfg(feature = "term")]
#[cfg(not(windows))]
pub mod term_provider {
    use super::{ClipboardProvider, ClipboardType};
    use crate::base64;
    use anyhow::Result;
    use std::borrow::Cow;

    #[derive(Debug)]
    pub struct SetClipboardCommand {
        encoded_content: String,
        clipboard_type: ClipboardType,
    }

    impl SetClipboardCommand {
        pub fn new(content: &str, clipboard_type: ClipboardType) -> Self {
            Self {
                encoded_content: base64::encode(content.as_bytes()),
                clipboard_type,
            }
        }
    }

    impl crossterm::Command for SetClipboardCommand {
        fn write_ansi(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
            let kind = match &self.clipboard_type {
                ClipboardType::Clipboard => "c",
                ClipboardType::Selection => "p",
            };
            // Send an OSC 52 set command: https://terminalguide.namepad.de/seq/osc-52/
            write!(f, "\x1b]52;{};{}\x1b\\", kind, &self.encoded_content)
        }
    }

    #[derive(Debug)]
    pub struct TermProvider {
        buf: String,
        primary_buf: String,
    }

    impl TermProvider {
        pub fn new() -> Self {
            #[cfg(feature = "term")]
            log::debug!("Yanking by OSC 52 and pasting will be internal to Helix");
            Self {
                buf: String::new(),
                primary_buf: String::new(),
            }
        }
    }

    impl Default for TermProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ClipboardProvider for TermProvider {
        fn name(&self) -> Cow<str> {
            Cow::Borrowed("Term (OSC copy code, paste internal to Helix)")
        }

        fn get_contents(&self, clipboard_type: ClipboardType) -> Result<String> {
            // This is the same noop if term is enabled or not.
            // We don't use the get side of OSC 52 as it isn't often enabled, it's a security hole,
            // and it would require this to be async to listen for the response
            let value = match clipboard_type {
                ClipboardType::Clipboard => self.buf.clone(),
                ClipboardType::Selection => self.primary_buf.clone(),
            };

            Ok(value)
        }

        fn set_contents(&mut self, content: String, clipboard_type: ClipboardType) -> Result<()> {
            #[cfg(feature = "term")]
            crossterm::execute!(
                std::io::stdout(),
                SetClipboardCommand::new(&content, clipboard_type)
            )?;
            // Set our internal variables to use in get_content regardless of using OSC 52
            match clipboard_type {
                ClipboardType::Clipboard => self.buf = content,
                ClipboardType::Selection => self.primary_buf = content,
            }
            Ok(())
        }
    }
}

#[cfg(not(any(windows, target_arch = "wasm32")))]
pub mod command_provider {
    pub mod command {
        use crate::clipboard::{ClipboardProvider, ClipboardType};
        use anyhow::{bail, Context as _, Result};
        use std::borrow::Cow;

        #[cfg(not(any(windows, target_os = "macos")))]
        pub fn is_exit_success(program: &str, args: &[&str]) -> bool {
            std::process::Command::new(program)
                .args(args)
                .output()
                .ok()
                .and_then(|out| out.status.success().then_some(()))
                .is_some()
        }

        #[derive(Debug)]
        pub struct CommandConfig {
            pub prg: String,
            pub args: Vec<String>,
        }

        impl CommandConfig {
            fn execute(&self, input: Option<&str>, pipe_output: bool) -> Result<Option<String>> {
                use std::io::Write;
                use std::process::{Command, Stdio};

                let stdin = input.map(|_| Stdio::piped()).unwrap_or_else(Stdio::null);
                let stdout = pipe_output.then(Stdio::piped).unwrap_or_else(Stdio::null);

                let mut command: Command = Command::new(&self.prg);

                let mut command_mut: &mut Command = command
                    .args(&self.args)
                    .stdin(stdin)
                    .stdout(stdout)
                    .stderr(Stdio::null());

                // Fix for https://github.com/helix-editor/helix/issues/5424
                if cfg!(unix) {
                    use std::os::unix::process::CommandExt;

                    unsafe {
                        command_mut = command_mut.pre_exec(|| match libc::setsid() {
                            -1 => Err(std::io::Error::last_os_error()),
                            _ => Ok(()),
                        });
                    }
                }

                let mut child = command_mut.spawn()?;

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
            pub name: String,
            pub get_cmd: CommandConfig,
            pub set_cmd: CommandConfig,
            pub get_primary_cmd: Option<CommandConfig>,
            pub set_primary_cmd: Option<CommandConfig>,
        }

        impl ClipboardProvider for Provider {
            fn name(&self) -> Cow<str> {
                if self.get_cmd.prg != self.set_cmd.prg {
                    Cow::Owned(format!(
                        "{} ({}+{})",
                        self.name, self.get_cmd.prg, self.set_cmd.prg
                    ))
                } else {
                    Cow::Owned(format!("{} ({})", self.name, self.get_cmd.prg))
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

#[cfg(target_os = "windows")]
mod win_provider {
    use super::{ClipboardProvider, ClipboardType};
    use anyhow::Result;
    use std::borrow::Cow;

    #[derive(Default, Debug)]
    pub struct WindowsProvider;

    impl ClipboardProvider for WindowsProvider {
        fn name(&self) -> Cow<str> {
            log::debug!("Using clipboard-win to interact with the system clipboard");
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
}
