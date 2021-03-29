mod completion;
mod editor;
mod markdown;
mod menu;
mod picker;
mod popup;
mod prompt;
mod text;

pub use completion::Completion;
pub use editor::EditorView;
pub use markdown::Markdown;
pub use menu::Menu;
pub use picker::Picker;
pub use popup::Popup;
pub use prompt::{Prompt, PromptEvent};
pub use text::Text;

pub use tui::layout::Rect;
pub use tui::style::{Color, Modifier, Style};

use helix_core::regex::Regex;
use helix_view::{Document, Editor};

use std::path::{Path, PathBuf};

// TODO: temp
#[inline(always)]
pub fn text_color() -> Style {
    Style::default().fg(Color::Rgb(219, 191, 239)) // lilac
}

pub fn regex_prompt(
    cx: &mut crate::commands::Context,
    prompt: String,
    fun: impl Fn(&mut Document, Regex) + 'static,
) -> Prompt {
    let snapshot = cx.doc().selection().clone();

    Prompt::new(
        prompt,
        |input: &str| Vec::new(), // this is fine because Vec::new() doesn't allocate
        move |editor: &mut Editor, input: &str, event: PromptEvent| {
            match event {
                PromptEvent::Abort => {
                    // TODO: also revert text
                    let id = editor.view().doc;
                    let doc = &mut editor.documents[id];
                    doc.set_selection(snapshot.clone());
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
                            // let view = &mut editor.view_mut();
                            let id = editor.view().doc;
                            let doc = &mut editor.documents[id];

                            // revert state to what it was before the last update
                            // TODO: also revert text
                            doc.set_selection(snapshot.clone());

                            fun(doc, regex);

                            editor.ensure_cursor_in_view(editor.view().id);
                        }
                        Err(_err) => (), // TODO: mark command line as error
                    }
                }
            }
        },
    )
}

pub fn file_picker(root: &str) -> Picker<PathBuf> {
    use ignore::Walk;
    // TODO: determine root based on git root
    let files = Walk::new(root).filter_map(|entry| match entry {
        Ok(entry) => {
            // filter dirs, but we might need special handling for symlinks!
            if !entry.file_type().unwrap().is_dir() {
                Some(entry.into_path())
            } else {
                None
            }
        }
        Err(_err) => None,
    });

    const MAX: usize = 1024;

    use helix_view::Editor;
    Picker::new(
        files.take(MAX).collect(),
        |path: &PathBuf| {
            // format_fn
            path.strip_prefix("./").unwrap().to_str().unwrap().into()
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
    use std::borrow::Cow;
    // TODO: we could return an iter/lazy thing so it can fetch as many as it needs.
    pub fn filename(input: &str) -> Vec<Completion> {
        // Rust's filename handling is really annoying.

        use ignore::WalkBuilder;
        use std::path::{Path, PathBuf};

        let path = Path::new(input);

        let (dir, file_name) = if input.ends_with('/') {
            (path.into(), None)
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
                    let is_dir = entry
                        .file_type()
                        .map(|entry| entry.is_dir())
                        .unwrap_or(false);

                    let mut path = entry.path().strip_prefix(&dir).unwrap().to_path_buf();

                    if is_dir {
                        path.push("");
                    }
                    let path = path.to_str().unwrap().to_string();
                    (end.clone(), Cow::from(path))
                })
            }) // TODO: unwrap or skip
            .filter(|(_, path)| !path.is_empty()) // TODO
            .collect();

        // if empty, return a list of dirs and files in current dir
        if let Some(file_name) = file_name {
            use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
            use fuzzy_matcher::FuzzyMatcher;
            use std::cmp::Reverse;

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

            let range = ((input.len() - file_name.len())..);

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
