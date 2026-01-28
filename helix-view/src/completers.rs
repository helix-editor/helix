use crate::command_line::{self, Tokenizer};
use crate::document::SCRATCH_BUFFER_NAME;
use crate::text::Span;
use crate::theme;
use crate::{editor::Config, Editor};
use core::ops::RangeFrom;
use helix_core::fuzzy::fuzzy_match;
use helix_core::syntax::config::LanguageServerFeature;
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::collections::BTreeSet;

pub type Completion = (RangeFrom<usize>, Span<'static>);
pub type Completer = fn(&Editor, &str) -> Vec<Completion>;

struct Utf8PathBuf {
    path: String,
    is_dir: bool,
}

impl AsRef<str> for Utf8PathBuf {
    fn as_ref(&self) -> &str {
        &self.path
    }
}

pub fn none(_editor: &Editor, _input: &str) -> Vec<Completion> {
    Vec::new()
}

pub fn buffer(editor: &Editor, input: &str) -> Vec<Completion> {
    let names = editor.documents.values().map(|doc| {
        doc.relative_path()
            .map(|p| p.display().to_string().into())
            .unwrap_or_else(|| Cow::from(SCRATCH_BUFFER_NAME))
    });

    fuzzy_match(input, names, true)
        .into_iter()
        .map(|(name, _)| ((0..), name.into()))
        .collect()
}

pub fn theme(_editor: &Editor, input: &str) -> Vec<Completion> {
    let mut names = theme::Loader::read_names(&helix_loader::config_dir().join("themes"));
    for rt_dir in helix_loader::runtime_dirs() {
        names.extend(theme::Loader::read_names(&rt_dir.join("themes")));
    }
    names.push("default".into());
    names.push("base16_default".into());
    names.sort();
    names.dedup();

    fuzzy_match(input, names, false)
        .into_iter()
        .map(|(name, _)| ((0..), name.into()))
        .collect()
}

/// Recursive function to get all keys from this value and add them to vec
fn get_keys(value: &serde_json::Value, vec: &mut Vec<String>, scope: Option<&str>) {
    if let Some(map) = value.as_object() {
        for (key, value) in map.iter() {
            let key = match scope {
                Some(scope) => format!("{}.{}", scope, key),
                None => key.clone(),
            };
            get_keys(value, vec, Some(&key));
            if !value.is_object() {
                vec.push(key);
            }
        }
    }
}

/// Completes names of language servers which are running for the current document.
pub fn active_language_servers(editor: &Editor, input: &str) -> Vec<Completion> {
    let language_servers = doc!(editor).language_servers().map(|ls| ls.name());

    fuzzy_match(input, language_servers, false)
        .into_iter()
        .map(|(name, _)| ((0..), Span::raw(name.to_string())))
        .collect()
}

/// Completes names of language servers which are configured for the language of the current
/// document.
pub fn configured_language_servers(editor: &Editor, input: &str) -> Vec<Completion> {
    let language_servers = doc!(editor)
        .language_config()
        .into_iter()
        .flat_map(|config| &config.language_servers)
        .map(|ls| ls.name.as_str());

    fuzzy_match(input, language_servers, false)
        .into_iter()
        .map(|(name, _)| ((0..), Span::raw(name.to_string())))
        .collect()
}

pub fn setting(_editor: &Editor, input: &str) -> Vec<Completion> {
    static KEYS: Lazy<Vec<String>> = Lazy::new(|| {
        let mut keys = Vec::new();
        let json = serde_json::json!(Config::default());
        get_keys(&json, &mut keys, None);
        keys
    });

    fuzzy_match(input, &*KEYS, false)
        .into_iter()
        .map(|(name, _)| ((0..), Span::raw(name)))
        .collect()
}

pub fn filename(editor: &Editor, input: &str) -> Vec<Completion> {
    filename_with_git_ignore(editor, input, true)
}

pub fn filename_with_git_ignore(editor: &Editor, input: &str, git_ignore: bool) -> Vec<Completion> {
    filename_impl(editor, input, git_ignore, |entry| {
        let is_dir = entry.file_type().is_some_and(|entry| entry.is_dir());

        if is_dir {
            FileMatch::AcceptIncomplete
        } else {
            FileMatch::Accept
        }
    })
}

pub fn language(editor: &Editor, input: &str) -> Vec<Completion> {
    let text: String = "text".into();

    let loader = editor.syn_loader.load();
    let language_ids = loader
        .language_configs()
        .map(|config| &config.language_id)
        .chain(std::iter::once(&text));

    fuzzy_match(input, language_ids, false)
        .into_iter()
        .map(|(name, _)| ((0..), name.to_owned().into()))
        .collect()
}

pub fn lsp_workspace_command(editor: &Editor, input: &str) -> Vec<Completion> {
    let commands = doc!(editor)
        .language_servers_with_feature(LanguageServerFeature::WorkspaceCommand)
        .flat_map(|ls| {
            ls.capabilities()
                .execute_command_provider
                .iter()
                .flat_map(|options| options.commands.iter())
        });

    fuzzy_match(input, commands, false)
        .into_iter()
        .map(|(name, _)| ((0..), name.to_owned().into()))
        .collect()
}

pub fn directory(editor: &Editor, input: &str) -> Vec<Completion> {
    directory_with_git_ignore(editor, input, true)
}

pub fn directory_with_git_ignore(
    editor: &Editor,
    input: &str,
    git_ignore: bool,
) -> Vec<Completion> {
    filename_impl(editor, input, git_ignore, |entry| {
        let is_dir = entry.file_type().is_some_and(|entry| entry.is_dir());

        if is_dir {
            FileMatch::Accept
        } else {
            FileMatch::Reject
        }
    })
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum FileMatch {
    /// Entry should be ignored
    Reject,
    /// Entry is usable but can't be the end (for instance if the entry is a directory and we
    /// try to match a file)
    AcceptIncomplete,
    /// Entry is usable and can be the end of the match
    Accept,
}

// TODO: we could return an iter/lazy thing so it can fetch as many as it needs.
fn filename_impl<F>(editor: &Editor, input: &str, git_ignore: bool, filter_fn: F) -> Vec<Completion>
where
    F: Fn(&ignore::DirEntry) -> FileMatch,
{
    // Rust's filename handling is really annoying.

    use ignore::WalkBuilder;
    use std::path::Path;

    let is_tilde = input == "~";
    let path = helix_stdx::path::expand_tilde(Path::new(input));

    let (dir, file_name) = if input.ends_with(std::path::MAIN_SEPARATOR) {
        (path, None)
    } else {
        let is_period = (input.ends_with((format!("{}.", std::path::MAIN_SEPARATOR)).as_str())
            && input.len() > 2)
            || input == ".";
        let file_name = if is_period {
            Some(String::from("."))
        } else {
            path.file_name()
                .and_then(|file| file.to_str().map(|path| path.to_owned()))
        };

        let path = if is_period {
            path
        } else {
            match path.parent() {
                Some(path) if !path.as_os_str().is_empty() => Cow::Borrowed(path),
                // Path::new("h")'s parent is Some("")...
                _ => Cow::Owned(helix_stdx::env::current_working_dir()),
            }
        };

        (path, file_name)
    };

    let end = input.len()..;

    let files = WalkBuilder::new(&dir)
        .hidden(false)
        .follow_links(false) // We're scanning over depth 1
        .git_ignore(git_ignore)
        .max_depth(Some(1))
        .build()
        .filter_map(|file| {
            file.ok().and_then(|entry| {
                let fmatch = filter_fn(&entry);

                if fmatch == FileMatch::Reject {
                    return None;
                }

                let is_dir = entry.file_type().is_some_and(|entry| entry.is_dir());

                let path = entry.path();
                let mut path = if is_tilde {
                    // if it's a single tilde an absolute path is displayed so that when `TAB` is pressed on
                    // one of the directories the tilde will be replaced with a valid path not with a relative
                    // home directory name.
                    // ~ -> <TAB> -> /home/user
                    // ~/ -> <TAB> -> ~/first_entry
                    path.to_path_buf()
                } else {
                    path.strip_prefix(&dir).unwrap_or(path).to_path_buf()
                };

                if fmatch == FileMatch::AcceptIncomplete {
                    path.push("");
                }

                let path = path.into_os_string().into_string().ok()?;
                Some(Utf8PathBuf { path, is_dir })
            })
        }) // TODO: unwrap or skip
        .filter(|path| !path.path.is_empty());

    let directory_color = editor.theme.get("ui.text.directory");

    let style_from_file = |file: Utf8PathBuf| {
        if file.is_dir {
            Span::styled(file.path, directory_color)
        } else {
            Span::raw(file.path)
        }
    };

    // if empty, return a list of dirs and files in current dir
    if let Some(file_name) = file_name {
        let range = (input.len().saturating_sub(file_name.len()))..;
        fuzzy_match(&file_name, files, true)
            .into_iter()
            .map(|(name, _)| (range.clone(), style_from_file(name)))
            .collect()

        // TODO: complete to longest common match
    } else {
        let mut files: Vec<_> = files
            .map(|file| (end.clone(), style_from_file(file)))
            .collect();
        files.sort_unstable_by(|(_, path1), (_, path2)| path1.content.cmp(&path2.content));
        files
    }
}

pub fn register(editor: &Editor, input: &str) -> Vec<Completion> {
    let iter = editor
        .registers
        .iter_preview()
        // Exclude special registers that shouldn't be written to
        .filter(|(ch, _)| !matches!(ch, '%' | '#' | '.'))
        .map(|(ch, _)| ch.to_string());

    fuzzy_match(input, iter, false)
        .into_iter()
        .map(|(name, _)| ((0..), name.into()))
        .collect()
}

pub fn program(_editor: &Editor, input: &str) -> Vec<Completion> {
    static PROGRAMS_IN_PATH: Lazy<BTreeSet<String>> = Lazy::new(|| {
        // Go through the entire PATH and read all files into a set.
        let Some(path) = std::env::var_os("PATH") else {
            return Default::default();
        };

        std::env::split_paths(&path)
            .filter_map(|path| std::fs::read_dir(path).ok())
            .flatten()
            .filter_map(|res| {
                let entry = res.ok()?;
                let metadata = entry.metadata().ok()?;
                if metadata.is_file() || metadata.is_symlink() {
                    entry.file_name().into_string().ok()
                } else {
                    None
                }
            })
            .collect()
    });

    fuzzy_match(input, PROGRAMS_IN_PATH.iter(), false)
        .into_iter()
        .map(|(name, _)| ((0..), name.clone().into()))
        .collect()
}

/// This expects input to be a raw string of arguments, because this is what Signature's raw_after does.
pub fn repeating_filenames(editor: &Editor, input: &str) -> Vec<Completion> {
    let token = match Tokenizer::new(input, false).last() {
        Some(token) => token.unwrap(),
        None => return filename(editor, input),
    };

    let offset = token.content_start;

    let mut completions = filename(editor, &input[offset..]);
    for completion in completions.iter_mut() {
        completion.0.start += offset;
    }
    completions
}

pub fn shell(editor: &Editor, input: &str) -> Vec<Completion> {
    let (command, args, complete_command) = command_line::split(input);

    if complete_command {
        return program(editor, command);
    }

    let mut completions = repeating_filenames(editor, args);
    for completion in completions.iter_mut() {
        // + 1 for separator between `command` and `args`
        completion.0.start += command.len() + 1;
    }

    completions
}

#[derive(Clone)]
pub struct CommandCompleter {
    // Arguments with specific completion methods based on their position.
    positional_args: &'static [Completer],

    // All remaining arguments will use this completion method, if set.
    var_args: Completer,
}

impl CommandCompleter {
    pub const fn none() -> Self {
        Self {
            positional_args: &[],
            var_args: self::none,
        }
    }

    pub const fn positional(completers: &'static [Completer]) -> Self {
        Self {
            positional_args: completers,
            var_args: self::none,
        }
    }

    pub const fn all(completer: Completer) -> Self {
        Self {
            positional_args: &[],
            var_args: completer,
        }
    }

    pub fn for_argument_number(&self, n: usize) -> &Completer {
        match self.positional_args.get(n) {
            Some(completer) => completer,
            _ => &self.var_args,
        }
    }
}
