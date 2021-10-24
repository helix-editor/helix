mod completion;
pub(crate) mod editor;
mod info;
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
pub use picker::{FilePicker, Picker};
pub use popup::Popup;
pub use prompt::{Prompt, PromptEvent};
pub use spinner::{ProgressSpinners, Spinner};
pub use text::Text;

use helix_core::regex::Regex;
use helix_core::regex::RegexBuilder;
use helix_view::{Document, Editor, View};

use std::path::PathBuf;

pub fn regex_prompt(
    cx: &mut crate::commands::Context,
    prompt: std::borrow::Cow<'static, str>,
    history_register: Option<char>,
    fun: impl Fn(&mut View, &mut Document, Regex, PromptEvent) + 'static,
) -> Prompt {
    let (view, doc) = current!(cx.editor);
    let view_id = view.id;
    let snapshot = doc.selection(view_id).clone();

    Prompt::new(
        prompt,
        history_register,
        |_input: &str| Vec::new(), // this is fine because Vec::new() doesn't allocate
        move |cx: &mut crate::compositor::Context, input: &str, event: PromptEvent| {
            match event {
                PromptEvent::Abort => {
                    let (view, doc) = current!(cx.editor);
                    doc.set_selection(view.id, snapshot.clone());
                }
                PromptEvent::Validate => {
                    // TODO: push_jump to store selection just before jump

                    match Regex::new(input) {
                        Ok(regex) => {
                            let (view, doc) = current!(cx.editor);
                            fun(view, doc, regex, event);
                        }
                        Err(_err) => (), // TODO: mark command line as error
                    }
                }
                PromptEvent::Update => {
                    // skip empty input, TODO: trigger default
                    if input.is_empty() {
                        return;
                    }

                    let case_insensitive = if cx.editor.config.smart_case {
                        !input.chars().any(char::is_uppercase)
                    } else {
                        false
                    };

                    match RegexBuilder::new(input)
                        .case_insensitive(case_insensitive)
                        .build()
                    {
                        Ok(regex) => {
                            let (view, doc) = current!(cx.editor);

                            // revert state to what it was before the last update
                            doc.set_selection(view.id, snapshot.clone());

                            fun(view, doc, regex, event);

                            view.ensure_cursor_in_view(doc, cx.editor.config.scrolloff);
                        }
                        Err(_err) => (), // TODO: mark command line as error
                    }
                }
            }
        },
    )
}

pub fn file_picker(root: PathBuf) -> FilePicker<PathBuf> {
    use ignore::{types::TypesBuilder, WalkBuilder};
    use std::time;

    // We want to exclude files that the editor can't handle yet
    let mut type_builder = TypesBuilder::new();
    let mut walk_builder = WalkBuilder::new(&root);
    let walk_builder = match type_builder.add(
        "compressed",
        "*.{zip,gz,bz2,zst,lzo,sz,tgz,tbz2,lz,lz4,lzma,lzo,z,Z,xz,7z,rar,cab}",
    ) {
        Err(_) => &walk_builder,
        _ => {
            type_builder.negate("all");
            let excluded_types = type_builder.build().unwrap();
            walk_builder.types(excluded_types)
        }
    };

    let files = walk_builder.build().filter_map(|entry| {
        let entry = entry.ok()?;
        // Path::is_dir() traverses symlinks, so we use it over DirEntry::is_dir
        if entry.path().is_dir() {
            // Will give a false positive if metadata cannot be read (eg. permission error)
            return None;
        }

        let time = entry.metadata().map_or(time::UNIX_EPOCH, |metadata| {
            metadata
                .accessed()
                .or_else(|_| metadata.modified())
                .or_else(|_| metadata.created())
                .unwrap_or(time::UNIX_EPOCH)
        });

        Some((entry.into_path(), time))
    });

    let mut files: Vec<_> = if root.join(".git").is_dir() {
        files.collect()
    } else {
        const MAX: usize = 8192;
        files.take(MAX).collect()
    };

    files.sort_by_key(|file| std::cmp::Reverse(file.1));

    let files = files.into_iter().map(|(path, _)| path).collect();

    FilePicker::new(
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
            editor
                .open(path.into(), action)
                .expect("editor.open failed");
        },
        |_editor, path| Some((path.clone(), None)),
    )
}

pub mod completers {
    use crate::ui::prompt::Completion;
    use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
    use fuzzy_matcher::FuzzyMatcher;
    use helix_view::theme;
    use std::borrow::Cow;
    use std::cmp::Reverse;

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
            .filter_map(|(_range, name)| {
                matcher.fuzzy_match(&name, input).map(|score| (name, score))
            })
            .collect();

        matches.sort_unstable_by_key(|(_file, score)| Reverse(*score));
        names = matches.into_iter().map(|(name, _)| ((0..), name)).collect();

        names
    }

    pub fn filename(input: &str) -> Vec<Completion> {
        filename_impl(input, |entry| {
            let is_dir = entry.file_type().map_or(false, |entry| entry.is_dir());

            if is_dir {
                FileMatch::AcceptIncomplete
            } else {
                FileMatch::Accept
            }
        })
    }

    pub fn directory(input: &str) -> Vec<Completion> {
        filename_impl(input, |entry| {
            let is_dir = entry.file_type().map_or(false, |entry| entry.is_dir());

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
    fn filename_impl<F>(input: &str, filter_fn: F) -> Vec<Completion>
    where
        F: Fn(&ignore::DirEntry) -> FileMatch,
    {
        // Rust's filename handling is really annoying.

        use ignore::WalkBuilder;
        use std::path::Path;

        let is_tilde = input.starts_with('~') && input.len() == 1;
        let path = helix_core::path::expand_tilde(Path::new(input));

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

        let end = input.len()..;

        let mut files: Vec<_> = WalkBuilder::new(&dir)
            .hidden(false)
            .max_depth(Some(1))
            .build()
            .filter_map(|file| {
                file.ok().and_then(|entry| {
                    let fmatch = filter_fn(&entry);

                    if fmatch == FileMatch::Reject {
                        return None;
                    }

                    //let is_dir = entry.file_type().map_or(false, |entry| entry.is_dir());

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

                    let path = path.to_str().unwrap().to_owned();
                    Some((end.clone(), Cow::from(path)))
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
                .filter_map(|(_range, file)| {
                    matcher
                        .fuzzy_match(&file, &file_name)
                        .map(|score| (file, score))
                })
                .collect();

            let range = (input.len().saturating_sub(file_name.len()))..;

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
