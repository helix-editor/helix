// TODO: check if there could be overlap for the space mode keymap and docs. Might need to make doc Option
// so that if there is overlap then it can be backwards compatible, as long as ":waq" = [":write --all", ":quit"] works.
// [commands]
// ":waq" = { commands = [":write --all", ":quit"], doc = "write all buffers to disk and quit out of helix" }

// To be able to display the prompt in a better manner for all typed commands, as well as the aliases. Might need
// a Prompt trait that would have a `prompt(&self) -> String` function This way another function could take
// `T: Prompt` and there could live a `Box<dyn Prompt>`, with a way to iterate over and display them all.

// waq: write all buffers to disk and quit out of helix
//
// aliases:
//     :write --all -> :quit

#[derive(Debug, Clone, Copy)]
pub struct Aliases {
    pub aliases: &'static [Alias],
}

impl Aliases {
    pub const fn empty() -> Self {
        Self { aliases: &[] }
    }

    pub const fn is_empty(&self) -> bool {
        self.aliases.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&Alias> {
        self.aliases.iter().find(|alias| alias.name == name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Alias> {
        self.aliases.iter()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Alias {
    pub name: &'static str,
    pub command: Option<&'static str>,
}

impl Alias {
    pub fn flags(&self) -> Option<&str> {
        if let Some(command) = self.command {
            let (_, flags) = command.split_once(' ')?;
            return Some(flags);
        }
        None
    }
}

// # Example usage:
//     aliases!["wa!" => "write --all --force", "w", "wa" => "write --all"]
#[macro_export]
macro_rules! aliases {
    () => {
        $crate::commands::alias::Aliases::empty()
    };
    ($($alias:expr $(=> $command:expr)?),* $(,)?) => {{
        const ALIASES: &[$crate::commands::alias::Alias] = &[
            $(
                $crate::commands::alias::Alias {
                    name: $alias,
                    command: {
                        #[allow(unused_mut, unused_assignments)]
                        let mut command = None;
                        $(command = Some($command);)?
                        command
                    },
                }
            ),*
        ];
        $crate::commands::alias::Aliases { aliases: ALIASES }
    }};
}
