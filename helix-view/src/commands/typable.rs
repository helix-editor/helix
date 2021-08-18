use super::{make_format_callback, Modified};
use crate::{
    clipboard::ClipboardType,
    completers::{self, Completer},
    editor::Action,
    job::Job,
    widgets, Document, Editor, View,
};
use anyhow::{anyhow, bail};
use helix_core::{indent, line_ending::get_line_ending_of_str, Tendril, Transaction};
use helix_lsp::lsp;
use once_cell::sync::Lazy;
use std::{borrow::Cow, collections::HashMap};

pub struct Context {}

#[derive(Clone)]
pub struct TypableCommand<C>
where
    C: Fn(&str) -> Vec<lsp::request::Completion>,
{
    pub name: &'static str,
    pub alias: Option<&'static str>,
    pub doc: &'static str,
    // params, flags, helper, completer
    pub fun: fn(&mut Context, &[&str], widgets::prompt::Event) -> anyhow::Result<()>,
    pub completer: Option<Completer>,
}

fn quit(cx: &mut Context, _args: &[&str], _event: widgets::prompt::Event) -> anyhow::Result<()> {
    // last view and we have unsaved changes
    if cx.editor.tree.views().count() == 1 {
        buffers_remaining_impl(cx.editor)?
    }

    cx.editor
        .close(view!(cx.editor).id, /* close_buffer */ false);

    Ok(())
}

fn force_quit(
    cx: &mut Context,
    _args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    cx.editor
        .close(view!(cx.editor).id, /* close_buffer */ false);

    Ok(())
}

fn open(cx: &mut Context, args: &[&str], _event: widgets::prompt::Event) -> anyhow::Result<()> {
    let path = args.get(0).context("wrong argument count")?;
    let _ = cx.editor.open(path.into(), Action::Replace)?;
    Ok(())
}

fn write_impl<P: AsRef<std::path::Path>>(
    cx: &mut Context,
    path: Option<P>,
) -> Result<tokio::task::JoinHandle<Result<(), anyhow::Error>>, anyhow::Error> {
    let jobs = &mut cx.jobs;
    let (_, doc) = current!(cx.editor);

    if let Some(path) = path {
        doc.set_path(path.as_ref()).context("invalid filepath")?;
    }
    if doc.path().is_none() {
        bail!("cannot write a buffer without a filename");
    }
    let fmt = doc.auto_format().map(|fmt| {
        let shared = fmt.shared();
        let callback = make_format_callback(
            doc.id(),
            doc.version(),
            Modified::SetUnmodified,
            shared.clone(),
        );
        jobs.callback(callback);
        shared
    });
    Ok(tokio::spawn(doc.format_and_save(fmt)))
}

fn write(cx: &mut Context, args: &[&str], _event: widgets::prompt::Event) -> anyhow::Result<()> {
    let handle = write_impl(cx, args.first())?;
    cx.jobs
        .add(Job::new(handle.unwrap_or_else(|e| Err(e.into()))).wait_before_exiting());

    Ok(())
}

fn new_file(
    cx: &mut Context,
    _args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    cx.editor.new_file(Action::Replace);

    Ok(())
}

