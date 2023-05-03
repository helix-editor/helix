use std::fmt::Write;
use std::ops::Deref;

use crate::job::Job;

use super::*;

use helix_core::{encoding, shellwords::Shellwords};
use helix_view::document::DEFAULT_LANGUAGE_NAME;
use helix_view::editor::{Action, CloseError, ConfigEvent};
use serde_json::Value;
use ui::completers::{self, Completer};

#[derive(Clone)]
pub struct TypableCommand {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub doc: &'static str,
    // params, flags, helper, completer
    pub fun: fn(&mut compositor::Context, &[Cow<str>], PromptEvent) -> anyhow::Result<()>,
    /// What completion methods, if any, does this command have?
    pub signature: CommandSignature,
}

impl TypableCommand {
    fn completer_for_argument_number(&self, n: usize) -> &Completer {
        match self.signature.positional_args.get(n) {
            Some(completer) => completer,
            _ => &self.signature.var_args,
        }
    }
}

#[derive(Clone)]
pub struct CommandSignature {
    // Arguments with specific completion methods based on their position.
    positional_args: &'static [Completer],

    // All remaining arguments will use this completion method, if set.
    var_args: Completer,
}

impl CommandSignature {
    const fn none() -> Self {
        Self {
            positional_args: &[],
            var_args: completers::none,
        }
    }

    const fn positional(completers: &'static [Completer]) -> Self {
        Self {
            positional_args: completers,
            var_args: completers::none,
        }
    }

    const fn all(completer: Completer) -> Self {
        Self {
            positional_args: &[],
            var_args: completer,
        }
    }
}

fn quit(cx: &mut compositor::Context, args: &[Cow<str>], event: PromptEvent) -> anyhow::Result<()> {
    log::debug!("quitting...");

    if event != PromptEvent::Validate {
        return Ok(());
    }

    ensure!(args.is_empty(), ":quit takes no arguments");

    // last view and we have unsaved changes
    if cx.editor.tree.views().count() == 1 {
        buffers_remaining_impl(cx.editor)?
    }

    cx.block_try_flush_writes()?;
    cx.editor.close(view!(cx.editor).id);

    Ok(())
}

fn force_quit(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    ensure!(args.is_empty(), ":quit! takes no arguments");

    cx.block_try_flush_writes()?;
    cx.editor.close(view!(cx.editor).id);

    Ok(())
}

fn open(cx: &mut compositor::Context, args: &[Cow<str>], event: PromptEvent) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    ensure!(!args.is_empty(), "wrong argument count");
    for arg in args {
        let (path, pos) = args::parse_file(arg);
        let path = helix_core::path::expand_tilde(&path);
        // If the path is a directory, open a file picker on that directory and update the status
        // message
        if let Ok(true) = std::fs::canonicalize(&path).map(|p| p.is_dir()) {
            let callback = async move {
                let call: job::Callback = job::Callback::EditorCompositor(Box::new(
                    move |editor: &mut Editor, compositor: &mut Compositor| {
                        let picker = ui::file_picker(path, &editor.config());
                        compositor.push(Box::new(overlaid(picker)));
                    },
                ));
                Ok(call)
            };
            cx.jobs.callback(callback);
        } else {
            // Otherwise, just open the file
            let _ = cx.editor.open(&path, Action::Replace)?;
            let (view, doc) = current!(cx.editor);
            let pos = Selection::point(pos_at_coords(doc.text().slice(..), pos, true));
            doc.set_selection(view.id, pos);
            // does not affect opening a buffer without pos
            align_view(doc, view, Align::Center);
        }
    }
    Ok(())
}

fn buffer_close_by_ids_impl(
    cx: &mut compositor::Context,
    doc_ids: &[DocumentId],
    force: bool,
) -> anyhow::Result<()> {
    cx.block_try_flush_writes()?;

    let (modified_ids, modified_names): (Vec<_>, Vec<_>) = doc_ids
        .iter()
        .filter_map(|&doc_id| {
            if let Err(CloseError::BufferModified(name)) = cx.editor.close_document(doc_id, force) {
                Some((doc_id, name))
            } else {
                None
            }
        })
        .unzip();

    if let Some(first) = modified_ids.first() {
        let current = doc!(cx.editor);
        // If the current document is unmodified, and there are modified
        // documents, switch focus to the first modified doc.
        if !modified_ids.contains(&current.id()) {
            cx.editor.switch(*first, Action::Replace);
        }
        bail!(
            "{} unsaved buffer(s) remaining: {:?}",
            modified_names.len(),
            modified_names
        );
    }

    Ok(())
}

fn buffer_gather_paths_impl(editor: &mut Editor, args: &[Cow<str>]) -> Vec<DocumentId> {
    // No arguments implies current document
    if args.is_empty() {
        let doc_id = view!(editor).doc;
        return vec![doc_id];
    }

    let mut nonexistent_buffers = vec![];
    let mut document_ids = vec![];
    for arg in args {
        let doc_id = editor.documents().find_map(|doc| {
            let arg_path = Some(Path::new(arg.as_ref()));
            if doc.path().map(|p| p.as_path()) == arg_path
                || doc.relative_path().as_deref() == arg_path
            {
                Some(doc.id())
            } else {
                None
            }
        });

        match doc_id {
            Some(doc_id) => document_ids.push(doc_id),
            None => nonexistent_buffers.push(format!("'{}'", arg)),
        }
    }

    if !nonexistent_buffers.is_empty() {
        editor.set_error(format!(
            "cannot close non-existent buffers: {}",
            nonexistent_buffers.join(", ")
        ));
    }

    document_ids
}

fn buffer_close(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let document_ids = buffer_gather_paths_impl(cx.editor, args);
    buffer_close_by_ids_impl(cx, &document_ids, false)
}

fn force_buffer_close(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let document_ids = buffer_gather_paths_impl(cx.editor, args);
    buffer_close_by_ids_impl(cx, &document_ids, true)
}

fn buffer_gather_others_impl(editor: &mut Editor) -> Vec<DocumentId> {
    let current_document = &doc!(editor).id();
    editor
        .documents()
        .map(|doc| doc.id())
        .filter(|doc_id| doc_id != current_document)
        .collect()
}

fn buffer_close_others(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let document_ids = buffer_gather_others_impl(cx.editor);
    buffer_close_by_ids_impl(cx, &document_ids, false)
}

fn force_buffer_close_others(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let document_ids = buffer_gather_others_impl(cx.editor);
    buffer_close_by_ids_impl(cx, &document_ids, true)
}

fn buffer_gather_all_impl(editor: &mut Editor) -> Vec<DocumentId> {
    editor.documents().map(|doc| doc.id()).collect()
}

fn buffer_close_all(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let document_ids = buffer_gather_all_impl(cx.editor);
    buffer_close_by_ids_impl(cx, &document_ids, false)
}

fn force_buffer_close_all(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let document_ids = buffer_gather_all_impl(cx.editor);
    buffer_close_by_ids_impl(cx, &document_ids, true)
}

fn buffer_next(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    goto_buffer(cx.editor, Direction::Forward);
    Ok(())
}

fn buffer_previous(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    goto_buffer(cx.editor, Direction::Backward);
    Ok(())
}

fn write_impl(
    cx: &mut compositor::Context,
    path: Option<&Cow<str>>,
    force: bool,
) -> anyhow::Result<()> {
    let editor_auto_fmt = cx.editor.config().auto_format;
    let jobs = &mut cx.jobs;
    let (view, doc) = current!(cx.editor);
    let path = path.map(AsRef::as_ref);

    let fmt = if editor_auto_fmt {
        doc.auto_format().map(|fmt| {
            let callback = make_format_callback(
                doc.id(),
                doc.version(),
                view.id,
                fmt,
                Some((path.map(Into::into), force)),
            );

            jobs.add(Job::with_callback(callback).wait_before_exiting());
        })
    } else {
        None
    };

    if fmt.is_none() {
        let id = doc.id();
        cx.editor.save(id, path, force)?;
    }

    Ok(())
}

