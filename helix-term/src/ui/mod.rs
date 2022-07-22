mod completion;
pub(crate) mod editor;
mod info;
pub mod lsp;
mod markdown;
pub mod menu;
pub mod overlay;
mod picker;
pub mod popup;
mod prompt;
mod spinner;
mod statusline;
mod text;

pub use completion::Completion;
pub use editor::EditorView;
pub use markdown::Markdown;
pub use menu::Menu;
pub use picker::{FileLocation, FilePicker, Picker};
pub use popup::Popup;
pub use prompt::{Prompt, PromptEvent};
pub use spinner::{ProgressSpinners, Spinner};
pub use text::Text;

use helix_core::regex::Regex;
use helix_core::regex::RegexBuilder;
use helix_view::{Document, Editor, View};

use std::path::PathBuf;

pub fn prompt(
    cx: &mut crate::commands::Context,
    prompt: std::borrow::Cow<'static, str>,
    history_register: Option<char>,
    completion_fn: impl FnMut(&Editor, &str) -> Vec<prompt::Completion> + 'static,
    callback_fn: impl FnMut(&mut crate::compositor::Context, &str, PromptEvent) + 'static,
) {
    show_prompt(
        cx,
        Prompt::new(prompt, history_register, completion_fn, callback_fn),
    );
}

pub fn prompt_with_input(
    cx: &mut crate::commands::Context,
    prompt: std::borrow::Cow<'static, str>,
    input: String,
    history_register: Option<char>,
    completion_fn: impl FnMut(&Editor, &str) -> Vec<prompt::Completion> + 'static,
    callback_fn: impl FnMut(&mut crate::compositor::Context, &str, PromptEvent) + 'static,
) {
    show_prompt(
        cx,
        Prompt::new(prompt, history_register, completion_fn, callback_fn).with_line(input),
    );
}

fn show_prompt(cx: &mut crate::commands::Context, mut prompt: Prompt) {
    // Calculate initial completion
    prompt.recalculate_completion(cx.editor);
    cx.push_layer(Box::new(prompt));
}

pub fn regex_prompt(
    cx: &mut crate::commands::Context,
    prompt: std::borrow::Cow<'static, str>,
    history_register: Option<char>,
    completion_fn: impl FnMut(&Editor, &str) -> Vec<prompt::Completion> + 'static,
    fun: impl Fn(&mut View, &mut Document, Regex, PromptEvent) + 'static,
) {
    let (view, doc) = current!(cx.editor);
    let doc_id = view.doc;
    let snapshot = doc.selection(view.id).clone();
    let offset_snapshot = view.offset;
    let config = cx.editor.config();

    let mut prompt = Prompt::new(
        prompt,
        history_register,
        completion_fn,
        move |cx: &mut crate::compositor::Context, input: &str, event: PromptEvent| {
            match event {
                PromptEvent::Abort => {
                    let (view, doc) = current!(cx.editor);
                    doc.set_selection(view.id, snapshot.clone());
                    view.offset = offset_snapshot;
                }
                PromptEvent::Update | PromptEvent::Validate => {
                    // skip empty input
                    if input.is_empty() {
                        return;
                    }

                    let case_insensitive = if config.search.smart_case {
                        !input.chars().any(char::is_uppercase)
                    } else {
                        false
                    };

                    match RegexBuilder::new(input)
                        .case_insensitive(case_insensitive)
                        .multi_line(true)
                        .build()
                    {
                        Ok(regex) => {
                            let (view, doc) = current!(cx.editor);

                            // revert state to what it was before the last update
                            doc.set_selection(view.id, snapshot.clone());

                            if event == PromptEvent::Validate {
                                // Equivalent to push_jump to store selection just before jump
                                view.jumps.push((doc_id, snapshot.clone()));
                            }

                            fun(view, doc, regex, event);

                            view.ensure_cursor_in_view(doc, config.scrolloff);
                        }
                        Err(_err) => (), // TODO: mark command line as error
                    }
                }
            }
        },
    );
    // Calculate initial completion
    prompt.recalculate_completion(cx.editor);
    // prompt
    cx.push_layer(Box::new(prompt));
}

pub fn file_picker(root: PathBuf, config: &helix_view::editor::Config) -> FilePicker<PathBuf> {
    use ignore::{types::TypesBuilder, WalkBuilder};
    use std::time::Instant;

    let now = Instant::now();

    let mut walk_builder = WalkBuilder::new(&root);
    walk_builder
        .hidden(config.file_picker.hidden)
        .parents(config.file_picker.parents)
        .ignore(config.file_picker.ignore)
        .follow_links(config.file_picker.follow_symlinks)
        .git_ignore(config.file_picker.git_ignore)
        .git_global(config.file_picker.git_global)
        .git_exclude(config.file_picker.git_exclude)
        .max_depth(config.file_picker.max_depth)
        // We always want to ignore the .git directory, otherwise if
        // `ignore` is turned off above, we end up with a lot of noise
        // in our picker.
        .filter_entry(|entry| entry.file_name() != ".git");

    // We want to exclude files that the editor can't handle yet
    let mut type_builder = TypesBuilder::new();
    type_builder
        .add(
            "compressed",
            "*.{zip,gz,bz2,zst,lzo,sz,tgz,tbz2,lz,lz4,lzma,lzo,z,Z,xz,7z,rar,cab}",
        )
        .expect("Invalid type definition");
    type_builder.negate("all");
    let excluded_types = type_builder
        .build()
        .expect("failed to build excluded_types");
    walk_builder.types(excluded_types);

    // We want files along with their modification date for sorting
    let files = walk_builder.build().filter_map(|entry| {
        let entry = entry.ok()?;

        // This is faster than entry.path().is_dir() since it uses cached fs::Metadata fetched by ignore/walkdir
        let is_dir = entry.file_type().map_or(false, |ft| ft.is_dir());
        if is_dir {
            // Will give a false positive if metadata cannot be read (eg. permission error)
            None
        } else {
            Some(entry.into_path())
        }
    });

    // Cap the number of files if we aren't in a git project, preventing
    // hangs when using the picker in your home directory
    let files: Vec<_> = if root.join(".git").is_dir() {
        files.collect()
    } else {
        // const MAX: usize = 8192;
        const MAX: usize = 100_000;
        files.take(MAX).collect()
    };

    log::debug!("file_picker init {:?}", Instant::now().duration_since(now));

    FilePicker::new(
        files,
        root,
        move |cx, path: &PathBuf, action| {
            if let Err(e) = cx.editor.open(path, action) {
                let err = if let Some(err) = e.source() {
                    format!("{}", err)
                } else {
                    format!("unable to open \"{}\"", path.display())
                };
                cx.editor.set_error(err);
            }
        },
        |_editor, path| Some((path.clone(), None)),
    )
}

