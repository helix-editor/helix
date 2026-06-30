//! Commands for the GitHub Copilot integration.
//!
//! Typed commands (`:copilot-signin`, `:copilot-signout`, `:copilot-status`,
//! `:copilot-toggle`) drive the connection lifecycle. The bindable static
//! commands (`copilot_request_completion`, `copilot_apply_completion`,
//! `copilot_dismiss_completion`) request, accept and dismiss the inline ghost
//! text suggestion stored on the focused [`Document`].

use std::sync::Arc;

use anyhow::Result;
use helix_core::command_line::Args;
use helix_core::indent::IndentStyle;
use helix_core::text_annotations::InlineAnnotation;
use helix_core::{Selection, Tendril, Transaction};
use helix_lsp::copilot::{self, FormattingOptions, InlineCompletionItem};
use helix_lsp::OffsetEncoding;
use helix_view::document::CopilotCompletion;
use helix_view::{DocumentId, Editor, ViewId};

use crate::compositor;
use crate::job::{self, Callback};
use crate::ui::PromptEvent;

use super::Context;

/// The position encoding Copilot expects (LSP defaults to UTF-16).
const ENCODING: OffsetEncoding = OffsetEncoding::Utf16;

fn editor_status(message: String) -> Callback {
    Callback::Editor(Box::new(move |editor: &mut Editor| {
        editor.set_status(message)
    }))
}

/// Spawn (and initialize) the Copilot language server if it is not already
/// running, returning a handle to the client. The started handle is stored on
/// the editor so subsequent commands reuse it.
async fn ensure_client(
    existing: Option<Arc<copilot::Client>>,
    command: String,
    args: Vec<String>,
    workspace: Option<std::path::PathBuf>,
) -> Result<Arc<copilot::Client>> {
    if let Some(client) = existing {
        return Ok(client);
    }

    let client = copilot::Client::start(&command, &args)?;
    client
        .initialize(
            "Helix",
            helix_loader::VERSION_AND_GIT_HASH,
            env!("CARGO_PKG_VERSION"),
            workspace.as_deref(),
        )
        .await?;

    let stored = client.clone();
    job::dispatch(move |editor, _| editor.copilot.set_client(stored)).await;

    Ok(client)
}

/// Snapshot of the editor configuration needed to reach the Copilot client.
fn client_context(editor: &Editor) -> (Option<Arc<copilot::Client>>, String, Vec<String>) {
    let config = editor.config();
    (
        editor.copilot.client(),
        config.copilot.command.clone(),
        config.copilot.args.clone(),
    )
}

fn workspace() -> Option<std::path::PathBuf> {
    Some(helix_loader::find_workspace().0)
}

// ---------------------------------------------------------------------------
// Typed commands
// ---------------------------------------------------------------------------

pub fn copilot_signin(cx: &mut compositor::Context, _args: Args, event: PromptEvent) -> Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (existing, command, args) = client_context(cx.editor);
    let workspace = workspace();

    cx.jobs.callback(async move {
        let client = ensure_client(existing, command, args, workspace).await?;

        let status = client.check_status().await?;
        if status.signed_in() {
            let user = status.user.unwrap_or_default();
            return Ok(Callback::Editor(Box::new(move |editor: &mut Editor| {
                editor.copilot.signed_in = true;
                editor.set_status(format!("Copilot: already signed in as {user}"));
            })));
        }

        let response = client.sign_in().await?;
        match response.status.as_str() {
            "PromptUserDeviceFlow" => {
                let code = response.user_code.unwrap_or_default();
                let uri = response
                    .verification_uri
                    .unwrap_or_else(|| "https://github.com/login/device".to_string());

                // Wait for the user to authorize the device code in the
                // background; this resolves only once they finish (or it times
                // out). It is reversible until the user actually authorizes.
                let waiter = client.clone();
                tokio::spawn(async move {
                    match waiter.finish_device_flow().await {
                        Ok(_) => {
                            job::dispatch(|editor, _| {
                                editor.copilot.signed_in = true;
                                editor.set_status("Copilot: signed in");
                            })
                            .await;
                        }
                        Err(err) => {
                            job::dispatch(move |editor, _| {
                                editor.set_error(format!("Copilot sign-in failed: {err}"));
                            })
                            .await;
                        }
                    }
                });

                Ok(editor_status(format!(
                    "Copilot: open {uri} and enter code {code}"
                )))
            }
            "AlreadySignedIn" => Ok(Callback::Editor(Box::new(|editor: &mut Editor| {
                editor.copilot.signed_in = true;
                editor.set_status("Copilot: already signed in");
            }))),
            other => Ok(editor_status(format!(
                "Copilot: unexpected sign-in status '{other}'"
            ))),
        }
    });

    Ok(())
}