fn write(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    write_impl(cx, args.first(), false)
}

fn force_write(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    write_impl(cx, args.first(), true)
}

fn write_buffer_close(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    write_impl(cx, args.first(), false)?;

    let document_ids = buffer_gather_paths_impl(cx.editor, args);
    buffer_close_by_ids_impl(cx, &document_ids, false)
}

fn force_write_buffer_close(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    write_impl(cx, args.first(), true)?;

    let document_ids = buffer_gather_paths_impl(cx.editor, args);
    buffer_close_by_ids_impl(cx, &document_ids, false)
}

fn new_file(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    cx.editor.new_file(Action::Replace);

    Ok(())
}

fn format(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (view, doc) = current!(cx.editor);
    if let Some(format) = doc.format() {
        let callback = make_format_callback(doc.id(), doc.version(), view.id, format, None);
        cx.jobs.callback(callback);
    }

    Ok(())
}
fn set_indent_style(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    use IndentStyle::*;

    // If no argument, report current indent style.
    if args.is_empty() {
        let style = doc!(cx.editor).indent_style;
        cx.editor.set_status(match style {
            Tabs => "tabs".to_owned(),
            Spaces(1) => "1 space".to_owned(),
            Spaces(n) if (2..=8).contains(&n) => format!("{} spaces", n),
            _ => unreachable!(), // Shouldn't happen.
        });
        return Ok(());
    }

    // Attempt to parse argument as an indent style.
    let style = match args.get(0) {
        Some(arg) if "tabs".starts_with(&arg.to_lowercase()) => Some(Tabs),
        Some(Cow::Borrowed("0")) => Some(Tabs),
        Some(arg) => arg
            .parse::<u8>()
            .ok()
            .filter(|n| (1..=8).contains(n))
            .map(Spaces),
        _ => None,
    };

    let style = style.context("invalid indent style")?;
    let doc = doc_mut!(cx.editor);
    doc.indent_style = style;

    Ok(())
}

/// Sets or reports the current document's line ending setting.
fn set_line_ending(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    use LineEnding::*;

    // If no argument, report current line ending setting.
    if args.is_empty() {
        let line_ending = doc!(cx.editor).line_ending;
        cx.editor.set_status(match line_ending {
            Crlf => "crlf",
            LF => "line feed",
            #[cfg(feature = "unicode-lines")]
            FF => "form feed",
            #[cfg(feature = "unicode-lines")]
            CR => "carriage return",
            #[cfg(feature = "unicode-lines")]
            Nel => "next line",

            // These should never be a document's default line ending.
            #[cfg(feature = "unicode-lines")]
            VT | LS | PS => "error",
        });

        return Ok(());
    }

    let arg = args
        .get(0)
        .context("argument missing")?
        .to_ascii_lowercase();

    // Attempt to parse argument as a line ending.
    let line_ending = match arg {
        arg if arg.starts_with("crlf") => Crlf,
        arg if arg.starts_with("lf") => LF,
        #[cfg(feature = "unicode-lines")]
        arg if arg.starts_with("cr") => CR,
        #[cfg(feature = "unicode-lines")]
        arg if arg.starts_with("ff") => FF,
        #[cfg(feature = "unicode-lines")]
        arg if arg.starts_with("nel") => Nel,
        _ => bail!("invalid line ending"),
    };
    let (view, doc) = current!(cx.editor);
    doc.line_ending = line_ending;

    let mut pos = 0;
    let transaction = Transaction::change(
        doc.text(),
        doc.text().lines().filter_map(|line| {
            pos += line.len_chars();
            match helix_core::line_ending::get_line_ending(&line) {
                Some(ending) if ending != line_ending => {
                    let start = pos - ending.len_chars();
                    let end = pos;
                    Some((start, end, Some(line_ending.as_str().into())))
                }
                _ => None,
            }
        }),
    );
    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view);

    Ok(())
}

fn earlier(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let uk = args.join(" ").parse::<UndoKind>().map_err(|s| anyhow!(s))?;

    let (view, doc) = current!(cx.editor);
    let success = doc.earlier(view, uk);
    if !success {
        cx.editor.set_status("Already at oldest change");
    }

    Ok(())
}

fn later(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let uk = args.join(" ").parse::<UndoKind>().map_err(|s| anyhow!(s))?;
    let (view, doc) = current!(cx.editor);
    let success = doc.later(view, uk);
    if !success {
        cx.editor.set_status("Already at newest change");
    }

    Ok(())
}

fn write_quit(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    write_impl(cx, args.first(), false)?;
    cx.block_try_flush_writes()?;
    quit(cx, &[], event)
}

fn force_write_quit(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    write_impl(cx, args.first(), true)?;
    cx.block_try_flush_writes()?;
    force_quit(cx, &[], event)
}

/// Results in an error if there are modified buffers remaining and sets editor
/// error, otherwise returns `Ok(())`. If the current document is unmodified,
/// and there are modified documents, switches focus to one of them.
pub(super) fn buffers_remaining_impl(editor: &mut Editor) -> anyhow::Result<()> {
    let (modified_ids, modified_names): (Vec<_>, Vec<_>) = editor
        .documents()
        .filter(|doc| doc.is_modified())
        .map(|doc| (doc.id(), doc.display_name()))
        .unzip();
    if let Some(first) = modified_ids.first() {
        let current = doc!(editor);
        // If the current document is unmodified, and there are modified
        // documents, switch focus to the first modified doc.
        if !modified_ids.contains(&current.id()) {
            editor.switch(*first, Action::Replace);
        }
        bail!(
            "{} unsaved buffer(s) remaining: {:?}",
            modified_names.len(),
            modified_names
        );
    }
    Ok(())
}

pub fn write_all_impl(
    cx: &mut compositor::Context,
    force: bool,
    write_scratch: bool,
) -> anyhow::Result<()> {
    let mut errors: Vec<&'static str> = Vec::new();
    let auto_format = cx.editor.config().auto_format;
    let jobs = &mut cx.jobs;
    let current_view = view!(cx.editor);

    // save all documents
    let saves: Vec<_> = cx
        .editor
        .documents
        .values_mut()
        .filter_map(|doc| {
            if !doc.is_modified() {
                return None;
            }
            if doc.path().is_none() {
                if write_scratch {
                    errors.push("cannot write a buffer without a filename\n");
                }
                return None;
            }

            // Look for a view to apply the formatting change to. If the document
            // is in the current view, just use that. Otherwise, since we don't
            // have any other metric available for better selection, just pick
            // the first view arbitrarily so that we still commit the document
            // state for undos. If somehow we have a document that has not been
            // initialized with any view, initialize it with the current view.
            let target_view = if doc.selections().contains_key(&current_view.id) {
                current_view.id
            } else if let Some(view) = doc.selections().keys().next() {
                *view
            } else {
                doc.ensure_view_init(current_view.id);
                current_view.id
            };

            let fmt = if auto_format {
                doc.auto_format().map(|fmt| {
                    let callback = make_format_callback(
                        doc.id(),
                        doc.version(),
                        target_view,
                        fmt,
                        Some((None, force)),
                    );
                    jobs.add(Job::with_callback(callback).wait_before_exiting());
                })
            } else {
                None
            };

            if fmt.is_none() {
                return Some(doc.id());
            }

            None
        })
        .collect();

    // manually call save for the rest of docs that don't have a formatter
    for id in saves {
        cx.editor.save::<PathBuf>(id, None, force)?;
    }

    if !errors.is_empty() && !force {
        bail!("{:?}", errors);
    }

    Ok(())
}

fn write_all(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    write_all_impl(cx, false, true)
}

fn write_all_quit(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }
    write_all_impl(cx, false, true)?;
    quit_all_impl(cx, false)
}

fn force_write_all_quit(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }
    let _ = write_all_impl(cx, true, true);
    quit_all_impl(cx, true)
}

