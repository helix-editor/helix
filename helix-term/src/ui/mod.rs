mod completion;
mod editor;
mod markdown;
mod menu;
mod picker;
mod popup;
mod prompt;
mod spinner;
mod text;

pub use completion::Completion;
pub use editor::EditorView;
pub use markdown::Markdown;
pub use menu::Menu;
pub use picker::Picker;
pub use popup::Popup;
pub use prompt::{Prompt, PromptEvent};
pub use spinner::{ProgressSpinners, Spinner};
pub use text::Text;

pub use tui::layout::Rect;
pub use tui::style::{Color, Modifier, Style};

use helix_core::regex::Regex;
use helix_core::register::Registers;
use helix_view::{Document, Editor, View};

use std::path::{Path, PathBuf};

pub fn regex_prompt(
    cx: &mut crate::commands::Context,
    prompt: String,
    fun: impl Fn(&mut View, &mut Document, &mut Registers, Regex) + 'static,
) -> Prompt {
    let (view, doc) = current!(cx.editor);
    let view_id = view.id;
    let snapshot = doc.selection(view_id).clone();

    Prompt::new(
        prompt,
        |input: &str| Vec::new(), // this is fine because Vec::new() doesn't allocate
        move |editor: &mut Editor, input: &str, event: PromptEvent| {
            match event {
                PromptEvent::Abort => {
                    // TODO: also revert text
                    let (view, doc) = current!(editor);
                    doc.set_selection(view.id, snapshot.clone());
                }
                PromptEvent::Validate => {
                    // TODO: push_jump to store selection just before jump
                }
                PromptEvent::Update => {
                    // skip empty input, TODO: trigger default
                    if input.is_empty() {
                        return;
                    }

                    match Regex::new(input) {
                        Ok(regex) => {
                            let (view, doc) = current!(editor);
                            let registers = &mut editor.registers;

                            // revert state to what it was before the last update
                            // TODO: also revert text
                            doc.set_selection(view.id, snapshot.clone());

                            fun(view, doc, registers, regex);

                            view.ensure_cursor_in_view(doc);
                        }
                        Err(_err) => (), // TODO: mark command line as error
                    }
                }
            }
        },
    )
}

pub fn file_picker(root: PathBuf) -> Picker<PathBuf> {
    use ignore::Walk;
    use std::time;
    let files = Walk::new(root.clone()).filter_map(|entry| match entry {
        Ok(entry) => {
            // filter dirs, but we might need special handling for symlinks!
            if !entry.file_type().map_or(false, |entry| entry.is_dir()) {
                let time = if let Ok(metadata) = entry.metadata() {
                    metadata
                        .accessed()
                        .or_else(|_| metadata.modified())
                        .or_else(|_| metadata.created())
                        .unwrap_or(time::UNIX_EPOCH)
                } else {
                    time::UNIX_EPOCH
                };

                Some((entry.into_path(), time))
            } else {
                None
            }
        }
        Err(_err) => None,
    });

    let mut files: Vec<_> = if root.join(".git").is_dir() {
        files.collect()
    } else {
        const MAX: usize = 8192;
        files.take(MAX).collect()
    };

    files.sort_by_key(|file| file.1);

    let files = files.into_iter().map(|(path, _)| path).collect();

    Picker::new(
        files,
        move |path: &PathBuf| {
            // format_fn
            path.strip_prefix(&root)
                .unwrap_or(path)
                .to_str()
                .unwrap()
                .into()
        },
        move |editor: &mut Editor, path: &PathBuf, action| {
            let document_id = editor
                .open(path.into(), action)
                .expect("editor.open failed");
        },
    )
}

pub mod completers {
    use crate::ui::prompt::Completion;
    use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
    use fuzzy_matcher::FuzzyMatcher;
    use helix_view::theme;
    use std::cmp::Reverse;
    use std::{borrow::Cow, sync::Arc};

    pub type Completer = fn(&str) -> Vec<Completion>;

    pub fn theme(input: &str) -> Vec<Completion> {
        let mut names = theme::Loader::read_names(&helix_core::runtime_dir().join("themes"));
        names.extend(theme::Loader::read_names(
            &helix_core::config_dir().join("themes"),
        ));
        names.push("default".into());

        let mut names: Vec<_> = names
            .into_iter()
            .map(|name| ((0..), Cow::from(name)))
            .collect();

        let matcher = Matcher::default();

        let mut matches: Vec<_> = names
            .into_iter()
            .filter_map(|(range, name)| {
                matcher
                    .fuzzy_match(&name, &input)
                    .map(|score| (name, score))
            })
            .collect();

        matches.sort_unstable_by_key(|(_file, score)| Reverse(*score));
        names = matches.into_iter().map(|(name, _)| ((0..), name)).collect();

        names
    }

    // TODO: we could return an iter/lazy thing so it can fetch as many as it needs.
    pub fn filename(input: &str) -> Vec<Completion> {
        // Rust's filename handling is really annoying.

        use ignore::WalkBuilder;
        use std::path::{Path, PathBuf};

        let is_tilde = input.starts_with('~') && input.len() == 1;
        let path = helix_view::document::expand_tilde(Path::new(input));

        let (dir, file_name) = if input.ends_with('/') {
            (path, None)
        } else {
            let file_name = path
                .file_name()
                .map(|file| file.to_str().unwrap().to_owned());

            let path = match path.parent() {
                Some(path) if !path.as_os_str().is_empty() => path.to_path_buf(),
                // Path::new("h")'s parent is Some("")...
                _ => std::env::current_dir().expect("couldn't determine current directory"),
            };

            (path, file_name)
        };

        let end = (input.len()..);

        let mut files: Vec<_> = WalkBuilder::new(dir.clone())
            .max_depth(Some(1))
            .build()
            .filter_map(|file| {
                file.ok().map(|entry| {
                    let is_dir = entry.file_type().map_or(false, |entry| entry.is_dir());

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

                    if is_dir {
                        path.push("");
                    }
                    let path = path.to_str().unwrap().to_owned();
                    (end.clone(), Cow::from(path))
                })
            }) // TODO: unwrap or skip
            .filter(|(_, path)| !path.is_empty()) // TODO
            .collect();

        // if empty, return a list of dirs and files in current dir
        if let Some(file_name) = file_name {
            let matcher = Matcher::default();

            // inefficient, but we need to calculate the scores, filter out None, then sort.
            let mut matches: Vec<_> = files
                .into_iter()
                .filter_map(|(range, file)| {
                    matcher
                        .fuzzy_match(&file, &file_name)
                        .map(|score| (file, score))
                })
                .collect();

            let range = ((input.len().saturating_sub(file_name.len()))..);

            matches.sort_unstable_by_key(|(_file, score)| Reverse(*score));
            files = matches
                .into_iter()
                .map(|(file, _)| (range.clone(), file))
                .collect();

            // TODO: complete to longest common match
        }

        files
    }
}
