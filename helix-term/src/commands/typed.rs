use std::ops::Deref;

use super::*;

use helix_view::editor::{Action, ConfigEvent};
use ui::completers::{self, Completer};

#[derive(Clone)]
pub struct TypableCommand {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub doc: &'static str,
    // params, flags, helper, completer
    pub fun: fn(&mut compositor::Context, &[Cow<str>], PromptEvent) -> anyhow::Result<()>,
    pub completer: Option<Completer>,
}

fn quit(cx: &mut compositor::Context, args: &[Cow<str>], event: PromptEvent) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    ensure!(args.is_empty(), ":quit takes no arguments");

    // last view and we have unsaved changes
    if cx.editor.tree.views().count() == 1 {
        buffers_remaining_impl(cx.editor)?
    }

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
        let _ = cx.editor.open(&path, Action::Replace)?;
        let (view, doc) = current!(cx.editor);
        let pos = Selection::point(pos_at_coords(doc.text().slice(..), pos, true));
        doc.set_selection(view.id, pos);
        // does not affect opening a buffer without pos
        align_view(doc, view, Align::Center);
    }
    Ok(())
}

fn buffer_close_by_ids_impl(
    editor: &mut Editor,
    doc_ids: &[DocumentId],
    force: bool,
) -> anyhow::Result<()> {
    for &doc_id in doc_ids {
        editor.close_document(doc_id, force)?;
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
    buffer_close_by_ids_impl(cx.editor, &document_ids, false)
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
    buffer_close_by_ids_impl(cx.editor, &document_ids, true)
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
    buffer_close_by_ids_impl(cx.editor, &document_ids, false)
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
    buffer_close_by_ids_impl(cx.editor, &document_ids, true)
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
    buffer_close_by_ids_impl(cx.editor, &document_ids, false)
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
    buffer_close_by_ids_impl(cx.editor, &document_ids, true)
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
    let auto_format = cx.editor.config().auto_format;
    let jobs = &mut cx.jobs;
    let doc = doc_mut!(cx.editor);

    if let Some(ref path) = path {
        doc.set_path(Some(path.as_ref().as_ref()))
            .context("invalid filepath")?;
    }
    if doc.path().is_none() {
        bail!("cannot write a buffer without a filename");
    }
    let fmt = if auto_format {
        doc.auto_format().map(|fmt| {
            let shared = fmt.shared();
            let callback = make_format_callback(
                doc.id(),
                doc.version(),
                Modified::SetUnmodified,
                shared.clone(),
            );
            jobs.callback(callback);
            shared
        })
    } else {
        None
    };
    let future = doc.format_and_save(fmt, force);
    cx.jobs.add(Job::new(future).wait_before_exiting());

    if path.is_some() {
        let id = doc.id();
        doc.detect_language(cx.editor.syn_loader.clone());
        let _ = cx.editor.refresh_language_server(id);
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

    let doc = doc!(cx.editor);
    if let Some(format) = doc.format() {
        let callback =
            make_format_callback(doc.id(), doc.version(), Modified::LeaveModified, format);
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
    doc.append_changes_to_history(view.id);

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
    let success = doc.earlier(view.id, uk);
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
    let success = doc.later(view.id, uk);
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
    helix_lsp::block_on(cx.jobs.finish())?;
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
    force_quit(cx, &[], event)
}

/// Results an error if there are modified buffers remaining and sets editor error,
/// otherwise returns `Ok(())`
pub(super) fn buffers_remaining_impl(editor: &mut Editor) -> anyhow::Result<()> {
    let modified: Vec<_> = editor
        .documents()
        .filter(|doc| doc.is_modified())
        .map(|doc| {
            doc.relative_path()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into())
        })
        .collect();
    if !modified.is_empty() {
        bail!(
            "{} unsaved buffer(s) remaining: {:?}",
            modified.len(),
            modified
        );
    }
    Ok(())
}

fn write_all_impl(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
    quit: bool,
    force: bool,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let mut errors = String::new();
    let auto_format = cx.editor.config().auto_format;
    let jobs = &mut cx.jobs;
    // save all documents
    for doc in &mut cx.editor.documents.values_mut() {
        if doc.path().is_none() {
            errors.push_str("cannot write a buffer without a filename\n");
            continue;
        }

        if !doc.is_modified() {
            continue;
        }

        let fmt = if auto_format {
            doc.auto_format().map(|fmt| {
                let shared = fmt.shared();
                let callback = make_format_callback(
                    doc.id(),
                    doc.version(),
                    Modified::SetUnmodified,
                    shared.clone(),
                );
                jobs.callback(callback);
                shared
            })
        } else {
            None
        };
        let future = doc.format_and_save(fmt, force);
        jobs.add(Job::new(future).wait_before_exiting());
    }

    if quit {
        if !force {
            buffers_remaining_impl(cx.editor)?;
        }

        // close all views
        let views: Vec<_> = cx.editor.tree.views().map(|(view, _)| view.id).collect();
        for view_id in views {
            cx.editor.close(view_id);
        }
    }

    bail!(errors)
}

fn write_all(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    write_all_impl(cx, args, event, false, false)
}

fn write_all_quit(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    write_all_impl(cx, args, event, true, false)
}

fn force_write_all_quit(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    write_all_impl(cx, args, event, true, true)
}

fn quit_all_impl(editor: &mut Editor, force: bool) -> anyhow::Result<()> {
    if !force {
        buffers_remaining_impl(editor)?;
    }

    // close all views
    let views: Vec<_> = editor.tree.views().map(|(view, _)| view.id).collect();
    for view_id in views {
        editor.close(view_id);
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

    quit_all_impl(cx.editor, false)
}

fn force_quit_all(
    cx: &mut compositor::Context,
    _args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    quit_all_impl(cx.editor, true)
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

    quit_all_impl(cx.editor, false)
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

    quit_all_impl(cx.editor, true)
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
            if let Some(theme_name) = args.first() {
                if let Ok(theme) = cx.editor.theme_loader.load(theme_name) {
                    if !(true_color || theme.is_16_color()) {
                        bail!("Unsupported theme: theme requires true color support");
                    }
                    cx.editor.set_theme_preview(theme);
                };
            };
        }
        PromptEvent::Validate => {
            let theme_name = args.first().with_context(|| "Theme name not provided")?;
            let theme = cx
                .editor
                .theme_loader
                .load(theme_name)
                .with_context(|| "Theme does not exist")?;
            if !(true_color || theme.is_16_color()) {
                bail!("Unsupported theme: theme requires true color support");
            }
            cx.editor.set_theme(theme);
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
    let (view, doc) = current!(cx.editor);

    match cx.editor.clipboard_provider.get_contents(clipboard_type) {
        Ok(contents) => {
            let selection = doc.selection(view.id);
            let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
                (range.from(), range.to(), Some(contents.as_str().into()))
            });

            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
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
    let (view, doc) = current!(cx.editor);
    doc.reload(view.id).map(|_| {
        view.ensure_cursor_in_view(doc, scrolloff);
    })
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
    cx.editor.set_status(format!("scopes: {:?}", &scopes));
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

    let path = helix_loader::runtime_dir().join("tutor.txt");
    cx.editor.open(&path, Action::Replace)?;
    // Unset path to prevent accidentally saving to the original tutor file.
    doc_mut!(cx.editor).set_path(None)?;
    Ok(())
}

pub(super) fn goto_line_number(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    ensure!(!args.is_empty(), "Line number required");

    let line = args[0].parse::<usize>()?;

    goto_line_impl(cx.editor, NonZeroUsize::new(line));

    let (view, doc) = current!(cx.editor);

    view.ensure_cursor_in_view(doc, line);
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

/// Change the language of the current buffer at runtime.
fn language(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    if args.len() != 1 {
        anyhow::bail!("Bad arguments. Usage: `:set-language language`");
    }

    let doc = doc_mut!(cx.editor);
    doc.set_language_by_language_id(&args[0], cx.editor.syn_loader.clone());

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
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id);

    let mut fragments: Vec<_> = selection
        .fragments(text)
        .map(|fragment| Tendril::from(fragment.as_ref()))
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
    doc.append_changes_to_history(view.id);

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

    let (view, doc) = current!(cx.editor);

    const DEFAULT_MAX_LEN: usize = 79;

    // Find the max line length by checking the following sources in order:
    //   - The passed argument in `args`
    //   - The configured max_line_len for this language in languages.toml
    //   - The const default we set above
    let max_line_len: usize = args
        .get(0)
        .map(|num| num.parse::<usize>())
        .transpose()?
        .or_else(|| {
            doc.language_config()
                .and_then(|config| config.max_line_length)
        })
        .unwrap_or(DEFAULT_MAX_LEN);

    let rope = doc.text();

    let selection = doc.selection(view.id);
    let transaction = Transaction::change_by_selection(rope, selection, |range| {
        let fragment = range.fragment(rope.slice(..));
        let reflowed_text = helix_core::wrap::reflow_hard_wrap(&fragment, max_line_len);

        (range.from(), range.to(), Some(reflowed_text))
    });

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);

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
            let contents = format!("```tsq\n{}\n```", selected_node.to_sexp());

            let callback = async move {
                let call: job::Callback =
                    Box::new(move |editor: &mut Editor, compositor: &mut Compositor| {
                        let contents = ui::Markdown::new(contents, editor.syn_loader.clone());
                        let popup = Popup::new("hover", contents).auto_close(true);
                        compositor.replace_or_push("hover", popup);
                    });
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

fn pipe(cx: &mut compositor::Context, args: &[Cow<str>], event: PromptEvent) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    ensure!(!args.is_empty(), "Shell command required");
    shell(cx, &args.join(" "), &ShellBehavior::Replace);
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

    let shell = &cx.editor.config().shell;
    let (output, success) = shell_impl(shell, &args.join(" "), None)?;
    if success {
        cx.editor.set_status("Command succeed");
    } else {
        cx.editor.set_error("Command failed");
    }

    if !output.is_empty() {
        let callback = async move {
            let call: job::Callback =
                Box::new(move |editor: &mut Editor, compositor: &mut Compositor| {
                    let contents = ui::Markdown::new(
                        format!("```sh\n{}\n```", output),
                        editor.syn_loader.clone(),
                    );
                    let popup = Popup::new("shell", contents).position(Some(
                        helix_core::Position::new(editor.cursor().0.unwrap_or_default().row, 2),
                    ));
                    compositor.replace_or_push("shell", popup);
                });
            Ok(call)
        };

        cx.jobs.callback(callback);
    }

    Ok(())
}

pub const TYPABLE_COMMAND_LIST: &[TypableCommand] = &[
        TypableCommand {
            name: "quit",
            aliases: &["q"],
            doc: "Close the current view.",
            fun: quit,
            completer: None,
        },
        TypableCommand {
            name: "quit!",
            aliases: &["q!"],
            doc: "Force close the current view, ignoring unsaved changes.",
            fun: force_quit,
            completer: None,
        },
        TypableCommand {
            name: "open",
            aliases: &["o"],
            doc: "Open a file from disk into the current view.",
            fun: open,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "buffer-close",
            aliases: &["bc", "bclose"],
            doc: "Close the current buffer.",
            fun: buffer_close,
            completer: Some(completers::buffer),
        },
        TypableCommand {
            name: "buffer-close!",
            aliases: &["bc!", "bclose!"],
            doc: "Close the current buffer forcefully, ignoring unsaved changes.",
            fun: force_buffer_close,
            completer: Some(completers::buffer),
        },
        TypableCommand {
            name: "buffer-close-others",
            aliases: &["bco", "bcloseother"],
            doc: "Close all buffers but the currently focused one.",
            fun: buffer_close_others,
            completer: None,
        },
        TypableCommand {
            name: "buffer-close-others!",
            aliases: &["bco!", "bcloseother!"],
            doc: "Force close all buffers but the currently focused one.",
            fun: force_buffer_close_others,
            completer: None,
        },
        TypableCommand {
            name: "buffer-close-all",
            aliases: &["bca", "bcloseall"],
            doc: "Close all buffers without quitting.",
            fun: buffer_close_all,
            completer: None,
        },
        TypableCommand {
            name: "buffer-close-all!",
            aliases: &["bca!", "bcloseall!"],
            doc: "Force close all buffers ignoring unsaved changes without quitting.",
            fun: force_buffer_close_all,
            completer: None,
        },
        TypableCommand {
            name: "buffer-next",
            aliases: &["bn", "bnext"],
            doc: "Goto next buffer.",
            fun: buffer_next,
            completer: None,
        },
        TypableCommand {
            name: "buffer-previous",
            aliases: &["bp", "bprev"],
            doc: "Goto previous buffer.",
            fun: buffer_previous,
            completer: None,
        },
        TypableCommand {
            name: "write",
            aliases: &["w"],
            doc: "Write changes to disk. Accepts an optional path (:write some/path.txt)",
            fun: write,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write!",
            aliases: &["w!"],
            doc: "Force write changes to disk creating necessary subdirectories. Accepts an optional path (:write some/path.txt)",
            fun: force_write,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "new",
            aliases: &["n"],
            doc: "Create a new scratch buffer.",
            fun: new_file,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "format",
            aliases: &["fmt"],
            doc: "Format the file using the LSP formatter.",
            fun: format,
            completer: None,
        },
        TypableCommand {
            name: "indent-style",
            aliases: &[],
            doc: "Set the indentation style for editing. ('t' for tabs or 1-8 for number of spaces.)",
            fun: set_indent_style,
            completer: None,
        },
        TypableCommand {
            name: "line-ending",
            aliases: &[],
            #[cfg(not(feature = "unicode-lines"))]
            doc: "Set the document's default line ending. Options: crlf, lf.",
            #[cfg(feature = "unicode-lines")]
            doc: "Set the document's default line ending. Options: crlf, lf, cr, ff, nel.",
            fun: set_line_ending,
            completer: None,
        },
        TypableCommand {
            name: "earlier",
            aliases: &["ear"],
            doc: "Jump back to an earlier point in edit history. Accepts a number of steps or a time span.",
            fun: earlier,
            completer: None,
        },
        TypableCommand {
            name: "later",
            aliases: &["lat"],
            doc: "Jump to a later point in edit history. Accepts a number of steps or a time span.",
            fun: later,
            completer: None,
        },
        TypableCommand {
            name: "write-quit",
            aliases: &["wq", "x"],
            doc: "Write changes to disk and close the current view. Accepts an optional path (:wq some/path.txt)",
            fun: write_quit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write-quit!",
            aliases: &["wq!", "x!"],
            doc: "Write changes to disk and close the current view forcefully. Accepts an optional path (:wq! some/path.txt)",
            fun: force_write_quit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write-all",
            aliases: &["wa"],
            doc: "Write changes from all buffers to disk.",
            fun: write_all,
            completer: None,
        },
        TypableCommand {
            name: "write-quit-all",
            aliases: &["wqa", "xa"],
            doc: "Write changes from all buffers to disk and close all views.",
            fun: write_all_quit,
            completer: None,
        },
        TypableCommand {
            name: "write-quit-all!",
            aliases: &["wqa!", "xa!"],
            doc: "Write changes from all buffers to disk and close all views forcefully (ignoring unsaved changes).",
            fun: force_write_all_quit,
            completer: None,
        },
        TypableCommand {
            name: "quit-all",
            aliases: &["qa"],
            doc: "Close all views.",
            fun: quit_all,
            completer: None,
        },
        TypableCommand {
            name: "quit-all!",
            aliases: &["qa!"],
            doc: "Force close all views ignoring unsaved changes.",
            fun: force_quit_all,
            completer: None,
        },
        TypableCommand {
            name: "cquit",
            aliases: &["cq"],
            doc: "Quit with exit code (default 1). Accepts an optional integer exit code (:cq 2).",
            fun: cquit,
            completer: None,
        },
        TypableCommand {
            name: "cquit!",
            aliases: &["cq!"],
            doc: "Force quit with exit code (default 1) ignoring unsaved changes. Accepts an optional integer exit code (:cq! 2).",
            fun: force_cquit,
            completer: None,
        },
        TypableCommand {
            name: "theme",
            aliases: &[],
            doc: "Change the editor theme.",
            fun: theme,
            completer: Some(completers::theme),
        },
        TypableCommand {
            name: "clipboard-yank",
            aliases: &[],
            doc: "Yank main selection into system clipboard.",
            fun: yank_main_selection_to_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-yank-join",
            aliases: &[],
            doc: "Yank joined selections into system clipboard. A separator can be provided as first argument. Default value is newline.", // FIXME: current UI can't display long doc.
            fun: yank_joined_to_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-yank",
            aliases: &[],
            doc: "Yank main selection into system primary clipboard.",
            fun: yank_main_selection_to_primary_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-yank-join",
            aliases: &[],
            doc: "Yank joined selections into system primary clipboard. A separator can be provided as first argument. Default value is newline.", // FIXME: current UI can't display long doc.
            fun: yank_joined_to_primary_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-after",
            aliases: &[],
            doc: "Paste system clipboard after selections.",
            fun: paste_clipboard_after,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-before",
            aliases: &[],
            doc: "Paste system clipboard before selections.",
            fun: paste_clipboard_before,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-replace",
            aliases: &[],
            doc: "Replace selections with content of system clipboard.",
            fun: replace_selections_with_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-paste-after",
            aliases: &[],
            doc: "Paste primary clipboard after selections.",
            fun: paste_primary_clipboard_after,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-paste-before",
            aliases: &[],
            doc: "Paste primary clipboard before selections.",
            fun: paste_primary_clipboard_before,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-paste-replace",
            aliases: &[],
            doc: "Replace selections with content of system primary clipboard.",
            fun: replace_selections_with_primary_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "show-clipboard-provider",
            aliases: &[],
            doc: "Show clipboard provider name in status bar.",
            fun: show_clipboard_provider,
            completer: None,
        },
        TypableCommand {
            name: "change-current-directory",
            aliases: &["cd"],
            doc: "Change the current working directory.",
            fun: change_current_directory,
            completer: Some(completers::directory),
        },
        TypableCommand {
            name: "show-directory",
            aliases: &["pwd"],
            doc: "Show the current working directory.",
            fun: show_current_directory,
            completer: None,
        },
        TypableCommand {
            name: "encoding",
            aliases: &[],
            doc: "Set encoding. Based on `https://encoding.spec.whatwg.org`.",
            fun: set_encoding,
            completer: None,
        },
        TypableCommand {
            name: "reload",
            aliases: &[],
            doc: "Discard changes and reload from the source file.",
            fun: reload,
            completer: None,
        },
        TypableCommand {
            name: "tree-sitter-scopes",
            aliases: &[],
            doc: "Display tree sitter scopes, primarily for theming and development.",
            fun: tree_sitter_scopes,
            completer: None,
       },
        TypableCommand {
            name: "debug-start",
            aliases: &["dbg"],
            doc: "Start a debug session from a given template with given parameters.",
            fun: debug_start,
            completer: None,
        },
        TypableCommand {
            name: "debug-remote",
            aliases: &["dbg-tcp"],
            doc: "Connect to a debug adapter by TCP address and start a debugging session from a given template with given parameters.",
            fun: debug_remote,
            completer: None,
        },
        TypableCommand {
            name: "debug-eval",
            aliases: &[],
            doc: "Evaluate expression in current debug context.",
            fun: debug_eval,
            completer: None,
        },
        TypableCommand {
            name: "vsplit",
            aliases: &["vs"],
            doc: "Open the file in a vertical split.",
            fun: vsplit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "vsplit-new",
            aliases: &["vnew"],
            doc: "Open a scratch buffer in a vertical split.",
            fun: vsplit_new,
            completer: None,
        },
        TypableCommand {
            name: "hsplit",
            aliases: &["hs", "sp"],
            doc: "Open the file in a horizontal split.",
            fun: hsplit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "hsplit-new",
            aliases: &["hnew"],
            doc: "Open a scratch buffer in a horizontal split.",
            fun: hsplit_new,
            completer: None,
        },
        TypableCommand {
            name: "tutor",
            aliases: &[],
            doc: "Open the tutorial.",
            fun: tutor,
            completer: None,
        },
        TypableCommand {
            name: "goto",
            aliases: &["g"],
            doc: "Goto line number.",
            fun: goto_line_number,
            completer: None,
        },
        TypableCommand {
            name: "set-language",
            aliases: &["lang"],
            doc: "Set the language of current buffer.",
            fun: language,
            completer: Some(completers::language),
        },
        TypableCommand {
            name: "set-option",
            aliases: &["set"],
            doc: "Set a config option at runtime.\nFor example to disable smart case search, use `:set search.smart-case false`.",
            fun: set_option,
            completer: Some(completers::setting),
        },
        TypableCommand {
            name: "get-option",
            aliases: &["get"],
            doc: "Get the current value of a config option.",
            fun: get_option,
            completer: Some(completers::setting),
        },
        TypableCommand {
            name: "sort",
            aliases: &[],
            doc: "Sort ranges in selection.",
            fun: sort,
            completer: None,
        },
        TypableCommand {
            name: "rsort",
            aliases: &[],
            doc: "Sort ranges in selection in reverse order.",
            fun: sort_reverse,
            completer: None,
        },
        TypableCommand {
            name: "reflow",
            aliases: &[],
            doc: "Hard-wrap the current selection of lines to a given width.",
            fun: reflow,
            completer: None,
        },
        TypableCommand {
            name: "tree-sitter-subtree",
            aliases: &["ts-subtree"],
            doc: "Display tree sitter subtree under cursor, primarily for debugging queries.",
            fun: tree_sitter_subtree,
            completer: None,
        },
        TypableCommand {
            name: "config-reload",
            aliases: &[],
            doc: "Refresh user config.",
            fun: refresh_config,
            completer: None,
        },
        TypableCommand {
            name: "config-open",
            aliases: &[],
            doc: "Open the user config.toml file.",
            fun: open_config,
            completer: None,
        },
        TypableCommand {
            name: "log-open",
            aliases: &[],
            doc: "Open the helix log file.",
            fun: open_log,
            completer: None,
        },
        TypableCommand {
            name: "insert-output",
            aliases: &[],
            doc: "Run shell command, inserting output after each selection.",
            fun: insert_output,
            completer: None,
        },
        TypableCommand {
            name: "append-output",
            aliases: &[],
            doc: "Run shell command, appending output after each selection.",
            fun: append_output,
            completer: None,
        },
        TypableCommand {
            name: "pipe",
            aliases: &[],
            doc: "Pipe each selection to the shell command.",
            fun: pipe,
            completer: None,
        },
        TypableCommand {
            name: "run-shell-command",
            aliases: &["sh"],
            doc: "Run a shell command",
            fun: run_shell_command,
            completer: Some(completers::directory),
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

pub fn command_mode(cx: &mut Context) {
    let mut prompt = Prompt::new(
        ":".into(),
        Some(':'),
        |editor: &Editor, input: &str| {
            static FUZZY_MATCHER: Lazy<fuzzy_matcher::skim::SkimMatcherV2> =
                Lazy::new(fuzzy_matcher::skim::SkimMatcherV2::default);

            // we use .this over split_whitespace() because we care about empty segments
            let parts = input.split(' ').collect::<Vec<&str>>();

            // simple heuristic: if there's no just one part, complete command name.
            // if there's a space, per command completion kicks in.
            if parts.len() <= 1 {
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
                let part = parts.last().unwrap();

                if let Some(typed::TypableCommand {
                    completer: Some(completer),
                    ..
                }) = typed::TYPABLE_COMMAND_MAP.get(parts[0])
                {
                    completer(editor, part)
                        .into_iter()
                        .map(|(range, file)| {
                            // offset ranges to input
                            let offset = input.len() - part.len();
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
                let args = shellwords::shellwords(input);

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