fn quit_all_impl(cx: &mut compositor::Context, force: bool) -> anyhow::Result<()> {
    cx.block_try_flush_writes()?;
    if !force {
        buffers_remaining_impl(cx.editor)?;
    }

    // close all views
    let views: Vec<_> = cx.editor.tree.views().map(|(view, _)| view.id).collect();
    for view_id in views {
        cx.editor.close(view_id);
    }

    Ok(())
}

fn quit_all(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    quit_all_impl(cx, false)
}

fn force_quit_all(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    quit_all_impl(cx, true)
}

fn cquit(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let exit_code = args
        .first()
        .and_then(|code| code.parse::<i32>().ok())
        .unwrap_or(1);

    cx.editor.exit_code = exit_code;
    quit_all_impl(cx, false)
}

fn force_cquit(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let exit_code = args
        .first()
        .and_then(|code| code.parse::<i32>().ok())
        .unwrap_or(1);
    cx.editor.exit_code = exit_code;

    quit_all_impl(cx, true)
}

fn theme(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    let true_color = cx.editor.config.load().true_color || crate::true_color();
    match event {
        PromptEvent::Abort => {
            cx.editor.unset_theme_preview();
        }
        PromptEvent::Update => {
            if args.is_empty() {
                // Ensures that a preview theme gets cleaned up if the user backspaces until the prompt is empty.
                cx.editor.unset_theme_preview();
            } else if let Some(theme_name) = args.first() {
                if let Ok(theme) = cx.editor.theme_loader.load(theme_name) {
                    if !(true_color || theme.is_16_color()) {
                        bail!("Unsupported theme: theme requires true color support");
                    }
                    cx.editor.set_theme_preview(theme);
                };
            };
        }
        PromptEvent::Validate => {
            if let Some(theme_name) = args.first() {
                let theme = cx
                    .editor
                    .theme_loader
                    .load(theme_name)
                    .map_err(|err| anyhow::anyhow!("Could not load theme: {}", err))?;
                if !(true_color || theme.is_16_color()) {
                    bail!("Unsupported theme: theme requires true color support");
                }
                cx.editor.set_theme(theme);
            } else {
                let name = cx.editor.theme.name().to_string();

                cx.editor.set_status(name);
            }
        }
    };

    Ok(())
}

fn yank_main_selection_to_clipboard(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    yank_main_selection_to_clipboard_impl(cx.editor, ClipboardType::Clipboard)
}

fn yank_joined_to_clipboard(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let doc = doc!(cx.editor);
    let default_sep = Cow::Borrowed(doc.line_ending.as_str());
    let separator = args.first().unwrap_or(&default_sep);
    yank_joined_to_clipboard_impl(cx.editor, separator, ClipboardType::Clipboard)
}

fn yank_main_selection_to_primary_clipboard(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    yank_main_selection_to_clipboard_impl(cx.editor, ClipboardType::Selection)
}

fn yank_joined_to_primary_clipboard(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let doc = doc!(cx.editor);
    let default_sep = Cow::Borrowed(doc.line_ending.as_str());
    let separator = args.first().unwrap_or(&default_sep);
    yank_joined_to_clipboard_impl(cx.editor, separator, ClipboardType::Selection)
}

fn paste_clipboard_after(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    paste_clipboard_impl(cx.editor, Paste::After, ClipboardType::Clipboard, 1)
}

fn paste_clipboard_before(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    paste_clipboard_impl(cx.editor, Paste::Before, ClipboardType::Clipboard, 1)
}

fn paste_primary_clipboard_after(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    paste_clipboard_impl(cx.editor, Paste::After, ClipboardType::Selection, 1)
}

fn paste_primary_clipboard_before(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    paste_clipboard_impl(cx.editor, Paste::Before, ClipboardType::Selection, 1)
}

fn replace_selections_with_clipboard_impl(
    cx: &mut compositor::Context,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let scrolloff = cx.editor.config().scrolloff;
    let (view, doc) = current!(cx.editor);

    match cx.editor.clipboard_provider.get_contents(clipboard_type) {
        Ok(contents) => {
            let selection = doc.selection(view.id);
            let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
                (range.from(), range.to(), Some(contents.as_str().into()))
            });

            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view);
            view.ensure_cursor_in_view(doc, scrolloff);
            Ok(())
        }
        Err(e) => Err(e.context("Couldn't get system clipboard contents")),
    }
}

fn replace_selections_with_clipboard(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    replace_selections_with_clipboard_impl(cx, ClipboardType::Clipboard)
}

fn replace_selections_with_primary_clipboard(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    replace_selections_with_clipboard_impl(cx, ClipboardType::Selection)
}

fn show_clipboard_provider(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    cx.editor
        .set_status(cx.editor.clipboard_provider.name().to_string());
    Ok(())
}

fn change_current_directory(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let dir = helix_core::path::expand_tilde(
        args.first()
            .context("target directory not provided")?
            .as_ref()
            .as_ref(),
    );

    if let Err(e) = std::env::set_current_dir(dir) {
        bail!("Couldn't change the current working directory: {}", e);
    }

    let cwd = std::env::current_dir().context("Couldn't get the new working directory")?;
    cx.editor.set_status(format!(
        "Current working directory is now {}",
        cwd.display()
    ));
    Ok(())
}

fn show_current_directory(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let cwd = std::env::current_dir().context("Couldn't get the new working directory")?;
    cx.editor
        .set_status(format!("Current working directory is {}", cwd.display()));
    Ok(())
}

/// Sets the [`Document`]'s encoding..
fn set_encoding(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let doc = doc_mut!(cx.editor);
    if let Some(label) = args.first() {
        doc.set_encoding(label)
    } else {
        let encoding = doc.encoding().name().to_owned();
        cx.editor.set_status(encoding);
        Ok(())
    }
}

