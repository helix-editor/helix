mod completion;
mod document;
pub(crate) mod editor;
mod info;
pub mod lsp;
mod markdown;
pub mod menu;
pub mod overlay;
pub mod picker;
pub mod popup;
mod prompt;
mod spinner;
mod statusline;
mod text;

use crate::compositor::{Component, Compositor};
use crate::filter_picker_entry;
use crate::job::{self, Callback};
pub use completion::{Completion, CompletionItem};
pub use editor::EditorView;
pub use markdown::Markdown;
pub use menu::Menu;
pub use picker::{DynamicPicker, FileLocation, Picker};
pub use popup::Popup;
pub use prompt::{Prompt, PromptEvent};
pub use spinner::{ProgressSpinners, Spinner};
pub use text::Text;

use helix_core::regex::Regex;
use helix_core::regex::RegexBuilder;
use helix_view::Editor;

use std::path::PathBuf;

pub fn prompt(
    cx: &mut crate::commands::Context,
    prompt: std::borrow::Cow<'static, str>,
    history_register: Option<char>,
    completion_fn: impl FnMut(&Editor, &str) -> Vec<prompt::Completion> + 'static,
    callback_fn: impl FnMut(&mut crate::compositor::Context, &str, PromptEvent) + 'static,
) {
    let mut prompt = Prompt::new(prompt, history_register, completion_fn, callback_fn);
    // Calculate the initial completion
    prompt.recalculate_completion(cx.editor);
    cx.push_layer(Box::new(prompt));
}

pub fn prompt_with_input(
    cx: &mut crate::commands::Context,
    prompt: std::borrow::Cow<'static, str>,
    input: String,
    history_register: Option<char>,
    completion_fn: impl FnMut(&Editor, &str) -> Vec<prompt::Completion> + 'static,
    callback_fn: impl FnMut(&mut crate::compositor::Context, &str, PromptEvent) + 'static,
) {
    let prompt = Prompt::new(prompt, history_register, completion_fn, callback_fn)
        .with_line(input, cx.editor);
    cx.push_layer(Box::new(prompt));
}

pub fn regex_prompt(
    cx: &mut crate::commands::Context,
    prompt: std::borrow::Cow<'static, str>,
    history_register: Option<char>,
    completion_fn: impl FnMut(&Editor, &str) -> Vec<prompt::Completion> + 'static,
    fun: impl Fn(&mut crate::compositor::Context, Regex, PromptEvent) + 'static,
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

                            fun(cx, regex, event);

                            let (view, doc) = current!(cx.editor);
                            view.ensure_cursor_in_view(doc, config.scrolloff);
                        }
                        Err(err) => {
                            let (view, doc) = current!(cx.editor);
                            doc.set_selection(view.id, snapshot.clone());
                            view.offset = offset_snapshot;

                            if event == PromptEvent::Validate {
                                let callback = async move {
                                    let call: job::Callback = Callback::EditorCompositor(Box::new(
                                        move |_editor: &mut Editor, compositor: &mut Compositor| {
                                            let contents = Text::new(format!("{}", err));
                                            let size = compositor.size();
                                            let mut popup = Popup::new("invalid-regex", contents)
                                                .position(Some(helix_core::Position::new(
                                                    size.height as usize - 2, // 2 = statusline + commandline
                                                    0,
                                                )))
                                                .auto_close(true);
                                            popup.required_size((size.width, size.height));

                                            compositor.replace_or_push("invalid-regex", popup);
                                        },
                                    ));
                                    Ok(call)
                                };

                                cx.jobs.callback(callback);
                            }
                        }
                    }
                }
            }
        },
    )
    .with_language("regex", std::sync::Arc::clone(&cx.editor.syn_loader));
    // Calculate initial completion
    prompt.recalculate_completion(cx.editor);
    // prompt
    cx.push_layer(Box::new(prompt));
}

