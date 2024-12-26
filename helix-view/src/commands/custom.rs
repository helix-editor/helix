use std::{fmt::Write, sync::Arc};

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
            .find(|command| command.name.as_ref() == name)
    }

    #[inline]
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.commands.iter().map(|command| command.name.as_ref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomTypableCommand {
    pub name: Arc<str>,
    pub desc: Option<Arc<str>>,
    pub commands: Arc<[Arc<str>]>,
    pub accepts: Option<Arc<str>>,
    pub completer: Option<Arc<str>>,
}

impl CustomTypableCommand {
    pub fn prompt(&self) -> String {
        // wcd! <path>: writes buffer forcefully, then changes to its directory
        //
        // maps:
        //     :write --force %{arg} -> :cd %sh{ %{arg} | path dirname }
        let mut prompt = String::new();

        prompt.push_str(self.name.as_ref());

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
            write!(prompt, ":{command}").unwrap();

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
        self.commands.iter().map(|command| command.as_ref())
    }
}