fn format(cx: &mut Context, _args: &[&str], _event: widgets::prompt::Event) -> anyhow::Result<()> {
    let (_, doc) = current!(cx.editor);

    if let Some(format) = doc.format() {
        let callback =
            make_format_callback(doc.id(), doc.version(), Modified::LeaveModified, format);
        cx.jobs.callback(callback);
    }

    Ok(())
}
fn set_indent_style(
    cx: &mut Context,
    args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    use helix_core::indent::IndentStyle::*;

    // If no argument, report current indent style.
    if args.is_empty() {
        let style = current!(cx.editor).1.indent_style;
        cx.editor.set_status(match style {
            Tabs => "tabs".into(),
            Spaces(1) => "1 space".into(),
            Spaces(n) if (2..=8).contains(&n) => format!("{} spaces", n),
            _ => "error".into(), // Shouldn't happen.
        });
        return Ok(());
    }

    // Attempt to parse argument as an indent style.
    let style = match args.get(0) {
        Some(arg) if "tabs".starts_with(&arg.to_lowercase()) => Some(Tabs),
        Some(&"0") => Some(Tabs),
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
    cx: &mut Context,
    args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    use helix_core::LineEnding::*;

    // If no argument, report current line ending setting.
    if args.is_empty() {
        let line_ending = current!(cx.editor).1.line_ending;
        cx.editor.set_status(match line_ending {
            Crlf => "crlf".into(),
            LF => "line feed".into(),
            FF => "form feed".into(),
            CR => "carriage return".into(),
            Nel => "next line".into(),

            // These should never be a document's default line ending.
            VT | LS | PS => "error".into(),
        });

        return Ok(());
    }

    // Attempt to parse argument as a line ending.
    let line_ending = match args.get(0) {
        // We check for CR first because it shares a common prefix with CRLF.
        Some(arg) if "cr".starts_with(&arg.to_lowercase()) => Some(CR),
        Some(arg) if "crlf".starts_with(&arg.to_lowercase()) => Some(Crlf),
        Some(arg) if "lf".starts_with(&arg.to_lowercase()) => Some(LF),
        Some(arg) if "ff".starts_with(&arg.to_lowercase()) => Some(FF),
        Some(arg) if "nel".starts_with(&arg.to_lowercase()) => Some(Nel),
        _ => None,
    };

    let line_ending = line_ending.context("invalid line ending")?;
    doc_mut!(cx.editor).line_ending = line_ending;
    Ok(())
}

fn earlier(cx: &mut Context, args: &[&str], _event: widgets::prompt::Event) -> anyhow::Result<()> {
    let uk = args
        .join(" ")
        .parse::<helix_core::history::UndoKind>()
        .map_err(|s| anyhow!(s))?;

    let (view, doc) = current!(cx.editor);
    doc.earlier(view.id, uk);

    Ok(())
}

fn later(cx: &mut Context, args: &[&str], _event: widgets::prompt::Event) -> anyhow::Result<()> {
    let uk = args
        .join(" ")
        .parse::<helix_core::history::UndoKind>()
        .map_err(|s| anyhow!(s))?;
    let (view, doc) = current!(cx.editor);
    doc.later(view.id, uk);

    Ok(())
}

fn write_quit(
    cx: &mut Context,
    args: &[&str],
    event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    let handle = write_impl(cx, args.first())?;
    let _ = helix_lsp::block_on(handle)?;
    quit(cx, &[], event)
}

fn force_write_quit(
    cx: &mut Context,
    args: &[&str],
    event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    let handle = write_impl(cx, args.first())?;
    let _ = helix_lsp::block_on(handle)?;
    force_quit(cx, &[], event)
}

/// Results an error if there are modified buffers remaining and sets editor error,
/// otherwise returns `Ok(())`
fn buffers_remaining_impl(editor: &mut Editor) -> anyhow::Result<()> {
    let modified: Vec<_> = editor
        .documents()
        .filter(|doc| doc.is_modified())
        .map(|doc| {
            doc.relative_path()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|| "[scratch]".into())
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
    editor: &mut Editor,
    _args: &[&str],
    _event: widgets::prompt::Event,
    quit: bool,
    force: bool,
) -> anyhow::Result<()> {
    let mut errors = String::new();

    // save all documents
    for (_, doc) in &mut editor.documents {
        if doc.path().is_none() {
            errors.push_str("cannot write a buffer without a filename\n");
            continue;
        }

        // TODO: handle error.
        let _ = helix_lsp::block_on(tokio::spawn(doc.save()));
    }

    if quit {
        if !force {
            buffers_remaining_impl(editor)?;
        }

        // close all views
        let views: Vec<_> = editor.tree.views().map(|(view, _)| view.id).collect();
        for view_id in views {
            editor.close(view_id, false);
        }
    }

    bail!(errors)
}

fn write_all(cx: &mut Context, args: &[&str], event: widgets::prompt::Event) -> anyhow::Result<()> {
    write_all_impl(&mut cx.editor, args, event, false, false)
}

fn write_all_quit(
    cx: &mut Context,
    args: &[&str],
    event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    write_all_impl(&mut cx.editor, args, event, true, false)
}

fn force_write_all_quit(
    cx: &mut Context,
    args: &[&str],
    event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    write_all_impl(&mut cx.editor, args, event, true, true)
}

fn quit_all_impl(
    editor: &mut Editor,
    _args: &[&str],
    _event: widgets::prompt::Event,
    force: bool,
) -> anyhow::Result<()> {
    if !force {
        buffers_remaining_impl(editor)?;
    }

    // close all views
    let views: Vec<_> = editor.tree.views().map(|(view, _)| view.id).collect();
    for view_id in views {
        editor.close(view_id, false);
    }

    Ok(())
}

fn quit_all(cx: &mut Context, args: &[&str], event: widgets::prompt::Event) -> anyhow::Result<()> {
    quit_all_impl(&mut cx.editor, args, event, false)
}

fn force_quit_all(
    cx: &mut Context,
    args: &[&str],
    event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    quit_all_impl(&mut cx.editor, args, event, true)
}

fn theme(cx: &mut Context, args: &[&str], _event: widgets::prompt::Event) -> anyhow::Result<()> {
    let theme = args.first().context("theme not provided")?;
    cx.editor.set_theme_from_name(theme)
}

fn yank_joined_to_clipboard_impl(
    editor: &mut Editor,
    separator: &str,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let values: Vec<String> = doc
        .selection(view.id)
        .fragments(text)
        .map(Cow::into_owned)
        .collect();

    let msg = format!(
        "joined and yanked {} selection(s) to system clipboard",
        values.len(),
    );

    let joined = values.join(separator);

    editor
        .clipboard_provider
        .set_contents(joined, clipboard_type)
        .context("Couldn't set system clipboard content")?;

    editor.set_status(msg);

    Ok(())
}

fn yank_main_selection_to_clipboard_impl(
    editor: &mut Editor,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let value = doc.selection(view.id).primary().fragment(text);

    if let Err(e) = editor
        .clipboard_provider
        .set_contents(value.into_owned(), clipboard_type)
    {
        bail!("Couldn't set system clipboard content: {:?}", e);
    }

    editor.set_status("yanked main selection to system clipboard".to_owned());
    Ok(())
}

fn yank_main_selection_to_clipboard(
    cx: &mut Context,
    _args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    yank_main_selection_to_clipboard_impl(&mut cx.editor, ClipboardType::Clipboard)
}

fn yank_joined_to_clipboard(
    cx: &mut Context,
    args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    let (_, doc) = current!(cx.editor);
    let separator = args
        .first()
        .copied()
        .unwrap_or_else(|| doc.line_ending.as_str());
    yank_joined_to_clipboard_impl(&mut cx.editor, separator, ClipboardType::Clipboard)
}

fn yank_main_selection_to_primary_clipboard(
    cx: &mut Context,
    _args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    yank_main_selection_to_clipboard_impl(&mut cx.editor, ClipboardType::Selection)
}

fn yank_joined_to_primary_clipboard(
    cx: &mut Context,
    args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    let (_, doc) = current!(cx.editor);
    let separator = args
        .first()
        .copied()
        .unwrap_or_else(|| doc.line_ending.as_str());
    yank_joined_to_clipboard_impl(&mut cx.editor, separator, ClipboardType::Selection)
}

#[derive(Copy, Clone)]
enum Paste {
    Before,
    After,
}

fn paste_impl(
    values: &[String],
    doc: &mut Document,
    view: &View,
    action: Paste,
) -> Option<Transaction> {
    let repeat = std::iter::repeat(
        values
            .last()
            .map(|value| Tendril::from_slice(value))
            .unwrap(),
    );

    // if any of values ends with a line ending, it's linewise paste
    let linewise = values
        .iter()
        .any(|value| get_line_ending_of_str(value).is_some());

    let mut values = values.iter().cloned().map(Tendril::from).chain(repeat);

    let text = doc.text();
    let selection = doc.selection(view.id);

    let transaction = Transaction::change_by_selection(text, selection, |range| {
        let pos = match (action, linewise) {
            // paste linewise before
            (Paste::Before, true) => text.line_to_char(text.char_to_line(range.from())),
            // paste linewise after
            (Paste::After, true) => {
                let line = range.line_range(text.slice(..)).1;
                text.line_to_char((line + 1).min(text.len_lines()))
            }
            // paste insert
            (Paste::Before, false) => range.from(),
            // paste append
            (Paste::After, false) => range.to(),
        };
        (pos, pos, Some(values.next().unwrap()))
    });

    Some(transaction)
}

fn paste_clipboard_impl(
    editor: &mut Editor,
    action: Paste,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);

    match editor
        .clipboard_provider
        .get_contents(clipboard_type)
        .map(|contents| paste_impl(&[contents], doc, view, action))
    {
        Ok(Some(transaction)) => {
            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(e) => Err(e.context("Couldn't get system clipboard contents")),
    }
}

fn paste_clipboard_after(
    cx: &mut Context,
    _args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Clipboard)
}

fn paste_clipboard_before(
    cx: &mut Context,
    _args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Clipboard)
}

