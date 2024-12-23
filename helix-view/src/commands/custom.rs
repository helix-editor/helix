// If a combination takes arguments, say with a `%{arg}` syntax, it could be positional
// so that an Args could just run a simple `next` and replace as it encounters them.
//
// For example:
//   "wcd!" = [":write --force %{arg}", ":cd %sh{ %{arg} | path dirname}"]
//
// Would this take arguments like:
//     :wcd! /repo/sub/sub/file.txt
//     $ :write --force repo/sub/sub/file -> cd /repo/sub/sub/
//
// Also want to be able to support prompt usage, allowing the ability to add prompt options in the
// config:
//   "wcd!" = {
//     commands = [":write --force %{arg}", ":cd %sh{ %{arg} | path dirname}"],
//     desc= "writes buffer forcefully, then changes to its directory"
//     # path would be a hardcoded completion option.
//     # Might also be able to name other commands to get there completions?
//     # completions = "write"
//     completions = "path"
//     # allow custom list of completions
//     completions = [
//         "foo",
//         "bar",
//         "baz",
//     ]
//
// TODO: mention the handling of optionl and required arguments, and that it will just be forwarded to the command
// and any checking it has itself.
// [commands.wcd!]
// commands = [":write --force %{arg}", ":cd  %sh{ %{arg} | path dirname }"]
// desc = "writes buffer forcefully, then changes to its directory"
// completions = "write"
// accepts = "<path>"
//
// %{arg} and %{arg:0} are equivalent.
// These represent arguments passed down from the command call.
// As this will call `Args::nth(arg:NUM)`, you can just number them from 0..
// to access the corresponding arg.
//
// TODO: When adding custom aliases to the command prompt list, must priotize the custom over the built-in.
// Should include removing the alias from the aliases command?

use serde::{Deserialize, Serialize};

// TODO: Might need to manually implement Serialize and Deserialize
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct CustomTypeableCommands {
    pub commands: Vec<CustomTypableCommand>,
}

impl CustomTypeableCommands {
    #[inline]
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CustomTypableCommand> {
        self.commands.iter().find(|command| command.name == name)
    }

    #[inline]
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.commands.iter().map(|command| command.name.as_str())
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
        self.commands.iter().map(String::as_str)
    }
}

// TODO: Need to get access to a new table in the config: [commands].
// TODO: If anycommands can be supported, then can add a check for `MappableCommand::STATIC_COMMAND_LIST`.
// Might also need to make a `MappableCommand::STATIC_COMMAND_MAP`.
//
// Could also just add support directly to `MappableCommand::from_str`. This would then allow a `command.execute`
// and support macros as well.

// Checking against user provided commands first gives priority
// to user defined aliases over the built-in, allowing for overriding.
// if let Some(custom: &CustomTypeableCommand) = cx.editor.config.load().commands.get(name) {
//     for command: &str in custom.commands.iter() {
//         let shellwords = Shellwords::from(command);
//
//         if let Some(command: &TypeableCommand) = typed::TYPABLE_COMMAND_MAP.get(shellwords.command()) {
//             // TODO: Add parsing support for u%{arg:NUM}` in `expand`.
//             let args = match variables::expand(cx.editor, shellwords.args().raw(), event == PromptEvent::Validate) {
//                Ok(args) => args,
//                Err(err) => {
//                     cx.editor.set_error(format!("{err}"));
//                     // short circuit if error
//                     return;
//                },
//             }
//
//             if let Err(err) = (command.fun)(cx, Args::from(&args), command.flags, event) {
//                cx.editor.set_error(format!("{err}"));
//                     // short circuit if error
//                     return;
//             }
//         } else {
//             cx.editor.set_error(format!("command `:{}` is not a valid command", shellwords.command()));
//             // short circuit if error
//             return;
//         }
// } else if let Some(command) = typed::TYPABLE_COMMAND_MAP.get(shellwords.command()) {
//    // Current impl
// }
//
//
// TODO: Could add an `aliases` to `CustomTypableCommand` and then add those as well?
// Prompt:
//
// fuzzy_match(
//     input,
//     // `chain` the names that are yielded buy the iterator.
//     TYPABLE_COMMAND_LIST.iter().map(|command| command.name).chain(editor.config.load().commands.iter()),
//     false,
// )
// .into_iter()
// .map(|(name, _)| (0.., name.into()))
// .collect()
//
//
// Completer:
//
// let command =  = editor
//     .config
//     .load()
//     .commands
//     .get(command)
//     .map(|command | command.completer)
//     .unwrap_or(command);
//
//     TYPABLE_COMMAND_MAP
//         .get(command)
//         .map(|tc| tc.completer_for_argument_number(argument_number_of(&shellwords)))
//         .map_or_else(Vec::new, |completer| {
//             completer(editor, word)
//                 .into_iter()
//                 .map(|(range, mut file)| {
//                     file.content = shellwords::escape(file.content);
//
//                     // offset ranges to input
//                     let offset = input.len() - len;
//                     let range = (range.start + offset)..;
//                     (range, file)
//                 })
//                 .collect()
//         })
