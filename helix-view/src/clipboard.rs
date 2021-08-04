// Implementation reference: https://github.com/neovim/neovim/blob/f2906a4669a2eef6d7bf86a29648793d63c98949/runtime/autoload/provider/clipboard.vim#L68-L152

use anyhow::Result;
use std::borrow::Cow;

pub trait ClipboardProvider: std::fmt::Debug {
    fn name(&self) -> Cow<str>;
    fn get_contents(&self) -> Result<String>;
    fn set_contents(&mut self, contents: String) -> Result<()>;
}

macro_rules! command_provider {
    (paste => $get_prg:literal $( , $get_arg:literal )* ; copy => $set_prg:literal $( , $set_arg:literal )* ; ) => {{
        Box::new(provider::CommandProvider {
            get_cmd: provider::CommandConfig {
                prg: $get_prg,
                args: &[ $( $get_arg ),* ],
            },
            set_cmd: provider::CommandConfig {
                prg: $set_prg,
                args: &[ $( $set_arg ),* ],
            },
        })
    }};
}

pub fn get_clipboard_provider() -> Box<dyn ClipboardProvider> {
    // TODO: support for user-defined provider, probably when we have plugin support by setting a
    // variable?

    if exists("pbcopy") && exists("pbpaste") {
        command_provider! {
            paste => "pbpaste";
            copy => "pbcopy";
        }
    } else if env_var_is_set("WAYLAND_DISPLAY") && exists("wl-copy") && exists("wl-paste") {
        command_provider! {
            paste => "wl-paste", "--no-newline";
            copy => "wl-copy", "--type", "text/plain";
        }
    } else if env_var_is_set("DISPLAY") && exists("xclip") {
        command_provider! {
            paste => "xclip", "-o", "-selection", "clipboard";
            copy => "xclip", "-i", "-selection", "clipboard";
        }
    } else if env_var_is_set("DISPLAY") && exists("xsel") && is_exit_success("xsel", &["-o", "-b"])
    {
        // FIXME: check performance of is_exit_success
        command_provider! {
            paste => "xsel", "-o", "-b";
            copy => "xsel", "--nodetach", "-i", "-b";
        }
    } else if exists("lemonade") {
        command_provider! {
            paste => "lemonade", "paste";
            copy => "lemonade", "copy";
        }
    } else if exists("doitclient") {
        command_provider! {
            paste => "doitclient", "wclip", "-r";
            copy => "doitclient", "wclip";
        }
    } else if exists("win32yank.exe") {
        // FIXME: does it work within WSL?
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
        #[cfg(target_os = "windows")]
        return Box::new(provider::WindowsProvider);

        #[cfg(not(target_os = "windows"))]
        return Box::new(provider::NopProvider{ buf: String::new() });
    }
}

fn exists(executable_name: &str) -> bool {
    which::which(executable_name).is_ok()
}

fn env_var_is_set(env_var_name: &str) -> bool {
    std::env::var_os(env_var_name).is_some()
}

fn is_exit_success(program: &str, args: &[&str]) -> bool {
    std::process::Command::new(program)
        .args(args)
        .output()
        .ok()
        .and_then(|out| out.status.success().then(|| ())) // TODO: use then_some when stabilized
        .is_some()
}

mod provider {
    use super::ClipboardProvider;
    use anyhow::{bail, Context as _, Result};
    use std::borrow::Cow;

    #[derive(Debug)]
    pub struct NopProvider {
        pub buf: String,
    }

    impl ClipboardProvider for NopProvider {
        fn name(&self) -> Cow<str> {
            Cow::Borrowed("none")
        }

        fn get_contents(&self) -> Result<String> {
            Ok(self.buf.clone())
        }

        fn set_contents(&mut self, content: String) -> Result<()> {
            self.buf = content;
            Ok(())
        }
    }

    #[cfg(target_os = "windows")]
    #[derive(Debug)]
    pub struct WindowsProvider;

    #[cfg(target_os = "windows")]
    impl ClipboardProvider for WindowsProvider {
        fn name(&self) -> Cow<str> {
            Cow::Borrowed("clipboard-win")
        }

        fn get_contents(&self) -> Result<String> {
            let contents = clipboard_win::get_clipboard(clipboard_win::formats::Unicode)?;
            Ok(contents)
        }

        fn set_contents(&mut self, contents: String) -> Result<()> {
            clipboard_win::set_clipboard(clipboard_win::formats::Unicode, contents)?;
            Ok(())
        }
    }

    #[derive(Debug)]
    pub struct CommandConfig {
        pub prg: &'static str,
        pub args: &'static [&'static str],
    }

    impl CommandConfig {
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
    pub struct CommandProvider {
        pub get_cmd: CommandConfig,
        pub set_cmd: CommandConfig,
    }

    impl ClipboardProvider for CommandProvider {
        fn name(&self) -> Cow<str> {
            if self.get_cmd.prg != self.set_cmd.prg {
                Cow::Owned(format!("{}+{}", self.get_cmd.prg, self.set_cmd.prg))
            } else {
                Cow::Borrowed(self.get_cmd.prg)
            }
        }

        fn get_contents(&self) -> Result<String> {
            let output = self
                .get_cmd
                .execute(None, true)?
                .context("output is missing")?;
            Ok(output)
        }

        fn set_contents(&mut self, value: String) -> Result<()> {
            self.set_cmd.execute(Some(&value), false).map(|_| ())
        }
    }
}