/// Shows info about the character under the primary cursor.
fn get_character_info(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (view, doc) = current_ref!(cx.editor);
    let text = doc.text().slice(..);

    let grapheme_start = doc.selection(view.id).primary().cursor(text);
    let grapheme_end = graphemes::next_grapheme_boundary(text, grapheme_start);

    if grapheme_start == grapheme_end {
        return Ok(());
    }

    let grapheme = text.slice(grapheme_start..grapheme_end).to_string();
    let encoding = doc.encoding();

    let printable = grapheme.chars().fold(String::new(), |mut s, c| {
        match c {
            '\0' => s.push_str("\\0"),
            '\t' => s.push_str("\\t"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            _ => s.push(c),
        }

        s
    });

    // Convert to Unicode codepoints if in UTF-8
    let unicode = if encoding == encoding::UTF_8 {
        let mut unicode = " (".to_owned();

        for (i, char) in grapheme.chars().enumerate() {
            if i != 0 {
                unicode.push(' ');
            }

            unicode.push_str("U+");

            let codepoint: u32 = if char.is_ascii() {
                char.into()
            } else {
                // Not ascii means it will be multi-byte, so strip out the extra
                // bits that encode the length & mark continuation bytes

                let s = String::from(char);
                let bytes = s.as_bytes();

                // First byte starts with 2-4 ones then a zero, so strip those off
                let first = bytes[0];
                let codepoint = first & (0xFF >> (first.leading_ones() + 1));
                let mut codepoint = u32::from(codepoint);

                // Following bytes start with 10
                for byte in bytes.iter().skip(1) {
                    codepoint <<= 6;
                    codepoint += u32::from(*byte) & 0x3F;
                }

                codepoint
            };

            write!(unicode, "{codepoint:0>4x}").unwrap();
        }

        unicode.push(')');
        unicode
    } else {
        String::new()
    };

    // Give the decimal value for ascii characters
    let dec = if encoding.is_ascii_compatible() && grapheme.len() == 1 {
        format!(" Dec {}", grapheme.as_bytes()[0])
    } else {
        String::new()
    };

    let hex = {
        let mut encoder = encoding.new_encoder();
        let max_encoded_len = encoder
            .max_buffer_length_from_utf8_without_replacement(grapheme.len())
            .unwrap();
        let mut bytes = Vec::with_capacity(max_encoded_len);
        let mut current_byte = 0;
        let mut hex = String::new();

        for (i, char) in grapheme.chars().enumerate() {
            if i != 0 {
                hex.push_str(" +");
            }

            let (result, _input_bytes_read) = encoder.encode_from_utf8_to_vec_without_replacement(
                &char.to_string(),
                &mut bytes,
                true,
            );

            if let encoding::EncoderResult::Unmappable(char) = result {
                bail!("{char:?} cannot be mapped to {}", encoding.name());
            }

            for byte in &bytes[current_byte..] {
                write!(hex, " {byte:0>2x}").unwrap();
            }

            current_byte = bytes.len();
        }

        hex
    };

    cx.editor
        .set_status(format!("\"{printable}\"{unicode}{dec} Hex{hex}"));

    Ok(())
}

/// Reload the [`Document`] from its source file.
fn reload(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let scrolloff = cx.editor.config().scrolloff;
    let redraw_handle = cx.editor.redraw_handle.clone();
    let (view, doc) = current!(cx.editor);
    doc.reload(view, &cx.editor.diff_providers, redraw_handle)
        .map(|_| {
            view.ensure_cursor_in_view(doc, scrolloff);
        })
}

fn reload_all(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let scrolloff = cx.editor.config().scrolloff;
    let view_id = view!(cx.editor).id;

    let docs_view_ids: Vec<(DocumentId, Vec<ViewId>)> = cx
        .editor
        .documents_mut()
        .map(|doc| {
            let mut view_ids: Vec<_> = doc.selections().keys().cloned().collect();

            if view_ids.is_empty() {
                doc.ensure_view_init(view_id);
                view_ids.push(view_id);
            };

            (doc.id(), view_ids)
        })
        .collect();

    for (doc_id, view_ids) in docs_view_ids {
        let doc = doc_mut!(cx.editor, &doc_id);

        // Every doc is guaranteed to have at least 1 view at this point.
        let view = view_mut!(cx.editor, view_ids[0]);

        // Ensure that the view is synced with the document's history.
        view.sync_changes(doc);

        let redraw_handle = cx.editor.redraw_handle.clone();
        doc.reload(view, &cx.editor.diff_providers, redraw_handle)?;

        for view_id in view_ids {
            let view = view_mut!(cx.editor, view_id);
            if view.doc.eq(&doc_id) {
                view.ensure_cursor_in_view(doc, scrolloff);
            }
        }
    }

    Ok(())
}

/// Update the [`Document`] if it has been modified.
fn update(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (_view, doc) = current!(cx.editor);
    if doc.is_modified() {
        write(cx, args, event)
    } else {
        Ok(())
    }
}

fn lsp_workspace_command(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (_, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => {
            cx.editor
                .set_status("Language server not active for current buffer");
            return Ok(());
        }
    };

    let options = match &language_server.capabilities().execute_command_provider {
        Some(options) => options,
        None => {
            cx.editor
                .set_status("Workspace commands are not supported for this language server");
            return Ok(());
        }
    };
    if args.is_empty() {
        let commands = options
            .commands
            .iter()
            .map(|command| helix_lsp::lsp::Command {
                title: command.clone(),
                command: command.clone(),
                arguments: None,
            })
            .collect::<Vec<_>>();
        let callback = async move {
            let call: job::Callback = Callback::EditorCompositor(Box::new(
                move |_editor: &mut Editor, compositor: &mut Compositor| {
                    let picker = ui::Picker::new(commands, (), |cx, command, _action| {
                        execute_lsp_command(cx.editor, command.clone());
                    });
                    compositor.push(Box::new(overlaid(picker)))
                },
            ));
            Ok(call)
        };
        cx.jobs.callback(callback);
    } else {
        let command = args.join(" ");
        if options.commands.iter().any(|c| c == &command) {
            execute_lsp_command(
                cx.editor,
                helix_lsp::lsp::Command {
                    title: command.clone(),
                    arguments: None,
                    command,
                },
            );
        } else {
            cx.editor.set_status(format!(
                "`{command}` is not supported for this language server"
            ));
            return Ok(());
        }
    }
    Ok(())
}

fn lsp_restart(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let editor_config = cx.editor.config.load();
    let (_view, doc) = current!(cx.editor);
    let config = doc
        .language_config()
        .context("LSP not defined for the current document")?;

    let scope = config.scope.clone();
    cx.editor.language_servers.restart(
        config,
        doc.path(),
        &editor_config.workspace_lsp_roots,
        editor_config.lsp.snippets,
    )?;

    // This collect is needed because refresh_language_server would need to re-borrow editor.
    let document_ids_to_refresh: Vec<DocumentId> = cx
        .editor
        .documents()
        .filter_map(|doc| match doc.language_config() {
            Some(config) if config.scope.eq(&scope) => Some(doc.id()),
            _ => None,
        })
        .collect();

    for document_id in document_ids_to_refresh {
        cx.editor.refresh_language_server(document_id);
    }

    Ok(())
}

fn lsp_stop(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let doc = doc!(cx.editor);

    let ls_id = doc
        .language_server()
        .map(|ls| ls.id())
        .context("LSP not running for the current document")?;

    let config = doc
        .language_config()
        .context("LSP not defined for the current document")?;
    cx.editor.language_servers.stop(config);

    for doc in cx.editor.documents_mut() {
        if doc.language_server().map_or(false, |ls| ls.id() == ls_id) {
            doc.set_language_server(None);
            doc.set_diagnostics(Default::default());
        }
    }

    Ok(())
}

fn tree_sitter_scopes(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let pos = doc.selection(view.id).primary().cursor(text);
    let scopes = indent::get_scopes(doc.syntax(), text, pos);

    let contents = format!("```json\n{:?}\n````", scopes);

    let callback = async move {
        let call: job::Callback = Callback::EditorCompositor(Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor| {
                let contents = ui::Markdown::new(contents, editor.syn_loader.clone());
                let popup = Popup::new("hover", contents).auto_close(true);
                compositor.replace_or_push("hover", popup);
            },
        ));
        Ok(call)
    };

    cx.jobs.callback(callback);

    Ok(())
}

fn vsplit(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let id = view!(cx.editor).doc;

    if args.is_empty() {
        cx.editor.switch(id, Action::VerticalSplit);
    } else {
        for arg in args {
            cx.editor
                .open(&PathBuf::from(arg.as_ref()), Action::VerticalSplit)?;
        }
    }

    Ok(())
}

fn hsplit(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let id = view!(cx.editor).doc;

    if args.is_empty() {
        cx.editor.switch(id, Action::HorizontalSplit);
    } else {
        for arg in args {
            cx.editor
                .open(&PathBuf::from(arg.as_ref()), Action::HorizontalSplit)?;
        }
    }

    Ok(())
}

fn vsplit_new(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    cx.editor.new_file(Action::VerticalSplit);

    Ok(())
}

fn hsplit_new(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    cx.editor.new_file(Action::HorizontalSplit);

    Ok(())
}

fn debug_eval(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    if let Some(debugger) = cx.editor.debugger.as_mut() {
        let (frame, thread_id) = match (debugger.active_frame, debugger.thread_id) {
            (Some(frame), Some(thread_id)) => (frame, thread_id),
            _ => {
                bail!("Cannot find current stack frame to access variables")
            }
        };

        // TODO: support no frame_id

        let frame_id = debugger.stack_frames[&thread_id][frame].id;
        let response = helix_lsp::block_on(debugger.eval(args.join(" "), Some(frame_id)))?;
        cx.editor.set_status(response.result);
    }
    Ok(())
}

fn debug_start(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let mut args = args.to_owned();
    let name = match args.len() {
        0 => None,
        _ => Some(args.remove(0)),
    };
    dap_start_impl(cx, name.as_deref(), None, Some(args))
}

fn debug_remote(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let mut args = args.to_owned();
    let address = match args.len() {
        0 => None,
        _ => Some(args.remove(0).parse()?),
    };
    let name = match args.len() {
        0 => None,
        _ => Some(args.remove(0)),
    };
    dap_start_impl(cx, name.as_deref(), address, Some(args))
}

