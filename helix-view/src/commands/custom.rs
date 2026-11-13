use std::{fmt::Write, sync::Arc};

use serde::{de::Visitor, Deserialize, Deserializer, Serialize};

/// Repository of custom commands.
///
/// This type wraps an `Arc` and is cheap to clone.
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
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
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
    /// Prefix for ignoring a custom typable command and referencing the editor typed command instead.
    pub const ESCAPE: char = '^';

    /// Builds the prompt documentation for command.
    #[inline]
    #[must_use]
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

impl<'de> Deserialize<'de> for CustomTypableCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(CustomTypableCommandVisitor)
    }
}

struct CustomTypableCommandVisitor;

impl<'de> Visitor<'de> for CustomTypableCommandVisitor {
    type Value = CustomTypableCommand;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a command, list of commands, or a detailed object")
    }

    fn visit_str<E>(self, command: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(CustomTypableCommand {
            name: String::new(), // Placeholder, will be assigned later
            desc: None,
            commands: vec![command.trim_start_matches(':').to_string()],
            accepts: None,
            completer: None,
            hidden: false,
        })
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: serde::de::SeqAccess<'de>,
    {
        let mut commands = Vec::with_capacity(4);
        while let Some(command) = seq.next_element::<String>()? {
            commands.push(command.trim_start_matches(':').to_string());
        }

        // Prevent macro keybindings from being used in command sequences.
        // This is meant to be a temporary restriction pending a larger
        // refactor of how command sequences are executed.
        let macros = commands
            .iter()
            .filter(|command| command.starts_with('@'))
            .count();

        if macros > 1 || (macros == 1 && commands.len() > 1) {
            return Err(serde::de::Error::custom(
                "macro keybindings may not be used in command sequences",
            ));
        }

        Ok(CustomTypableCommand {
            name: String::new(), // Placeholder, will be assigned later
            desc: None,
            commands,
            accepts: None,
            completer: None,
            hidden: false,
        })
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut commands = Vec::new();
        let mut desc = None;
        let mut accepts = None;
        let mut completer: Option<String> = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "commands" => {
                    commands = map
                        .next_value::<Vec<String>>()?
                        .into_iter()
                        .map(|cmd| cmd.trim_start_matches(':').to_string())
                        .collect();
                }
                "desc" => desc = map.next_value()?,
                "accepts" => accepts = map.next_value()?,
                "completer" => completer = map.next_value()?,
                _ => {
                    return Err(serde::de::Error::unknown_field(
                        &key,
                        &["commands", "desc", "accepts", "completer"],
                    ))
                }
            }
        }

        // Prevent macro keybindings from being used in command sequences.
        // This is meant to be a temporary restriction pending a larger
        // refactor of how command sequences are executed.
        let macros = commands
            .iter()
            .filter(|command| command.starts_with('@'))
            .count();

        if macros > 1 || (macros == 1 && commands.len() > 1) {
            return Err(serde::de::Error::custom(
                "macro keybindings may not be used in command sequences",
            ));
        }

        Ok(CustomTypableCommand {
            name: String::new(), // Placeholder, will be assigned later
            desc,
            commands,
            accepts,
            completer: completer.map(|c| c.trim_start_matches(':').to_string()),
            hidden: false,
        })
    }
}