pub fn file_picker(root: PathBuf, config: &helix_view::editor::Config) -> Picker<PathBuf> {
    use ignore::{types::TypesBuilder, WalkBuilder};
    use std::time::Instant;

    let now = Instant::now();

    let dedup_symlinks = config.file_picker.deduplicate_links;
    let absolute_root = root.canonicalize().unwrap_or_else(|_| root.clone());

    let mut walk_builder = WalkBuilder::new(&root);
    walk_builder
        .hidden(config.file_picker.hidden)
        .parents(config.file_picker.parents)
        .ignore(config.file_picker.ignore)
        .follow_links(config.file_picker.follow_symlinks)
        .git_ignore(config.file_picker.git_ignore)
        .git_global(config.file_picker.git_global)
        .git_exclude(config.file_picker.git_exclude)
        .sort_by_file_name(|name1, name2| name1.cmp(name2))
        .max_depth(config.file_picker.max_depth)
        .filter_entry(move |entry| filter_picker_entry(entry, &absolute_root, dedup_symlinks));

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
    let mut files = walk_builder.build().filter_map(|entry| {
        let entry = entry.ok()?;
        if !entry.file_type()?.is_file() {
            return None;
        }
        Some(entry.into_path())
    });
    log::debug!("file_picker init {:?}", Instant::now().duration_since(now));

    let picker = Picker::new(Vec::new(), root, move |cx, path: &PathBuf, action| {
        if let Err(e) = cx.editor.open(path, action) {
            let err = if let Some(err) = e.source() {
                format!("{}", err)
            } else {
                format!("unable to open \"{}\"", path.display())
            };
            cx.editor.set_error(err);
        }
    })
    .with_preview(|_editor, path| Some((path.clone().into(), None)));
    let injector = picker.injector();
    let timeout = std::time::Instant::now() + std::time::Duration::from_millis(30);

    let mut hit_timeout = false;
    for file in &mut files {
        if injector.push(file).is_err() {
            break;
        }
        if std::time::Instant::now() >= timeout {
            hit_timeout = true;
            break;
        }
    }
    if hit_timeout {
        std::thread::spawn(move || {
            for file in files {
                if injector.push(file).is_err() {
                    break;
                }
            }
        });
    }
    picker
}

pub mod completers {
    use crate::ui::prompt::Completion;
    use helix_core::fuzzy::fuzzy_match;
    use helix_core::syntax::LanguageServerFeature;
    use helix_view::document::SCRATCH_BUFFER_NAME;
    use helix_view::theme;
    use helix_view::{editor::Config, Editor};
    use once_cell::sync::Lazy;
    use std::borrow::Cow;

    pub type Completer = fn(&Editor, &str) -> Vec<Completion>;

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
            .map(|(name, _)| ((0..), name))
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

    pub fn setting(_editor: &Editor, input: &str) -> Vec<Completion> {
        static KEYS: Lazy<Vec<String>> = Lazy::new(|| {
            let mut keys = Vec::new();
            let json = serde_json::json!(Config::default());
            get_keys(&json, &mut keys, None);
            keys
        });

        fuzzy_match(input, &*KEYS, false)
            .into_iter()
            .map(|(name, _)| ((0..), name.into()))
            .collect()
    }

    pub fn filename(editor: &Editor, input: &str) -> Vec<Completion> {
        filename_with_git_ignore(editor, input, true)
    }

    pub fn filename_with_git_ignore(
        editor: &Editor,
        input: &str,
        git_ignore: bool,
    ) -> Vec<Completion> {
        filename_impl(editor, input, git_ignore, |entry| {
            let is_dir = entry.file_type().map_or(false, |entry| entry.is_dir());

            if is_dir {
                FileMatch::AcceptIncomplete
            } else {
                FileMatch::Accept
            }
        })
    }

    pub fn language(editor: &Editor, input: &str) -> Vec<Completion> {
        let text: String = "text".into();

        let language_ids = editor
            .syn_loader
            .language_configs()
            .map(|config| &config.language_id)
            .chain(std::iter::once(&text));

        fuzzy_match(input, language_ids, false)
            .into_iter()
            .map(|(name, _)| ((0..), name.to_owned().into()))
            .collect()
    }

    pub fn lsp_workspace_command(editor: &Editor, input: &str) -> Vec<Completion> {
        let Some(options) = doc!(editor)
            .language_servers_with_feature(LanguageServerFeature::WorkspaceCommand)
            .find_map(|ls| ls.capabilities().execute_command_provider.as_ref())
        else {
            return vec![];
        };

        fuzzy_match(input, &options.commands, false)
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
    fn filename_impl<F>(
        _editor: &Editor,
        input: &str,
        git_ignore: bool,
        filter_fn: F,
    ) -> Vec<Completion>
    where
        F: Fn(&ignore::DirEntry) -> FileMatch,
    {
        // Rust's filename handling is really annoying.

        use ignore::WalkBuilder;
        use std::path::Path;

        let is_tilde = input == "~";
        let path = helix_core::path::expand_tilde(Path::new(input));

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
                    Some(path) if !path.as_os_str().is_empty() => path.to_path_buf(),
                    // Path::new("h")'s parent is Some("")...
                    _ => helix_loader::current_working_dir(),
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

                    let path = path.into_os_string().into_string().ok()?;
                    Some(Cow::from(path))
                })
            }) // TODO: unwrap or skip
            .filter(|path| !path.is_empty());

        // if empty, return a list of dirs and files in current dir
        if let Some(file_name) = file_name {
            let range = (input.len().saturating_sub(file_name.len()))..;
            fuzzy_match(&file_name, files, true)
                .into_iter()
                .map(|(name, _)| (range.clone(), name))
                .collect()

            // TODO: complete to longest common match
        } else {
            let mut files: Vec<_> = files.map(|file| (end.clone(), file)).collect();
            files.sort_unstable_by(|(_, path1), (_, path2)| path1.cmp(path2));
            files
        }
    }
}