fn tutor(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let path = helix_loader::runtime_file(Path::new("tutor"));
    cx.editor.open(&path, Action::Replace)?;
    // Unset path to prevent accidentally saving to the original tutor file.
    doc_mut!(cx.editor).set_path(None)?;
    Ok(())
}

fn abort_goto_line_number_preview(cx: &mut compositor::Context) {
    if let Some(last_selection) = cx.editor.last_selection.take() {
        let scrolloff = cx.editor.config().scrolloff;

        let (view, doc) = current!(cx.editor);
        doc.set_selection(view.id, last_selection);
        view.ensure_cursor_in_view(doc, scrolloff);
    }
}

fn update_goto_line_number_preview(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
) -> anyhow::Result<()> {
    cx.editor.last_selection.get_or_insert_with(|| {
        let (view, doc) = current!(cx.editor);
        doc.selection(view.id).clone()
    });

    let scrolloff = cx.editor.config().scrolloff;
    let line = args[0].parse::<usize>()?;
    goto_line_without_jumplist(cx.editor, NonZeroUsize::new(line));

    let (view, doc) = current!(cx.editor);
    view.ensure_cursor_in_view(doc, scrolloff);

    Ok(())
}

pub(super) fn goto_line_number(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    match event {
        PromptEvent::Abort => abort_goto_line_number_preview(cx),
        PromptEvent::Validate => {
            ensure!(!args.is_empty(), "Line number required");

            // If we are invoked directly via a keybinding, Validate is
            // sent without any prior Update events. Ensure the cursor
            // is moved to the appropriate location.
            update_goto_line_number_preview(cx, args)?;

            let last_selection = cx
                .editor
                .last_selection
                .take()
                .expect("update_goto_line_number_preview should always set last_selection");

            let (view, doc) = current!(cx.editor);
            view.jumps.push((doc.id(), last_selection));
        }

        // When a user hits backspace and there are no numbers left,
        // we can bring them back to their original selection. If they
        // begin typing numbers again, we'll start a new preview session.
        PromptEvent::Update if args.is_empty() => abort_goto_line_number_preview(cx),
        PromptEvent::Update => update_goto_line_number_preview(cx, args)?,
    }

    Ok(())
}

// Fetch the current value of a config option and output as status.
fn get_option(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    if args.len() != 1 {
        anyhow::bail!("Bad arguments. Usage: `:get key`");
    }

    let key = &args[0].to_lowercase();
    let key_error = || anyhow::anyhow!("Unknown key `{}`", key);

    let config = serde_json::json!(cx.editor.config().deref());
    let pointer = format!("/{}", key.replace('.', "/"));
    let value = config.pointer(&pointer).ok_or_else(key_error)?;

    cx.editor.set_status(value.to_string());
    Ok(())
}

/// Change config at runtime. Access nested values by dot syntax, for
/// example to disable smart case search, use `:set search.smart-case false`.
fn set_option(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    if args.len() != 2 {
        anyhow::bail!("Bad arguments. Usage: `:set key field`");
    }
    let (key, arg) = (&args[0].to_lowercase(), &args[1]);

    let key_error = || anyhow::anyhow!("Unknown key `{}`", key);
    let field_error = |_| anyhow::anyhow!("Could not parse field `{}`", arg);

    let mut config = serde_json::json!(&cx.editor.config().deref());
    let pointer = format!("/{}", key.replace('.', "/"));
    let value = config.pointer_mut(&pointer).ok_or_else(key_error)?;

    *value = if value.is_string() {
        // JSON strings require quotes, so we can't .parse() directly
        serde_json::Value::String(arg.to_string())
    } else {
        arg.parse().map_err(field_error)?
    };
    let config = serde_json::from_value(config).map_err(field_error)?;

    cx.editor
        .config_events
        .0
        .send(ConfigEvent::Update(config))?;
    Ok(())
}

/// Toggle boolean config option at runtime. Access nested values by dot
/// syntax, for example to toggle smart case search, use `:toggle search.smart-
/// case`.
fn toggle_option(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    if args.len() != 1 {
        anyhow::bail!("Bad arguments. Usage: `:toggle key`");
    }
    let key = &args[0].to_lowercase();

    let key_error = || anyhow::anyhow!("Unknown key `{}`", key);

    let mut config = serde_json::json!(&cx.editor.config().deref());
    let pointer = format!("/{}", key.replace('.', "/"));
    let value = config.pointer_mut(&pointer).ok_or_else(key_error)?;

    let Value::Bool(old_value) = *value else {
        anyhow::bail!("Key `{}` is not toggle-able", key)
    };

    let new_value = !old_value;
    *value = Value::Bool(new_value);
    // This unwrap should never fail because we only replace one boolean value
    // with another, maintaining a valid json config
    let config = serde_json::from_value(config).unwrap();

    cx.editor
        .config_events
        .0
        .send(ConfigEvent::Update(config))?;
    cx.editor
        .set_status(format!("Option `{}` is now set to `{}`", key, new_value));
    Ok(())
}

/// Change the language of the current buffer at runtime.
fn language(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    if args.is_empty() {
        let doc = doc!(cx.editor);
        let language = &doc.language_name().unwrap_or(DEFAULT_LANGUAGE_NAME);
        cx.editor.set_status(language.to_string());
        return Ok(());
    }

    if args.len() != 1 {
        anyhow::bail!("Bad arguments. Usage: `:set-language language`");
    }

    let doc = doc_mut!(cx.editor);

    if args[0] == DEFAULT_LANGUAGE_NAME {
        doc.set_language(None, None)
    } else {
        doc.set_language_by_language_id(&args[0], cx.editor.syn_loader.clone())?;
    }
    doc.detect_indent_and_line_ending();

    let id = doc.id();
    cx.editor.refresh_language_server(id);
    Ok(())
}

fn sort(cx: &mut compositor::Context, args: &[Cow<str>], event: PromptEvent) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    sort_impl(cx, args, false)
}

fn sort_reverse(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    sort_impl(cx, args, true)
}

fn sort_impl(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    reverse: bool,
) -> anyhow::Result<()> {
    let scrolloff = cx.editor.config().scrolloff;
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id);

    let mut fragments: Vec<_> = selection
        .slices(text)
        .map(|fragment| fragment.chunks().collect())
        .collect();

    fragments.sort_by(match reverse {
        true => |a: &Tendril, b: &Tendril| b.cmp(a),
        false => |a: &Tendril, b: &Tendril| a.cmp(b),
    });

    let transaction = Transaction::change(
        doc.text(),
        selection
            .into_iter()
            .zip(fragments)
            .map(|(s, fragment)| (s.from(), s.to(), Some(fragment))),
    );

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view);
    view.ensure_cursor_in_view(doc, scrolloff);

    Ok(())
}

fn reflow(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let scrolloff = cx.editor.config().scrolloff;
    let cfg_text_width: usize = cx.editor.config().text_width;
    let (view, doc) = current!(cx.editor);

    // Find the text_width by checking the following sources in order:
    //   - The passed argument in `args`
    //   - The configured text-width for this language in languages.toml
    //   - The configured text-width in the config.toml
    let text_width: usize = args
        .get(0)
        .map(|num| num.parse::<usize>())
        .transpose()?
        .or_else(|| doc.language_config().and_then(|config| config.text_width))
        .unwrap_or(cfg_text_width);

    let rope = doc.text();

    let selection = doc.selection(view.id);
    let transaction = Transaction::change_by_selection(rope, selection, |range| {
        let fragment = range.fragment(rope.slice(..));
        let reflowed_text = helix_core::wrap::reflow_hard_wrap(&fragment, text_width);

        (range.from(), range.to(), Some(reflowed_text))
    });

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view);
    view.ensure_cursor_in_view(doc, scrolloff);

    Ok(())
}

