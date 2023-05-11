use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

pub trait Executor {
    fn execute(&self, command: Command) -> anyhow::Result<()>;
}

pub struct Direct;

impl Executor for Direct {
    fn execute(&self, mut command: Command) -> anyhow::Result<()> {
        let output = command.output()?;
        let output_text = std::str::from_utf8(&output.stdout)?;
        println!("{}", output_text);
        Ok(())
    }
}

pub struct Tmux {
    pane: String,
}

impl Tmux {
    pub fn new() -> Self {
        Tmux{
            pane: String::from("!"), // last pane
        }
    }
}

impl Executor for Tmux {
    fn execute(&self, command: Command) -> anyhow::Result<()> {
        // Need to make this smarter
        let mut command_vec = vec![command.get_program()];
        for arg in command.get_args() {
            command_vec.push(arg);
        }
        //command_vec.extend(command.get_args().collect());
        let command_to_send = command_vec.join(OsStr::new(" "));
        let command_to_send = shellify(command.get_current_dir(), &command_to_send);

        std::process::Command::new("tmux")
            .args([OsStr::new("send-keys"), OsStr::new("-t"), OsStr::new(&self.pane), &command_to_send, OsStr::new("Enter")])
            .spawn()?
            .wait()?;

        Ok(())
    }
}

pub fn get() -> Box<dyn Executor> {
    if let Ok(_) = std::env::var("TMUX") {
        Box::new(Tmux::new())
    } else {
        Box::new(Direct{})
    }
}

fn shellify(dir: Option<&Path>, cmd: &OsStr) -> OsString {
    if let Ok(shell) = std::env::var("SHELL") {
        match shell.as_str() {
            "/bin/sh" | "/bin/bash" | "/bin/zsh" => {
                let mut result = OsString::from("clear; ");
                match dir {
                    Some(dir) => {
                        // ???: Better way to format an OsString?
                        result.push("(cd ");
                        result.push(dir.as_os_str());
                        result.push(" && ");
                        result.push(cmd);
                        result.push(")");
                    },
                    None => result.push(cmd),
                }
                return result;
            },
            _ => (),
        }
    }
    OsString::from(cmd)
}
