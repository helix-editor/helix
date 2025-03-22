use std::{fmt::Write, sync::Arc};

use serde::{Deserialize, Serialize};

/// Repository of custom commands.
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
    /// Retrieves a command by its name if it exists.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CustomTypableCommand> {
        self.commands
            .iter()
            .find(|command| command.name == name.trim_start_matches(':'))
    }

    /// Returns the names of the custom commands that are not hidden.
    #[inline]
    pub fn non_hidden_names(&self) -> impl Iterator<Item = &str> {
        self.commands
            .iter()
            .filter(|command| !command.hidden)
            .map(|command| command.name.as_ref())
    }
}

/// Represents a user-custom typable command.
#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
pub struct CustomTypableCommand {
    /// The custom command that will be typed into the command line.
    ///
    /// For example `lg`
    pub name: String,
    /// The description of what the custom command does.
    pub desc: Option<String>,
    /// Single or multiple commands which will be executed via the custom command.
    pub commands: Vec<String>,
    /// Signifier if command accepts any input.
    ///
    /// This is only for documentation purposes.
    pub accepts: Option<String>,
    /// The name of the typeable of which the custom command emulate in its completions.
    pub completer: Option<String>,
    /// Whether or not the custom command is shown in the prompt list.
    pub hidden: bool,
}

impl CustomTypableCommand {
    /// Builds the prompt documentation for command.
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