fn tree_sitter_subtree(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (view, doc) = current!(cx.editor);

    if let Some(syntax) = doc.syntax() {
        let primary_selection = doc.selection(view.id).primary();
        let text = doc.text();
        let from = text.char_to_byte(primary_selection.from());
        let to = text.char_to_byte(primary_selection.to());
        if let Some(selected_node) = syntax
            .tree()
            .root_node()
            .descendant_for_byte_range(from, to)
        {
            let mut contents = String::from("```tsq\n");
            helix_core::syntax::pretty_print_tree(&mut contents, selected_node)?;
            contents.push_str("\n```");

            let callback = async move {
                let call: job::Callback = Callback::EditorCompositor(Box::new(
                    move |editor: &mut Editor, compositor: &mut Compositor| {
                        let contents = ui::Markdown::new(contents, editor.syn_loader.clone());
                        let popup = Popup::new("hover", contents).auto_close(true);
                        compositor.replace_or_push("hover", popup);
                    },
                ));
                Ok(call)
            };

            cx.jobs.callback(callback);
        }
    }

    Ok(())
}

fn open_config(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    cx.editor
        .open(&helix_loader::config_file(), Action::Replace)?;
    Ok(())
}

fn open_workspace_config(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    cx.editor
        .open(&helix_loader::workspace_config_file(), Action::Replace)?;
    Ok(())
}

fn open_log(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    cx.editor.open(&helix_loader::log_file(), Action::Replace)?;
    Ok(())
}

fn refresh_config(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    cx.editor.config_events.0.send(ConfigEvent::Refresh)?;
    Ok(())
}

fn append_output(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    ensure!(!args.is_empty(), "Shell command required");
    shell(cx, &args.join(" "), &ShellBehavior::Append);
    Ok(())
}

fn insert_output(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    ensure!(!args.is_empty(), "Shell command required");
    shell(cx, &args.join(" "), &ShellBehavior::Insert);
    Ok(())
}

fn pipe_to(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    pipe_impl(cx, args, event, &ShellBehavior::Ignore)
}

fn pipe(cx: &mut compositor::Context, args: &[Cow<str>], event: PromptEvent) -> anyhow::Result<()> {
    pipe_impl(cx, args, event, &ShellBehavior::Replace)
}

fn pipe_impl(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
    behavior: &ShellBehavior,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    ensure!(!args.is_empty(), "Shell command required");
    shell(cx, &args.join(" "), behavior);
    Ok(())
}

fn run_shell_command(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let shell = cx.editor.config().shell.clone();
    let args = args.join(" ");

    let callback = async move {
        let (output, success) = shell_impl_async(&shell, &args, None).await?;
        let call: job::Callback = Callback::EditorCompositor(Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor| {
                if !output.is_empty() {
                    let contents = ui::Markdown::new(
                        format!("```sh\n{}\n```", output),
                        editor.syn_loader.clone(),
                    );
                    let popup = Popup::new("shell", contents).position(Some(
                        helix_core::Position::new(editor.cursor().0.unwrap_or_default().row, 2),
                    ));
                    compositor.replace_or_push("shell", popup);
                }
                if success {
                    editor.set_status("Command succeeded");
                } else {
                    editor.set_error("Command failed");
                }
            },
        ));
        Ok(call)
    };
    cx.jobs.callback(callback);

    Ok(())
}

fn reset_diff_change(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }
    ensure!(args.is_empty(), ":reset-diff-change takes no arguments");

    let editor = &mut cx.editor;
    let scrolloff = editor.config().scrolloff;

    let (view, doc) = current!(editor);
    let Some(handle) = doc.diff_handle() else {
        bail!("Diff is not available in the current buffer")
    };

    let diff = handle.load();
    let doc_text = doc.text().slice(..);
    let line = doc.selection(view.id).primary().cursor_line(doc_text);

    let Some(hunk_idx) = diff.hunk_at(line as u32, true) else {
        bail!("There is no change at the cursor")
    };
    let hunk = diff.nth_hunk(hunk_idx);
    let diff_base = diff.diff_base();
    let before_start = diff_base.line_to_char(hunk.before.start as usize);
    let before_end = diff_base.line_to_char(hunk.before.end as usize);
    let text: Tendril = diff
        .diff_base()
        .slice(before_start..before_end)
        .chunks()
        .collect();
    let anchor = doc_text.line_to_char(hunk.after.start as usize);
    let transaction = Transaction::change(
        doc.text(),
        [(
            anchor,
            doc_text.line_to_char(hunk.after.end as usize),
            (!text.is_empty()).then_some(text),
        )]
        .into_iter(),
    );
    drop(diff); // make borrow check happy
    doc.apply(&transaction, view.id);
    // select inserted text
    let text_len = before_end - before_start;
    doc.set_selection(view.id, Selection::single(anchor, anchor + text_len));
    doc.append_changes_to_history(view);
    view.ensure_cursor_in_view(doc, scrolloff);
    Ok(())
}

fn clear_register(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    ensure!(args.len() <= 1, ":clear-register takes at most 1 argument");
    if args.is_empty() {
        cx.editor.registers.clear();
        cx.editor.set_status("All registers cleared");
        return Ok(());
    }

    ensure!(
        args[0].chars().count() == 1,
        format!("Invalid register {}", args[0])
    );
    let register = args[0].chars().next().unwrap_or_default();
    match cx.editor.registers.remove(register) {
        Some(_) => cx
            .editor
            .set_status(format!("Register {} cleared", register)),
        None => cx
            .editor
            .set_error(format!("Register {} not found", register)),
    }
    Ok(())
}