fn paste_primary_clipboard_after(
    cx: &mut Context,
    _args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Selection)
}

fn paste_primary_clipboard_before(
    cx: &mut Context,
    _args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Selection)
}

fn replace_selections_with_clipboard_impl(
    cx: &mut Context,
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
    cx: &mut Context,
    _args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    replace_selections_with_clipboard_impl(cx, ClipboardType::Clipboard)
}

fn replace_selections_with_primary_clipboard(
    cx: &mut Context,
    _args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    replace_selections_with_clipboard_impl(cx, ClipboardType::Selection)
}

fn show_clipboard_provider(
    cx: &mut Context,
    _args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    cx.editor
        .set_status(cx.editor.clipboard_provider.name().to_string());
    Ok(())
}

fn change_current_directory(
    cx: &mut Context,
    args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    let dir = args.first().context("target directory not provided")?;

    if let Err(e) = std::env::set_current_dir(dir) {
        bail!("Couldn't change the current working directory: {:?}", e);
    }

    let cwd = std::env::current_dir().context("Couldn't get the new working directory")?;
    cx.editor.set_status(format!(
        "Current working directory is now {}",
        cwd.display()
    ));
    Ok(())
}

fn show_current_directory(
    cx: &mut Context,
    _args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir().context("Couldn't get the new working directory")?;
    cx.editor
        .set_status(format!("Current working directory is {}", cwd.display()));
    Ok(())
}