pub fn copilot_signout(
    cx: &mut compositor::Context,
    _args: Args,
    event: PromptEvent,
) -> Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (existing, command, args) = client_context(cx.editor);
    let workspace = workspace();

    cx.jobs.callback(async move {
        let client = ensure_client(existing, command, args, workspace).await?;
        client.sign_out().await?;
        Ok(Callback::Editor(Box::new(|editor: &mut Editor| {
            editor.copilot.signed_in = false;
            editor.set_status("Copilot: signed out");
        })))
    });

    Ok(())
}

pub fn copilot_status(cx: &mut compositor::Context, _args: Args, event: PromptEvent) -> Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (existing, command, args) = client_context(cx.editor);
    let enabled = cx.editor.copilot.enabled;
    let workspace = workspace();

    cx.jobs.callback(async move {
        let client = ensure_client(existing, command, args, workspace).await?;
        let status = client.check_status().await?;
        let signed_in = status.signed_in();
        let state = if enabled { "enabled" } else { "disabled" };
        let message = if signed_in {
            let user = status.user.unwrap_or_default();
            format!("Copilot: signed in as {user} ({state})")
        } else {
            format!("Copilot: not signed in ({state}, run :copilot-signin)")
        };

        Ok(Callback::Editor(Box::new(move |editor: &mut Editor| {
            editor.copilot.signed_in = signed_in;
            editor.set_status(message);
        })))
    });

    Ok(())
}