pub const TYPABLE_COMMAND_LIST: &[TypableCommand] = &[
        TypableCommand {
            name: "quit",
            aliases: &["q"],
            doc: "Close the current view.",
            fun: quit,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "quit!",
            aliases: &["q!"],
            doc: "Force close the current view, ignoring unsaved changes.",
            fun: force_quit,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "open",
            aliases: &["o"],
            doc: "Open a file from disk into the current view.",
            fun: open,
            signature: CommandSignature::all(completers::filename),
        },
        TypableCommand {
            name: "buffer-close",
            aliases: &["bc", "bclose"],
            doc: "Close the current buffer.",
            fun: buffer_close,
            signature: CommandSignature::all(completers::buffer),
        },
        TypableCommand {
            name: "buffer-close!",
            aliases: &["bc!", "bclose!"],
            doc: "Close the current buffer forcefully, ignoring unsaved changes.",
            fun: force_buffer_close,
            signature: CommandSignature::all(completers::buffer)
        },
        TypableCommand {
            name: "buffer-close-others",
            aliases: &["bco", "bcloseother"],
            doc: "Close all buffers but the currently focused one.",
            fun: buffer_close_others,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "buffer-close-others!",
            aliases: &["bco!", "bcloseother!"],
            doc: "Force close all buffers but the currently focused one.",
            fun: force_buffer_close_others,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "buffer-close-all",
            aliases: &["bca", "bcloseall"],
            doc: "Close all buffers without quitting.",
            fun: buffer_close_all,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "buffer-close-all!",
            aliases: &["bca!", "bcloseall!"],
            doc: "Force close all buffers ignoring unsaved changes without quitting.",
            fun: force_buffer_close_all,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "buffer-next",
            aliases: &["bn", "bnext"],
            doc: "Goto next buffer.",
            fun: buffer_next,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "buffer-previous",
            aliases: &["bp", "bprev"],
            doc: "Goto previous buffer.",
            fun: buffer_previous,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "write",
            aliases: &["w"],
            doc: "Write changes to disk. Accepts an optional path (:write some/path.txt)",
            fun: write,
            signature: CommandSignature::positional(&[completers::filename]),
        },
        TypableCommand {
            name: "write!",
            aliases: &["w!"],
            doc: "Force write changes to disk creating necessary subdirectories. Accepts an optional path (:write! some/path.txt)",
            fun: force_write,
            signature: CommandSignature::positional(&[completers::filename]),
        },
        TypableCommand {
            name: "write-buffer-close",
            aliases: &["wbc"],
            doc: "Write changes to disk and closes the buffer. Accepts an optional path (:write-buffer-close some/path.txt)",
            fun: write_buffer_close,
            signature: CommandSignature::positional(&[completers::filename]),
        },
        TypableCommand {
            name: "write-buffer-close!",
            aliases: &["wbc!"],
            doc: "Force write changes to disk creating necessary subdirectories and closes the buffer. Accepts an optional path (:write-buffer-close! some/path.txt)",
            fun: force_write_buffer_close,
            signature: CommandSignature::positional(&[completers::filename]),
        },
        TypableCommand {
            name: "new",
            aliases: &["n"],
            doc: "Create a new scratch buffer.",
            fun: new_file,
            // TODO: This seems to complete with a filename, but doesn't use that filename to
            //       set the path of the newly created buffer.
            signature: CommandSignature::positional(&[completers::filename]),
        },
        TypableCommand {
            name: "format",
            aliases: &["fmt"],
            doc: "Format the file using the LSP formatter.",
            fun: format,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "indent-style",
            aliases: &[],
            doc: "Set the indentation style for editing. ('t' for tabs or 1-8 for number of spaces.)",
            fun: set_indent_style,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "line-ending",
            aliases: &[],
            #[cfg(not(feature = "unicode-lines"))]
            doc: "Set the document's default line ending. Options: crlf, lf.",
            #[cfg(feature = "unicode-lines")]
            doc: "Set the document's default line ending. Options: crlf, lf, cr, ff, nel.",
            fun: set_line_ending,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "earlier",
            aliases: &["ear"],
            doc: "Jump back to an earlier point in edit history. Accepts a number of steps or a time span.",
            fun: earlier,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "later",
            aliases: &["lat"],
            doc: "Jump to a later point in edit history. Accepts a number of steps or a time span.",
            fun: later,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "write-quit",
            aliases: &["wq", "x"],
            doc: "Write changes to disk and close the current view. Accepts an optional path (:wq some/path.txt)",
            fun: write_quit,
            signature: CommandSignature::positional(&[completers::filename]),
        },
        TypableCommand {
            name: "write-quit!",
            aliases: &["wq!", "x!"],
            doc: "Write changes to disk and close the current view forcefully. Accepts an optional path (:wq! some/path.txt)",
            fun: force_write_quit,
            signature: CommandSignature::positional(&[completers::filename]),
        },
        TypableCommand {
            name: "write-all",
            aliases: &["wa"],
            doc: "Write changes from all buffers to disk.",
            fun: write_all,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "write-quit-all",
            aliases: &["wqa", "xa"],
            doc: "Write changes from all buffers to disk and close all views.",
            fun: write_all_quit,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "write-quit-all!",
            aliases: &["wqa!", "xa!"],
            doc: "Write changes from all buffers to disk and close all views forcefully (ignoring unsaved changes).",
            fun: force_write_all_quit,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "quit-all",
            aliases: &["qa"],
            doc: "Close all views.",
            fun: quit_all,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "quit-all!",
            aliases: &["qa!"],
            doc: "Force close all views ignoring unsaved changes.",
            fun: force_quit_all,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "cquit",
            aliases: &["cq"],
            doc: "Quit with exit code (default 1). Accepts an optional integer exit code (:cq 2).",
            fun: cquit,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "cquit!",
            aliases: &["cq!"],
            doc: "Force quit with exit code (default 1) ignoring unsaved changes. Accepts an optional integer exit code (:cq! 2).",
            fun: force_cquit,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "theme",
            aliases: &[],
            doc: "Change the editor theme (show current theme if no name specified).",
            fun: theme,
            signature: CommandSignature::positional(&[completers::theme]),
        },
        TypableCommand {
            name: "clipboard-yank",
            aliases: &[],
            doc: "Yank main selection into system clipboard.",
            fun: yank_main_selection_to_clipboard,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "clipboard-yank-join",
            aliases: &[],
            doc: "Yank joined selections into system clipboard. A separator can be provided as first argument. Default value is newline.", // FIXME: current UI can't display long doc.
            fun: yank_joined_to_clipboard,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "primary-clipboard-yank",
            aliases: &[],
            doc: "Yank main selection into system primary clipboard.",
            fun: yank_main_selection_to_primary_clipboard,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "primary-clipboard-yank-join",
            aliases: &[],
            doc: "Yank joined selections into system primary clipboard. A separator can be provided as first argument. Default value is newline.", // FIXME: current UI can't display long doc.
            fun: yank_joined_to_primary_clipboard,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "clipboard-paste-after",
            aliases: &[],
            doc: "Paste system clipboard after selections.",
            fun: paste_clipboard_after,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "clipboard-paste-before",
            aliases: &[],
            doc: "Paste system clipboard before selections.",
            fun: paste_clipboard_before,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "clipboard-paste-replace",
            aliases: &[],
            doc: "Replace selections with content of system clipboard.",
            fun: replace_selections_with_clipboard,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "primary-clipboard-paste-after",
            aliases: &[],
            doc: "Paste primary clipboard after selections.",
            fun: paste_primary_clipboard_after,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "primary-clipboard-paste-before",
            aliases: &[],
            doc: "Paste primary clipboard before selections.",
            fun: paste_primary_clipboard_before,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "primary-clipboard-paste-replace",
            aliases: &[],
            doc: "Replace selections with content of system primary clipboard.",
            fun: replace_selections_with_primary_clipboard,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "show-clipboard-provider",
            aliases: &[],
            doc: "Show clipboard provider name in status bar.",
            fun: show_clipboard_provider,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "change-current-directory",
            aliases: &["cd"],
            doc: "Change the current working directory.",
            fun: change_current_directory,
            signature: CommandSignature::positional(&[completers::directory]),
        },
        TypableCommand {
            name: "show-directory",
            aliases: &["pwd"],
            doc: "Show the current working directory.",
            fun: show_current_directory,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "encoding",
            aliases: &[],
            doc: "Set encoding. Based on `https://encoding.spec.whatwg.org`.",
            fun: set_encoding,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "character-info",
            aliases: &["char"],
            doc: "Get info about the character under the primary cursor.",
            fun: get_character_info,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "reload",
            aliases: &[],
            doc: "Discard changes and reload from the source file.",
            fun: reload,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "reload-all",
            aliases: &[],
            doc: "Discard changes and reload all documents from the source files.",
            fun: reload_all,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "update",
            aliases: &["u"],
            doc: "Write changes only if the file has been modified.",
            fun: update,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "lsp-workspace-command",
            aliases: &[],
            doc: "Open workspace command picker",
            fun: lsp_workspace_command,
            signature: CommandSignature::positional(&[completers::lsp_workspace_command]),
        },
        TypableCommand {
            name: "lsp-restart",
            aliases: &[],
            doc: "Restarts the Language Server that is in use by the current doc",
            fun: lsp_restart,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "lsp-stop",
            aliases: &[],
            doc: "Stops the Language Server that is in use by the current doc",
            fun: lsp_stop,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "tree-sitter-scopes",
            aliases: &[],
            doc: "Display tree sitter scopes, primarily for theming and development.",
            fun: tree_sitter_scopes,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "debug-start",
            aliases: &["dbg"],
            doc: "Start a debug session from a given template with given parameters.",
            fun: debug_start,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "debug-remote",
            aliases: &["dbg-tcp"],
            doc: "Connect to a debug adapter by TCP address and start a debugging session from a given template with given parameters.",
            fun: debug_remote,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "debug-eval",
            aliases: &[],
            doc: "Evaluate expression in current debug context.",
            fun: debug_eval,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "vsplit",
            aliases: &["vs"],
            doc: "Open the file in a vertical split.",
            fun: vsplit,
            signature: CommandSignature::all(completers::filename)
        },
        TypableCommand {
            name: "vsplit-new",
            aliases: &["vnew"],
            doc: "Open a scratch buffer in a vertical split.",
            fun: vsplit_new,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "hsplit",
            aliases: &["hs", "sp"],
            doc: "Open the file in a horizontal split.",
            fun: hsplit,
            signature: CommandSignature::all(completers::filename)
        },
        TypableCommand {
            name: "hsplit-new",
            aliases: &["hnew"],
            doc: "Open a scratch buffer in a horizontal split.",
            fun: hsplit_new,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "tutor",
            aliases: &[],
            doc: "Open the tutorial.",
            fun: tutor,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "goto",
            aliases: &["g"],
            doc: "Goto line number.",
            fun: goto_line_number,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "set-language",
            aliases: &["lang"],
            doc: "Set the language of current buffer (show current language if no value specified).",
            fun: language,
            signature: CommandSignature::positional(&[completers::language]),
        },
        TypableCommand {
            name: "set-option",
            aliases: &["set"],
            doc: "Set a config option at runtime.\nFor example to disable smart case search, use `:set search.smart-case false`.",
            fun: set_option,
            // TODO: Add support for completion of the options value(s), when appropriate.
            signature: CommandSignature::positional(&[completers::setting]),
        },
        TypableCommand {
            name: "toggle-option",
            aliases: &["toggle"],
            doc: "Toggle a boolean config option at runtime.\nFor example to toggle smart case search, use `:toggle search.smart-case`.",
            fun: toggle_option,
            signature: CommandSignature::positional(&[completers::setting]),
        },
        TypableCommand {
            name: "get-option",
            aliases: &["get"],
            doc: "Get the current value of a config option.",
            fun: get_option,
            signature: CommandSignature::positional(&[completers::setting]),
        },
        TypableCommand {
            name: "sort",
            aliases: &[],
            doc: "Sort ranges in selection.",
            fun: sort,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "rsort",
            aliases: &[],
            doc: "Sort ranges in selection in reverse order.",
            fun: sort_reverse,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "reflow",
            aliases: &[],
            doc: "Hard-wrap the current selection of lines to a given width.",
            fun: reflow,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "tree-sitter-subtree",
            aliases: &["ts-subtree"],
            doc: "Display tree sitter subtree under cursor, primarily for debugging queries.",
            fun: tree_sitter_subtree,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "config-reload",
            aliases: &[],
            doc: "Refresh user config.",
            fun: refresh_config,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "config-open",
            aliases: &[],
            doc: "Open the user config.toml file.",
            fun: open_config,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "config-open-workspace",
            aliases: &[],
            doc: "Open the workspace config.toml file.",
            fun: open_workspace_config,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "log-open",
            aliases: &[],
            doc: "Open the helix log file.",
            fun: open_log,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "insert-output",
            aliases: &[],
            doc: "Run shell command, inserting output before each selection.",
            fun: insert_output,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "append-output",
            aliases: &[],
            doc: "Run shell command, appending output after each selection.",
            fun: append_output,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "pipe",
            aliases: &[],
            doc: "Pipe each selection to the shell command.",
            fun: pipe,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "pipe-to",
            aliases: &[],
            doc: "Pipe each selection to the shell command, ignoring output.",
            fun: pipe_to,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "run-shell-command",
            aliases: &["sh"],
            doc: "Run a shell command",
            fun: run_shell_command,
            signature: CommandSignature::all(completers::filename)
        },
       TypableCommand {
            name: "reset-diff-change",
            aliases: &["diffget", "diffg"],
            doc: "Reset the diff change at the cursor position.",
            fun: reset_diff_change,
            signature: CommandSignature::none(),
        },
        TypableCommand {
            name: "clear-register",
            aliases: &[],
            doc: "Clear given register. If no argument is provided, clear all registers.",
            fun: clear_register,
            signature: CommandSignature::none(),
        },
    ];

