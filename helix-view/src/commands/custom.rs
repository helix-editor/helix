// TODO: When adding custom aliases to the command prompt list, must priotize the custom over the built-in.
// - Should include removing the alias from the aliases command?
//
// TODO: Need to get access to a new table in the config: [commands].
// TODO: Could add an `aliases` to `CustomTypableCommand` and then add those as well?

use std::fmt::Write;

use serde::{Deserialize, Serialize};

// TODO: Might need to manually implement Serialize and Deserialize
//     -Will need to do so if want to use `Arc` to make cloning cheaper.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CustomTypeableCommands {
    pub commands: Vec<CustomTypableCommand>,
}

impl Default for CustomTypeableCommands {
    fn default() -> Self {
        Self {
            commands: vec![
                CustomTypableCommand {
                    name: String::from(":lg"),
                    desc: Some(String::from("runs lazygit in a floating pane")),
                    commands: vec![String::from(
                        ":sh wezterm cli spawn --floating-pane lazygit",
                    )],
                    accepts: None,
                    completer: None,
                },
                CustomTypableCommand {
                    name: String::from(":w"),
                    desc: Some(String::from(
                        "writes buffer forcefully and changes directory",
                    )),
                    commands: vec![
                        String::from(":write --force %{arg}"),
                        String::from(":cd %sh{ %{arg} | path dirname }"),
                        String::from(":cd %sh{ %{arg} | path dirname }"),
                        String::from(":cd %sh{ %{arg} | path dirname }"),
                        String::from(":cd %sh{ %{arg} | path dirname }"),
                    ],
                    accepts: Some(String::from("<path>")),
                    completer: Some(String::from(":write")),
                },
            ],
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

// TODO: Arc<str> ?
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
        let mut prompt = String::new();

        prompt.push_str(self.name.trim_start_matches(':'));

        if let Some(accepts) = &self.accepts {
            write!(prompt, " {accepts}").unwrap();
        }

        prompt.push(':');

        // TODO: Might need to port the spacing algo from argument flags branch.
        if let Some(desc) = &self.desc {
            write!(prompt, " {desc}").unwrap();
        }

        prompt.push('\n');
        prompt.push('\n');

        writeln!(prompt, "maps:").unwrap();
        prompt.push_str("   ");

        for (idx, command) in self.commands.iter().enumerate() {
            write!(prompt, ":{}", command.trim_start_matches(':')).unwrap();

            if idx + 1 == self.commands.len() {
                break;
            }

            // There are two columns of commands, and after that they will overflow
            // downward:
            //
            // maps:
            //     :write --force %{arg} -> :cd %sh{ %{arg} | path dirname }
            //     -> :write --force %{arg} -> :cd %sh{ %{arg} | path dirname }
            //     -> :write --force %{arg} -> :cd %sh{ %{arg} | path dirname }
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

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.commands
            .iter()
            .map(|command| command.trim_start_matches(':'))
    }
}