pub fn copilot_toggle(cx: &mut compositor::Context, _args: Args, event: PromptEvent) -> Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let enabled = !cx.editor.copilot.enabled;
    cx.editor.copilot.enabled = enabled;

    if !enabled {
        // Clear any visible suggestion in the focused document.
        let doc = doc_mut!(cx.editor);
        doc.clear_copilot_completion();
        cx.editor.set_status("Copilot: disabled");
        return Ok(());
    }

    cx.editor.set_status("Copilot: enabled");

    // Eagerly start the server so the first suggestion is fast.
    if cx.editor.copilot.client().is_none() {
        let (existing, command, args) = client_context(cx.editor);
        let workspace = workspace();
        cx.jobs.callback(async move {
            ensure_client(existing, command, args, workspace).await?;
            Ok(editor_status("Copilot: ready".to_string()))
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Static (bindable) commands
// ---------------------------------------------------------------------------

/// Request an inline suggestion for the focused document at the cursor.
pub fn copilot_request_completion(cx: &mut Context) {
    if !cx.editor.copilot.enabled {
        cx.editor
            .set_status("Copilot is disabled (run :copilot-toggle to enable)");
        return;
    }

    let (existing, command, args) = client_context(cx.editor);
    let workspace = workspace();

    let (view, doc) = current!(cx.editor);
    let Some(uri) = doc.url() else {
        cx.editor
            .set_status("Copilot: the buffer must be saved to disk first");
        return;
    };

    let view_id = view.id;
    let doc_id = doc.id();
    let version = doc.version();
    let text = doc.text();
    let cursor = doc.selection(view_id).primary().cursor(text.slice(..));
    let lsp_pos = helix_lsp::util::pos_to_lsp_pos(text, cursor, ENCODING);
    let full_text = text.to_string();
    let language_id = doc.language_id().unwrap_or("plaintext").to_string();
    let formatting = FormattingOptions {
        tab_size: doc.tab_width() as u32,
        insert_spaces: matches!(doc.indent_style, IndentStyle::Spaces(_)),
    };
    let was_open = cx.editor.copilot.is_open(doc_id);

    cx.jobs.callback(async move {
        let client = ensure_client(existing, command, args, workspace).await?;

        if was_open {
            client.did_change(&uri, version, &full_text);
        } else {
            client.did_open(&uri, version, &language_id, &full_text);
            job::dispatch(move |editor, _| editor.copilot.mark_open(doc_id)).await;
        }

        let items = match client
            .inline_completion(&uri, version, lsp_pos.line, lsp_pos.character, formatting)
            .await
        {
            Ok(items) => items,
            Err(helix_lsp::Error::Rpc(err)) if err.code.code() == 1000 => {
                return Ok(editor_status(
                    "Copilot: not signed in (run :copilot-signin)".to_string(),
                ));
            }
            Err(err) => return Err(err.into()),
        };

        let Some(item) = items.into_iter().next() else {
            return Ok(editor_status("Copilot: no suggestion".to_string()));
        };

        if let Some(id) = item_id(&item) {
            client.notify_shown(&id);
        }

        Ok(Callback::Editor(Box::new(move |editor: &mut Editor| {
            store_suggestion(editor, doc_id, view_id, cursor, version, item);
        })))
    });
}

/// Accept the active inline suggestion, inserting its text.
pub fn copilot_apply_completion(cx: &mut Context) {
    let accepted = {
        let (view, doc) = current!(cx.editor);
        let Some(completion) = doc.copilot_completion().cloned() else {
            return;
        };
        if completion.view_id != view.id {
            return;
        }
        doc.clear_copilot_completion();

        let new_cursor = completion.range.start + completion.text.chars().count();
        let transaction = Transaction::change(
            doc.text(),
            std::iter::once((
                completion.range.start,
                completion.range.end,
                Some(Tendril::from(completion.text)),
            )),
        )
        .with_selection(Selection::point(new_cursor));
        doc.apply(&transaction, view.id);

        completion.item_id
    };

    if let (Some(id), Some(client)) = (accepted, cx.editor.copilot.client()) {
        client.notify_accepted(&id);
    }
}

/// Dismiss the active inline suggestion without inserting it.
pub fn copilot_dismiss_completion(cx: &mut Context) {
    let doc = doc_mut!(cx.editor);
    doc.clear_copilot_completion();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn item_id(item: &InlineCompletionItem) -> Option<String> {
    item.command
        .as_ref()
        .and_then(|command| command.arguments.as_ref())
        .and_then(|arguments| arguments.first())
        .and_then(|argument| argument.as_str())
        .map(|id| id.to_string())
}

/// Store a returned suggestion as ghost text on the document, provided it is
/// still up to date.
fn store_suggestion(
    editor: &mut Editor,
    doc_id: DocumentId,
    view_id: ViewId,
    cursor: usize,
    requested_version: i32,
    item: InlineCompletionItem,
) {
    let stored = {
        let Some(doc) = editor.documents.get_mut(&doc_id) else {
            return;
        };
        // Discard stale suggestions if the buffer changed in the meantime.
        if doc.version() != requested_version {
            return;
        }

        let text = doc.text();
        let range = match item.range {
            Some(range) => {
                let start =
                    helix_lsp::util::lsp_pos_to_pos(text, range.start, ENCODING).unwrap_or(cursor);
                let end =
                    helix_lsp::util::lsp_pos_to_pos(text, range.end, ENCODING).unwrap_or(cursor);
                start..end
            }
            None => cursor..cursor,
        };

        // The portion of the suggestion that has not been typed yet, shown as
        // ghost text after the cursor.
        let already_typed = cursor.saturating_sub(range.start);
        let ghost: String = item.insert_text.chars().skip(already_typed).collect();
        let first_line = ghost.split('\n').next().unwrap_or("").to_string();
        let display = if first_line.is_empty() {
            Vec::new()
        } else {
            vec![InlineAnnotation::new(cursor, first_line)]
        };

        let item_id = item_id(&item);
        doc.set_copilot_completion(CopilotCompletion {
            view_id,
            anchor: cursor,
            range,
            version: requested_version,
            text: item.insert_text,
            display,
            item_id,
        });
        true
    };

    if stored {
        editor.needs_redraw = true;
    }
}