pub static TYPABLE_COMMAND_MAP: Lazy<HashMap<&'static str, &'static TypableCommand>> =
    Lazy::new(|| {
        TYPABLE_COMMAND_LIST
            .iter()
            .flat_map(|cmd| {
                std::iter::once((cmd.name, cmd))
                    .chain(cmd.aliases.iter().map(move |&alias| (alias, cmd)))
            })
            .collect()
    });

#[allow(clippy::unnecessary_unwrap)]
pub(super) fn command_mode(cx: &mut Context) {
    let mut prompt = Prompt::new(
        ":".into(),
        Some(':'),
        |editor: &Editor, input: &str| {
            static FUZZY_MATCHER: Lazy<fuzzy_matcher::skim::SkimMatcherV2> =
                Lazy::new(fuzzy_matcher::skim::SkimMatcherV2::default);

            let shellwords = Shellwords::from(input);
            let words = shellwords.words();

            if words.is_empty() || (words.len() == 1 && !shellwords.ends_with_whitespace()) {
                // If the command has not been finished yet, complete commands.
                let mut matches: Vec<_> = typed::TYPABLE_COMMAND_LIST
                    .iter()
                    .filter_map(|command| {
                        FUZZY_MATCHER
                            .fuzzy_match(command.name, input)
                            .map(|score| (command.name, score))
                    })
                    .collect();

                matches.sort_unstable_by_key(|(_file, score)| std::cmp::Reverse(*score));
                matches
                    .into_iter()
                    .map(|(name, _)| (0.., name.into()))
                    .collect()
            } else {
                // Otherwise, use the command's completer and the last shellword
                // as completion input.
                let (part, part_len) = if words.len() == 1 || shellwords.ends_with_whitespace() {
                    (&Cow::Borrowed(""), 0)
                } else {
                    (
                        words.last().unwrap(),
                        shellwords.parts().last().unwrap().len(),
                    )
                };

                let argument_number = argument_number_of(&shellwords);

                if let Some(completer) = TYPABLE_COMMAND_MAP
                    .get(&words[0] as &str)
                    .map(|tc| tc.completer_for_argument_number(argument_number))
                {
                    completer(editor, part)
                        .into_iter()
                        .map(|(range, file)| {
                            let file = shellwords::escape(file);

                            // offset ranges to input
                            let offset = input.len() - part_len;
                            let range = (range.start + offset)..;
                            (range, file)
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
        }, // completion
        move |cx: &mut compositor::Context, input: &str, event: PromptEvent| {
            let parts = input.split_whitespace().collect::<Vec<&str>>();
            if parts.is_empty() {
                return;
            }

            // If command is numeric, interpret as line number and go there.
            if parts.len() == 1 && parts[0].parse::<usize>().ok().is_some() {
                if let Err(e) = typed::goto_line_number(cx, &[Cow::from(parts[0])], event) {
                    cx.editor.set_error(format!("{}", e));
                }
                return;
            }

            // Handle typable commands
            if let Some(cmd) = typed::TYPABLE_COMMAND_MAP.get(parts[0]) {
                let shellwords = Shellwords::from(input);
                let args = shellwords.words();

                if let Err(e) = (cmd.fun)(cx, &args[1..], event) {
                    cx.editor.set_error(format!("{}", e));
                }
            } else if event == PromptEvent::Validate {
                cx.editor
                    .set_error(format!("no such command: '{}'", parts[0]));
            }
        },
    );
    prompt.doc_fn = Box::new(|input: &str| {
        let part = input.split(' ').next().unwrap_or_default();

        if let Some(typed::TypableCommand { doc, aliases, .. }) =
            typed::TYPABLE_COMMAND_MAP.get(part)
        {
            if aliases.is_empty() {
                return Some((*doc).into());
            }
            return Some(format!("{}\nAliases: {}", doc, aliases.join(", ")).into());
        }

        None
    });

    // Calculate initial completion
    prompt.recalculate_completion(cx.editor);
    cx.push_layer(Box::new(prompt));
}

fn argument_number_of(shellwords: &Shellwords) -> usize {
    if shellwords.ends_with_whitespace() {
        shellwords.words().len().saturating_sub(1)
    } else {
        shellwords.words().len().saturating_sub(2)
    }
}

#[test]
fn test_argument_number_of() {
    let cases = vec![
        ("set-option", 0),
        ("set-option ", 0),
        ("set-option a", 0),
        ("set-option asdf", 0),
        ("set-option asdf ", 1),
        ("set-option asdf xyz", 1),
        ("set-option asdf xyz abc", 2),
        ("set-option asdf xyz abc ", 3),
    ];

    for case in cases {
        assert_eq!(case.1, argument_number_of(&Shellwords::from(case.0)));
    }
}
