use super::{ClipboardProvider, ClipboardType};
use anyhow::{bail, Context as _, Result};
use std::fmt;

#[derive(Debug)]
pub struct NopProvider {
    buf: String,
    primary_buf: String,
}

impl NopProvider {
    pub fn new() -> Self {
        Self {
            buf: String::new(),
            primary_buf: String::new(),
        }
    }
}

impl fmt::Display for NopProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("none")
    }
}

impl ClipboardProvider for NopProvider {
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
impl fmt::Display for WindowsProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("clipboard-win")
    }
}

#[cfg(target_os = "windows")]
impl ClipboardProvider for WindowsProvider {
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

        let mut child = Command::new(&self.prg)
            .args(&self.args)
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
    pub get_primary_cmd: Option<CommandConfig>,
    pub set_primary_cmd: Option<CommandConfig>,
}

impl fmt::Display for CommandProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (get, set) = (&self.get_cmd.prg, &self.set_cmd.prg);
        if get == set {
            f.write_str(get)
        } else {
            write!(f, "{}+{}", get, set)
        }
    }
}

impl ClipboardProvider for CommandProvider {
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