pub mod completers {
    use crate::ui::prompt::Completion;
    use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
    use fuzzy_matcher::FuzzyMatcher;
    use helix_view::document::SCRATCH_BUFFER_NAME;
    use helix_view::theme;
    use helix_view::{editor::Config, Editor};
    use once_cell::sync::Lazy;
    use std::borrow::Cow;
    use std::cmp::Reverse;

    pub type Completer = fn(&Editor, &str) -> Vec<Completion>;

    pub fn none(_editor: &Editor, _input: &str) -> Vec<Completion> {
        Vec::new()
    }

    pub fn buffer(editor: &Editor, input: &str) -> Vec<Completion> {
        let mut names: Vec<_> = editor
            .documents
            .iter()
            .map(|(_id, doc)| {
                let name = doc
                    .relative_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| String::from(SCRATCH_BUFFER_NAME));
                ((0..), Cow::from(name))
            })
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

    pub fn theme(_editor: &Editor, input: &str) -> Vec<Completion> {
        let mut names = theme::Loader::read_names(&helix_loader::runtime_dir().join("themes"));
        names.extend(theme::Loader::read_names(
            &helix_loader::config_dir().join("themes"),
        ));
        names.push("default".into());
        names.push("base16_default".into());
        names.sort();

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

        matches.sort_unstable_by(|(name1, score1), (name2, score2)| {
            (Reverse(*score1), name1).cmp(&(Reverse(*score2), name2))
        });
        names = matches.into_iter().map(|(name, _)| ((0..), name)).collect();

        names
    }

    pub fn setting(_editor: &Editor, input: &str) -> Vec<Completion> {
        static KEYS: Lazy<Vec<String>> = Lazy::new(|| {
            serde_json::json!(Config::default())
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect()
        });

        let matcher = Matcher::default();

        let mut matches: Vec<_> = KEYS
            .iter()
            .filter_map(|name| matcher.fuzzy_match(name, input).map(|score| (name, score)))
            .collect();

        matches.sort_unstable_by_key(|(_file, score)| Reverse(*score));
        matches
            .into_iter()
            .map(|(name, _)| ((0..), name.into()))
            .collect()
    }

    pub fn filename(editor: &Editor, input: &str) -> Vec<Completion> {
        filename_impl(editor, input, |entry| {
            let is_dir = entry.file_type().map_or(false, |entry| entry.is_dir());

            if is_dir {
                FileMatch::AcceptIncomplete
            } else {
                FileMatch::Accept
            }
        })
    }

    pub fn language(editor: &Editor, input: &str) -> Vec<Completion> {
        let matcher = Matcher::default();

        let mut matches: Vec<_> = editor
            .syn_loader
            .language_configs()
            .filter_map(|config| {
                matcher
                    .fuzzy_match(&config.language_id, input)
                    .map(|score| (&config.language_id, score))
            })
            .collect();

        matches.sort_unstable_by(|(language1, score1), (language2, score2)| {
            (Reverse(*score1), language1).cmp(&(Reverse(*score2), language2))
        });

        matches
            .into_iter()
            .map(|(language, _score)| ((0..), language.clone().into()))
            .collect()
    }

    pub fn directory(editor: &Editor, input: &str) -> Vec<Completion> {
        filename_impl(editor, input, |entry| {
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
    fn filename_impl<F>(editor: &Editor, input: &str, filter_fn: F) -> Vec<Completion>
    where
        F: Fn(&ignore::DirEntry) -> FileMatch,
    {
        // Rust's filename handling is really annoying.

        use ignore::WalkBuilder;
        use std::path::Path;

        let is_tilde = input.starts_with('~') && input.len() == 1;
        let path = helix_core::path::expand_tilde(Path::new(input));

        let (dir, file_name) = if input.ends_with(std::path::MAIN_SEPARATOR) {
            (path, None)
        } else {
            let file_name = path
                .file_name()
                .and_then(|file| file.to_str().map(|path| path.to_owned()));

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
            .follow_links(editor.config().file_picker.follow_symlinks)
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

                    let path = path.to_str()?.to_owned();
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

            matches.sort_unstable_by(|(file1, score1), (file2, score2)| {
                (Reverse(*score1), file1).cmp(&(Reverse(*score2), file2))
            });

            files = matches
                .into_iter()
                .map(|(file, _)| (range.clone(), file))
                .collect();

            // TODO: complete to longest common match
        } else {
            files.sort_unstable_by(|(_, path1), (_, path2)| path1.cmp(path2));
        }

        files
    }
}
