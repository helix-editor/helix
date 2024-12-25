// TODO: When adding custom aliases to the command prompt list, must priotize the custom over the built-in.
// - Should include removing the alias from the aliases command?
//
// TODO: Need to get access to a new table in the config: [commands].
// TODO: Could add an `aliases` to `CustomTypableCommand` and then add those as well?

use serde::{Deserialize, Serialize};

// TODO: Might need to manually implement Serialize and Deserialize
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CustomTypeableCommands {
    pub commands: Vec<CustomTypableCommand>,
}

impl Default for CustomTypeableCommands {
    fn default() -> Self {
        Self {
            commands: vec![CustomTypableCommand {
                name: String::from(":lg"),
                desc: Some(String::from("runs lazygit in a floating pane")),
                commands: vec![String::from(
                    ":sh wezterm cli spawn --floating-pane lazygit",
                )],
                accepts: None,
                completer: None,
            }],
        }
    }
}

impl CustomTypeableCommands {
    #[inline]
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CustomTypableCommand> {
        self.commands
            .iter()
            .find(|command| command.name.trim_start_matches(':') == name.trim_start_matches(':'))
    }

    #[inline]
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.commands
            .iter()
            // ":wbc!" -> "wbc!"
            .map(|command| command.name.as_str().trim_start_matches(':'))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CustomTypableCommand {
    pub name: String,
    pub desc: Option<String>,
    pub commands: Vec<String>,
    pub accepts: Option<String>,
    pub completer: Option<String>,
}

impl CustomTypableCommand {
    pub fn prompt(&self) -> String {
        // wcd! <path>: writes buffer forcefully, then changes to its directory
        //
        // maps:
        //     :write --force %{arg} -> :cd %sh{ %{arg} | path dirname }
        todo!()
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.commands
            .iter()
            .map(|command| command.trim_start_matches(':'))
    }
}
