use std::{borrow::Cow, collections::HashSet, fmt, future::Future};

use futures_util::{stream::FuturesOrdered, FutureExt as _};
use helix_core::syntax::config::LanguageServerFeature;
use helix_lsp::{lsp, util::range_to_lsp_range, LanguageServerId};
use tokio_stream::StreamExt as _;

use crate::Editor;

/// A generic action against the editor.
///
/// This corresponds to the LSP code action feature. LSP code actions are implemented in terms of
/// `Action` but `Action` is generic and may be used for internal actions as well.
pub struct Action {
    title: Cow<'static, str>,
    priority: u8,
    action: Box<dyn Fn(&mut Editor) + Send + Sync + 'static>,
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CodeAction")
            .field("title", &self.title)
            .field("priority", &self.priority)
            .finish_non_exhaustive()
    }
}

impl Action {
    pub fn new<T: Into<Cow<'static, str>>, F: Fn(&mut Editor) + Send + Sync + 'static>(
        title: T,
        priority: u8,
        action: F,
    ) -> Self {
        Self {
            title: title.into(),
            priority,
            action: Box::new(action),
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn execute(&self, editor: &mut Editor) {
        (self.action)(editor);
    }

    fn lsp(server_id: LanguageServerId, action: lsp::CodeActionOrCommand) -> Self {
        let title = match &action {
            lsp::CodeActionOrCommand::CodeAction(action) => action.title.clone(),
            lsp::CodeActionOrCommand::Command(command) => command.title.clone(),
        };
        let priority = lsp_code_action_priority(&action);

        Self::new(title, priority, move |editor| {
            let Some(language_server) = editor.language_server_by_id(server_id) else {
                editor.set_error("Language Server disappeared");
                return;
            };
            let offset_encoding = language_server.offset_encoding();
            match &action {
                lsp::CodeActionOrCommand::Command(command) => {
                    log::debug!("code action command: {:?}", command);
                    editor.execute_lsp_command(command.clone(), server_id);
                }
                lsp::CodeActionOrCommand::CodeAction(code_action) => {
                    log::debug!("code action: {:?}", code_action);
                    // we support lsp "codeAction/resolve" for `edit` and `command` fields
                    let code_action = if code_action.edit.is_none() || code_action.command.is_none()
                    {
                        language_server
                            .resolve_code_action(code_action)
                            .and_then(|future| helix_lsp::block_on(future).ok())
                            .unwrap_or(code_action.clone())
                    } else {
                        code_action.clone()
                    };

                    if let Some(ref workspace_edit) = code_action.edit {
                        let _ = editor.apply_workspace_edit(offset_encoding, workspace_edit);
                    }

                    // if code action provides both edit and command first the edit
                    // should be applied and then the command
                    if let Some(command) = code_action.command {
                        editor.execute_lsp_command(command, server_id);
                    }
                }
            }
        })
    }
}

/// Computes a priority score for LSP code actions.
///
/// This roughly matches how VSCode should behave: <https://github.com/microsoft/vscode/blob/eaec601dd69aeb4abb63b9601a6f44308c8d8c6e/src/vs/editor/contrib/codeAction/browser/codeActionWidget.ts>.
/// The scoring is basically equivalent to comparing code actions by:
/// `(category, fixes_diagnostic, is_preferred)`. First code actions are sorted by the category
/// declared on the `kind` field (if present), then whether the action fixes a diagnostic and then
/// whether it is marked as `is_preferred`.
fn lsp_code_action_priority(action: &lsp::CodeActionOrCommand) -> u8 {
    // The `kind` field is defined as open ended in the LSP spec - any value may be used. In
    // practice a closed set of common values (mostly suggested in the LSP spec) are used.
    // VSCode displays these components as menu headers. We don't do the same but we aim to sort
    // the code actions in the same way.
    let category = if let lsp::CodeActionOrCommand::CodeAction(lsp::CodeAction {
        kind: Some(kind),
        ..
    }) = action
    {
        let mut components = kind.as_str().split('.');
        match components.next() {
            Some("quickfix") => 7,
            Some("refactor") => match components.next() {
                Some("extract") => 6,
                Some("inline") => 5,
                Some("rewrite") => 4,
                Some("move") => 3,
                Some("surround") => 2,
                _ => 1,
            },
            Some("source") => 1,
            _ => 0,
        }
    } else {
        0
    };
    let fixes_diagnostic = matches!(
        action,
        lsp::CodeActionOrCommand::CodeAction(lsp::CodeAction {
            diagnostics: Some(diagnostics),
            ..
        }) if !diagnostics.is_empty()
    );
    let is_preferred = matches!(
        action,
        lsp::CodeActionOrCommand::CodeAction(lsp::CodeAction {
            is_preferred: Some(true),
            ..
        })
    );

    // The constants here weigh the three criteria so that their scores can't overlap:
    // two code actions in the same category should be sorted closer than code actions in
    // separate categories. `fixes_diagnostic` and `is_preferred` break ties.
    let mut priority = category * 4;
    if fixes_diagnostic {
        priority += 2;
    }
    if is_preferred {
        priority += 1;
    }
    priority
}

impl Editor {
    /// Finds the available actions given the current selection range.
    pub fn actions(&self) -> Option<impl Future<Output = Vec<Action>>> {
        let (view, doc) = current_ref!(self);
        let selection = doc.selection(view.id).primary();
        let mut seen_language_servers = HashSet::new();

        let mut futures: FuturesOrdered<_> = doc
            .language_servers_with_feature(LanguageServerFeature::CodeAction)
            .filter(|ls| seen_language_servers.insert(ls.id()))
            .map(|language_server| {
                let offset_encoding = language_server.offset_encoding();
                let language_server_id = language_server.id();
                let range = range_to_lsp_range(doc.text(), selection, offset_encoding);
                // Filter and convert overlapping diagnostics
                let context = lsp::CodeActionContext {
                    diagnostics: doc
                        .diagnostics()
                        .iter()
                        .filter(|&diag| {
                            diag.inner.provider.language_server_id() == Some(language_server_id)
                                && selection.overlaps(&helix_core::Range::new(
                                    diag.range.start,
                                    diag.range.end,
                                ))
                        })
                        .map(|diag| diag.inner.to_lsp_diagnostic(doc.text(), offset_encoding))
                        .collect(),
                    only: None,
                    trigger_kind: Some(lsp::CodeActionTriggerKind::INVOKED),
                };
                let future = language_server
                    .code_actions(doc.identifier(), range, context)
                    .unwrap();
                async move {
                    let Some(actions) = future.await? else {
                        return anyhow::Ok(Vec::new());
                    };

                    let actions: Vec<_> = actions
                        .into_iter()
                        .filter(|action| {
                            // remove disabled code actions
                            matches!(
                                action,
                                lsp::CodeActionOrCommand::Command(_)
                                    | lsp::CodeActionOrCommand::CodeAction(lsp::CodeAction {
                                        disabled: None,
                                        ..
                                    })
                            )
                        })
                        .map(move |action| Action::lsp(language_server_id, action))
                        .collect();

                    Ok(actions)
                }
                .boxed()
            })
            .chain(self.spelling_actions())
            .collect();

        if futures.is_empty() {
            return None;
        }

        Some(async move {
            let mut actions = Vec::new();
            while let Some(response) = futures.next().await {
                match response {
                    Ok(mut items) => actions.append(&mut items),
                    Err(err) => log::error!("Error requesting code actions: {err}"),
                }
            }
            actions.sort_by_key(|action| std::cmp::Reverse(action.priority));
            actions
        })
    }
}