/// Sets the [`Document`]'s encoding..
fn set_encoding(
    cx: &mut Context,
    args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    let (_, doc) = current!(cx.editor);
    if let Some(label) = args.first() {
        doc.set_encoding(label)
    } else {
        let encoding = doc.encoding().name().to_string();
        cx.editor.set_status(encoding);
        Ok(())
    }
}

/// Reload the [`Document`] from its source file.
fn reload(cx: &mut Context, _args: &[&str], _event: widgets::prompt::Event) -> anyhow::Result<()> {
    let (view, doc) = current!(cx.editor);
    doc.reload(view.id)
}

fn tree_sitter_scopes(
    cx: &mut Context,
    _args: &[&str],
    _event: widgets::prompt::Event,
) -> anyhow::Result<()> {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let pos = doc.selection(view.id).primary().cursor(text);
    let scopes = indent::get_scopes(doc.syntax(), text, pos);
    cx.editor.set_status(format!("scopes: {:?}", &scopes));
    Ok(())
}

pub const TYPABLE_COMMAND_LIST: &[TypableCommand] = &[
        TypableCommand {
            name: "quit",
            alias: Some("q"),
            doc: "Close the current view.",
            fun: quit,
            completer: None,
        },
        TypableCommand {
            name: "quit!",
            alias: Some("q!"),
            doc: "Close the current view.",
            fun: force_quit,
            completer: None,
        },
        TypableCommand {
            name: "open",
            alias: Some("o"),
            doc: "Open a file from disk into the current view.",
            fun: open,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write",
            alias: Some("w"),
            doc: "Write changes to disk. Accepts an optional path (:write some/path.txt)",
            fun: write,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "new",
            alias: Some("n"),
            doc: "Create a new scratch buffer.",
            fun: new_file,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "format",
            alias: Some("fmt"),
            doc: "Format the file using a formatter.",
            fun: format,
            completer: None,
        },
        TypableCommand {
            name: "indent-style",
            alias: None,
            doc: "Set the indentation style for editing. ('t' for tabs or 1-8 for number of spaces.)",
            fun: set_indent_style,
            completer: None,
        },
        TypableCommand {
            name: "line-ending",
            alias: None,
            doc: "Set the document's default line ending. Options: crlf, lf, cr, ff, nel.",
            fun: set_line_ending,
            completer: None,
        },
        TypableCommand {
            name: "earlier",
            alias: Some("ear"),
            doc: "Jump back to an earlier point in edit history. Accepts a number of steps or a time span.",
            fun: earlier,
            completer: None,
        },
        TypableCommand {
            name: "later",
            alias: Some("lat"),
            doc: "Jump to a later point in edit history. Accepts a number of steps or a time span.",
            fun: later,
            completer: None,
        },
        TypableCommand {
            name: "write-quit",
            alias: Some("wq"),
            doc: "Writes changes to disk and closes the current view. Accepts an optional path (:wq some/path.txt)",
            fun: write_quit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write-quit!",
            alias: Some("wq!"),
            doc: "Writes changes to disk and closes the current view forcefully. Accepts an optional path (:wq! some/path.txt)",
            fun: force_write_quit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write-all",
            alias: Some("wa"),
            doc: "Writes changes from all views to disk.",
            fun: write_all,
            completer: None,
        },
        TypableCommand {
            name: "write-quit-all",
            alias: Some("wqa"),
            doc: "Writes changes from all views to disk and close all views.",
            fun: write_all_quit,
            completer: None,
        },
        TypableCommand {
            name: "write-quit-all!",
            alias: Some("wqa!"),
            doc: "Writes changes from all views to disk and close all views forcefully (ignoring unsaved changes).",
            fun: force_write_all_quit,
            completer: None,
        },
        TypableCommand {
            name: "quit-all",
            alias: Some("qa"),
            doc: "Close all views.",
            fun: quit_all,
            completer: None,
        },
        TypableCommand {
            name: "quit-all!",
            alias: Some("qa!"),
            doc: "Close all views forcefully (ignoring unsaved changes).",
            fun: force_quit_all,
            completer: None,
        },
        TypableCommand {
            name: "theme",
            alias: None,
            doc: "Change the theme of current view. Requires theme name as argument (:theme <name>)",
            fun: theme,
            completer: Some(completers::theme),
        },
        TypableCommand {
            name: "clipboard-yank",
            alias: None,
            doc: "Yank main selection into system clipboard.",
            fun: yank_main_selection_to_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-yank-join",
            alias: None,
            doc: "Yank joined selections into system clipboard. A separator can be provided as first argument. Default value is newline.", // FIXME: current UI can't display long doc.
            fun: yank_joined_to_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-yank",
            alias: None,
            doc: "Yank main selection into system primary clipboard.",
            fun: yank_main_selection_to_primary_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-yank-join",
            alias: None,
            doc: "Yank joined selections into system primary clipboard. A separator can be provided as first argument. Default value is newline.", // FIXME: current UI can't display long doc.
            fun: yank_joined_to_primary_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-after",
            alias: None,
            doc: "Paste system clipboard after selections.",
            fun: paste_clipboard_after,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-before",
            alias: None,
            doc: "Paste system clipboard before selections.",
            fun: paste_clipboard_before,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-replace",
            alias: None,
            doc: "Replace selections with content of system clipboard.",
            fun: replace_selections_with_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-paste-after",
            alias: None,
            doc: "Paste primary clipboard after selections.",
            fun: paste_primary_clipboard_after,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-paste-before",
            alias: None,
            doc: "Paste primary clipboard before selections.",
            fun: paste_primary_clipboard_before,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-paste-replace",
            alias: None,
            doc: "Replace selections with content of system primary clipboard.",
            fun: replace_selections_with_primary_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "show-clipboard-provider",
            alias: None,
            doc: "Show clipboard provider name in status bar.",
            fun: show_clipboard_provider,
            completer: None,
        },
        TypableCommand {
            name: "change-current-directory",
            alias: Some("cd"),
            doc: "Change the current working directory (:cd <dir>).",
            fun: change_current_directory,
            completer: Some(completers::directory),
        },
        TypableCommand {
            name: "show-directory",
            alias: Some("pwd"),
            doc: "Show the current working directory.",
            fun: show_current_directory,
            completer: None,
        },
        TypableCommand {
            name: "encoding",
            alias: None,
            doc: "Set encoding based on `https://encoding.spec.whatwg.org`",
            fun: set_encoding,
            completer: None,
        },
        TypableCommand {
            name: "reload",
            alias: None,
            doc: "Discard changes and reload from the source file.",
            fun: reload,
            completer: None,
        },
        TypableCommand {
            name: "tree-sitter-scopes",
            alias: None,
            doc: "Display tree sitter scopes, primarily for theming and development.",
            fun: tree_sitter_scopes,
            completer: None,
        }
    ];

pub static COMMANDS: Lazy<HashMap<&'static str, &'static TypableCommand>> = Lazy::new(|| {
    let mut map = HashMap::new();

    for cmd in TYPABLE_COMMAND_LIST {
        map.insert(cmd.name, cmd);
        if let Some(alias) = cmd.alias {
            map.insert(alias, cmd);
        }
    }

    map
});
