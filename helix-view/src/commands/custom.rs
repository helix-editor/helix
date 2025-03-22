use std::{fmt::Write, sync::Arc};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomTypeableCommands {
    pub commands: Arc<[CustomTypableCommand]>,
}

impl Default for CustomTypeableCommands {
    fn default() -> Self {
        Self {
            commands: Arc::new([]),
        }
    }
}

impl CustomTypeableCommands {
    #[inline]
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CustomTypableCommand> {
        self.commands
            .iter()
            .find(|command| command.name == name.trim_start_matches(':'))
    }

    #[inline]
    pub fn non_hidden_names(&self) -> impl Iterator<Item = &str> {
        self.commands
            .iter()
            .filter(|command| !command.hidden)
            .map(|command| command.name.as_ref())
    }
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
pub struct CustomTypableCommand {
    pub name: String,
    pub desc: Option<String>,
    pub commands: Vec<String>,
    pub accepts: Option<String>,
    pub completer: Option<String>,
    pub hidden: bool,
}

impl CustomTypableCommand {
    pub fn prompt(&self) -> String {
        // wcd! <path>: writes buffer forcefully, then changes to its directory
        //
        // maps:
        //     :write! %arg{0} -> :cd %sh{ %arg{0} | path dirname }
        let mut prompt = String::new();

        prompt.push_str(self.name.as_ref());

        if let Some(accepts) = &self.accepts {
            write!(prompt, " {accepts}").unwrap();
        }

        prompt.push(':');

        if let Some(desc) = &self.desc {
            write!(prompt, " {desc}").unwrap();
        }

        prompt.push('\n');
        prompt.push('\n');

        writeln!(prompt, "maps:").unwrap();
        prompt.push_str("   ");

        for (idx, command) in self.commands.iter().enumerate() {
            write!(prompt, ":{command}").unwrap();

            if idx + 1 == self.commands.len() {
                break;
            }

            // There are two columns of commands, and after that they will overflow
            // downward:
            //
            // maps:
            //     :write! %arg{0} -> :cd %sh{ %arg{0} | path dirname }
            //     -> :write! %arg{0} -> :cd %sh{ %arg{0} | path dirname }
            //     -> :write! %arg{0} -> :cd %sh{ %arg{0} | path dirname }
            //
            // Its starts with `->` to indicate that its not a new `:command`
            // but still one sequence.
            if idx % 2 == 0 {
                prompt.push('\n');
                prompt.push_str("    -> ");
            } else {
                prompt.push_str(" -> ");
            }
        }

        prompt
    }
}
