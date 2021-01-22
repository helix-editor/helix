mod editor;
mod picker;
mod prompt;

pub use editor::EditorView;
pub use picker::Picker;
pub use prompt::{Prompt, PromptEvent};

pub use tui::layout::Rect;
pub use tui::style::{Color, Modifier, Style};

use helix_core::regex::Regex;
use helix_view::{Document, Editor};

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
    let snapshot = cx.doc().state.clone();

    Prompt::new(
        prompt,
        |input: &str| Vec::new(), // this is fine because Vec::new() doesn't allocate
        move |editor: &mut Editor, input: &str, event: PromptEvent| {
            match event {
                PromptEvent::Abort => {
                    // revert state
                    let doc = &mut editor.view_mut().doc;
                    doc.state = snapshot.clone();
                }
                PromptEvent::Validate => {
                    //
                }
                PromptEvent::Update => {
                    match Regex::new(input) {
                        Ok(regex) => {
                            let doc = &mut editor.view_mut().doc;

                            // revert state to what it was before the last update
                            doc.state = snapshot.clone();

                            fun(doc, regex);
                        }
                        Err(_err) => (), // TODO: mark command line as error
                    }
                }
            }
        },
    )
}

use std::path::{Path, PathBuf};
pub fn file_picker(root: &str, ex: &'static smol::Executor) -> Picker<PathBuf> {
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
        move |editor: &mut Editor, path: &PathBuf| {
            let size = editor.view().size;
            editor.open(path.into(), size, ex);
        },
    )
}

use helix_view::View;
pub fn buffer_picker(views: &[View], current: usize) -> Picker<(Option<PathBuf>, usize)> {
    use helix_view::Editor;
    Picker::new(
        views
            .iter()
            .enumerate()
            .map(|(i, view)| (view.doc.relative_path().map(Path::to_path_buf), i))
            .collect(),
        move |(path, index): &(Option<PathBuf>, usize)| {
            // format_fn
            match path {
                Some(path) => {
                    if *index == current {
                        format!("{} (*)", path.to_str().unwrap()).into()
                    } else {
                        path.to_str().unwrap().into()
                    }
                }
                None => "[NEW]".into(),
            }
        },
        |editor: &mut Editor, &(_, index): &(Option<PathBuf>, usize)| {
            if index < editor.views.len() {
                editor.focus = index;
            }
        },
    )
}
