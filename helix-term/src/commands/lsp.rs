use futures_util::{future::BoxFuture, stream::FuturesUnordered, FutureExt};
use helix_lsp::{
    block_on,
    lsp::{
        self, CodeAction, CodeActionOrCommand, CodeActionTriggerKind, DiagnosticSeverity,
        NumberOrString,
    },
    util::{diagnostic_to_lsp_diagnostic, lsp_range_to_range, range_to_lsp_range},
    Client, OffsetEncoding,
};
use serde_json::Value;
use tokio_stream::StreamExt;
use tui::{
    text::{Span, Spans},
    widgets::Row,
};

use super::{align_view, push_jump, Align, Context, Editor, Open};

use helix_core::{
    path, syntax::LanguageServerFeature, text_annotations::InlineAnnotation, Selection,
};
use helix_view::{
    document::{DocumentInlayHints, DocumentInlayHintsId, Mode},
    editor::Action,
    graphics::Margin,
    theme::Style,
    Document, View,
};

use crate::{
    compositor::{self, Compositor},
    job::Callback,
    ui::{
        self, lsp::SignatureHelp, overlay::overlaid, DynamicPicker, FileLocation, Picker, Popup,
        PromptEvent,
    },
};

use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashSet},
    fmt::Write,
    future::Future,
    path::PathBuf,
    sync::Arc,
};

/// Gets the first language server that is attached to a document which supports a specific feature.
/// If there is no configured language server that supports the feature, this displays a status message.
/// Using this macro in a context where the editor automatically queries the LSP
/// (instead of when the user explicitly does so via a keybind like `gd`)
/// will spam the "No configured language server supports \<feature>" status message confusingly.
#[macro_export]
macro_rules! language_server_with_feature {
    ($editor:expr, $doc:expr, $feature:expr) => {{
        let language_server = $doc.language_servers_with_feature($feature).next();
        match language_server {
            Some(language_server) => language_server,
            None => {
                $editor.set_status(format!(
                    "No configured language server supports {}",
                    $feature
                ));
                return;
            }
        }
    }};
}

impl ui::menu::Item for lsp::Location {
    /// Current working directory.
    type Data = PathBuf;

    fn format(&self, cwdir: &Self::Data) -> Row {
        // The preallocation here will overallocate a few characters since it will account for the
        // URL's scheme, which is not used most of the time since that scheme will be "file://".
        // Those extra chars will be used to avoid allocating when writing the line number (in the
        // common case where it has 5 digits or less, which should be enough for a cast majority
        // of usages).
        let mut res = String::with_capacity(self.uri.as_str().len());

        if self.uri.scheme() == "file" {
            // With the preallocation above and UTF-8 paths already, this closure will do one (1)
            // allocation, for `to_file_path`, else there will be two (2), with `to_string_lossy`.
            let mut write_path_to_res = || -> Option<()> {
                let path = self.uri.to_file_path().ok()?;
                res.push_str(&path.strip_prefix(cwdir).unwrap_or(&path).to_string_lossy());
                Some(())
            };
            write_path_to_res();
        } else {
            // Never allocates since we declared the string with this capacity already.
            res.push_str(self.uri.as_str());
        }

        // Most commonly, this will not allocate, especially on Unix systems where the root prefix
        // is a simple `/` and not `C:\` (with whatever drive letter)
        write!(&mut res, ":{}", self.range.start.line + 1)
            .expect("Will only failed if allocating fail");
        res.into()
    }
}

struct SymbolInformationItem {
    symbol: lsp::SymbolInformation,
    offset_encoding: OffsetEncoding,
}

impl ui::menu::Item for SymbolInformationItem {
    /// Path to currently focussed document
    type Data = Option<lsp::Url>;

    fn format(&self, current_doc_path: &Self::Data) -> Row {
        if current_doc_path.as_ref() == Some(&self.symbol.location.uri) {
            self.symbol.name.as_str().into()
        } else {
            match self.symbol.location.uri.to_file_path() {
                Ok(path) => {
                    let get_relative_path = path::get_relative_path(path.as_path());
                    format!(
                        "{} ({})",
                        &self.symbol.name,
                        get_relative_path.to_string_lossy()
                    )
                    .into()
                }
                Err(_) => format!("{} ({})", &self.symbol.name, &self.symbol.location.uri).into(),
            }
        }
    }
}

struct DiagnosticStyles {
    hint: Style,
    info: Style,
    warning: Style,
    error: Style,
}

struct PickerDiagnostic {
    url: lsp::Url,
    diag: lsp::Diagnostic,
    offset_encoding: OffsetEncoding,
}

impl ui::menu::Item for PickerDiagnostic {
    type Data = (DiagnosticStyles, DiagnosticsFormat);

    fn format(&self, (styles, format): &Self::Data) -> Row {
        let mut style = self
            .diag
            .severity
            .map(|s| match s {
                DiagnosticSeverity::HINT => styles.hint,
                DiagnosticSeverity::INFORMATION => styles.info,
                DiagnosticSeverity::WARNING => styles.warning,
                DiagnosticSeverity::ERROR => styles.error,
                _ => Style::default(),
            })
            .unwrap_or_default();

        // remove background as it is distracting in the picker list
        style.bg = None;

        let code = match self.diag.code.as_ref() {
            Some(NumberOrString::Number(n)) => format!(" ({n})"),
            Some(NumberOrString::String(s)) => format!(" ({s})"),
            None => String::new(),
        };

        let path = match format {
            DiagnosticsFormat::HideSourcePath => String::new(),
            DiagnosticsFormat::ShowSourcePath => {
                let file_path = self.url.to_file_path().unwrap();
                let path = path::get_truncated_path(file_path);
                format!("{}: ", path.to_string_lossy())
            }
        };

        Spans::from(vec![
            Span::raw(path),
            Span::styled(&self.diag.message, style),
            Span::styled(code, style),
        ])
        .into()
    }
}

fn location_to_file_location(location: &lsp::Location) -> FileLocation {
    let path = location.uri.to_file_path().unwrap();
    let line = Some((
        location.range.start.line as usize,
        location.range.end.line as usize,
    ));
    (path.into(), line)
}

fn jump_to_location(
    editor: &mut Editor,
    location: &lsp::Location,
    offset_encoding: OffsetEncoding,
    action: Action,
) {
    let (view, doc) = current!(editor);
    push_jump(view, doc);

    let path = match location.uri.to_file_path() {
        Ok(path) => path,
        Err(_) => {
            let err = format!("unable to convert URI to filepath: {}", location.uri);
            editor.set_error(err);
            return;
        }
    };

    let doc = match editor.open(&path, action) {
        Ok(id) => doc_mut!(editor, &id),
        Err(err) => {
            let err = format!("failed to open path: {:?}: {:?}", location.uri, err);
            editor.set_error(err);
            return;
        }
    };
    let view = view_mut!(editor);
    // TODO: convert inside server
    let new_range =
        if let Some(new_range) = lsp_range_to_range(doc.text(), location.range, offset_encoding) {
            new_range
        } else {
            log::warn!("lsp position out of bounds - {:?}", location.range);
            return;
        };
    // we flip the range so that the cursor sits on the start of the symbol
    // (for example start of the function).
    doc.set_selection(view.id, Selection::single(new_range.head, new_range.anchor));
    if action.align_view(view, doc.id()) {
        align_view(doc, view, Align::Center);
    }
}

type SymbolPicker = Picker<SymbolInformationItem>;

fn sym_picker(symbols: Vec<SymbolInformationItem>, current_path: Option<lsp::Url>) -> SymbolPicker {
    // TODO: drop current_path comparison and instead use workspace: bool flag?
    Picker::new(symbols, current_path, move |cx, item, action| {
        jump_to_location(
            cx.editor,
            &item.symbol.location,
            item.offset_encoding,
            action,
        );
    })
    .with_preview(move |_editor, item| Some(location_to_file_location(&item.symbol.location)))
    .truncate_start(false)
}

#[derive(Copy, Clone, PartialEq)]
enum DiagnosticsFormat {
    ShowSourcePath,
    HideSourcePath,
}

fn diag_picker(
    cx: &Context,
    diagnostics: BTreeMap<lsp::Url, Vec<(lsp::Diagnostic, usize)>>,
    _current_path: Option<lsp::Url>,
    format: DiagnosticsFormat,
) -> Picker<PickerDiagnostic> {
    // TODO: drop current_path comparison and instead use workspace: bool flag?

    // flatten the map to a vec of (url, diag) pairs
    let mut flat_diag = Vec::new();
    for (url, diags) in diagnostics {
        flat_diag.reserve(diags.len());

        for (diag, ls) in diags {
            if let Some(ls) = cx.editor.language_server_by_id(ls) {
                flat_diag.push(PickerDiagnostic {
                    url: url.clone(),
                    diag,
                    offset_encoding: ls.offset_encoding(),
                });
            }
        }
    }

    let styles = DiagnosticStyles {
        hint: cx.editor.theme.get("hint"),
        info: cx.editor.theme.get("info"),
        warning: cx.editor.theme.get("warning"),
        error: cx.editor.theme.get("error"),
    };

    Picker::new(
        flat_diag,
        (styles, format),
        move |cx,
              PickerDiagnostic {
                  url,
                  diag,
                  offset_encoding,
              },
              action| {
            jump_to_location(
                cx.editor,
                &lsp::Location::new(url.clone(), diag.range),
                *offset_encoding,
                action,
            )
        },
    )
    .with_preview(move |_editor, PickerDiagnostic { url, diag, .. }| {
        let location = lsp::Location::new(url.clone(), diag.range);
        Some(location_to_file_location(&location))
    })
    .truncate_start(false)
}

pub fn symbol_picker(cx: &mut Context) {
    fn nested_to_flat(
        list: &mut Vec<SymbolInformationItem>,
        file: &lsp::TextDocumentIdentifier,
        symbol: lsp::DocumentSymbol,
        offset_encoding: OffsetEncoding,
    ) {
        #[allow(deprecated)]
        list.push(SymbolInformationItem {
            symbol: lsp::SymbolInformation {
                name: symbol.name,
                kind: symbol.kind,
                tags: symbol.tags,
                deprecated: symbol.deprecated,
                location: lsp::Location::new(file.uri.clone(), symbol.selection_range),
                container_name: None,
            },
            offset_encoding,
        });
        for child in symbol.children.into_iter().flatten() {
            nested_to_flat(list, file, child, offset_encoding);
        }
    }
    let doc = doc!(cx.editor);

    let mut seen_language_servers = HashSet::new();

    let mut futures: FuturesUnordered<_> = doc
        .language_servers_with_feature(LanguageServerFeature::DocumentSymbols)
        .filter(|ls| seen_language_servers.insert(ls.id()))
        .map(|language_server| {
            let request = language_server.document_symbols(doc.identifier()).unwrap();
            let offset_encoding = language_server.offset_encoding();
            let doc_id = doc.identifier();

            async move {
                let json = request.await?;
                let response: Option<lsp::DocumentSymbolResponse> = serde_json::from_value(json)?;
                let symbols = match response {
                    Some(symbols) => symbols,
                    None => return anyhow::Ok(vec![]),
                };
                // lsp has two ways to represent symbols (flat/nested)
                // convert the nested variant to flat, so that we have a homogeneous list
                let symbols = match symbols {
                    lsp::DocumentSymbolResponse::Flat(symbols) => symbols
                        .into_iter()
                        .map(|symbol| SymbolInformationItem {
                            symbol,
                            offset_encoding,
                        })
                        .collect(),
                    lsp::DocumentSymbolResponse::Nested(symbols) => {
                        let mut flat_symbols = Vec::new();
                        for symbol in symbols {
                            nested_to_flat(&mut flat_symbols, &doc_id, symbol, offset_encoding)
                        }
                        flat_symbols
                    }
                };
                Ok(symbols)
            }
        })
        .collect();
    let current_url = doc.url();

    if futures.is_empty() {
        cx.editor
            .set_error("No configured language server supports document symbols");
        return;
    }

    cx.jobs.callback(async move {
        let mut symbols = Vec::new();
        // TODO if one symbol request errors, all other requests are discarded (even if they're valid)
        while let Some(mut lsp_items) = futures.try_next().await? {
            symbols.append(&mut lsp_items);
        }
        let call = move |_editor: &mut Editor, compositor: &mut Compositor| {
            let picker = sym_picker(symbols, current_url);
            compositor.push(Box::new(overlaid(picker)))
        };

        Ok(Callback::EditorCompositor(Box::new(call)))
    });
}

pub fn workspace_symbol_picker(cx: &mut Context) {
    let doc = doc!(cx.editor);
    if doc
        .language_servers_with_feature(LanguageServerFeature::WorkspaceSymbols)
        .count()
        == 0
    {
        cx.editor
            .set_error("No configured language server supports workspace symbols");
        return;
    }

    let get_symbols = move |pattern: String, editor: &mut Editor| {
        let doc = doc!(editor);
        let mut seen_language_servers = HashSet::new();
        let mut futures: FuturesUnordered<_> = doc
            .language_servers_with_feature(LanguageServerFeature::WorkspaceSymbols)
            .filter(|ls| seen_language_servers.insert(ls.id()))
            .map(|language_server| {
                let request = language_server.workspace_symbols(pattern.clone()).unwrap();
                let offset_encoding = language_server.offset_encoding();
                async move {
                    let json = request.await?;

                    let response =
                        serde_json::from_value::<Option<Vec<lsp::SymbolInformation>>>(json)?
                            .unwrap_or_default()
                            .into_iter()
                            .map(|symbol| SymbolInformationItem {
                                symbol,
                                offset_encoding,
                            })
                            .collect();

                    anyhow::Ok(response)
                }
            })
            .collect();

        if futures.is_empty() {
            editor.set_error("No configured language server supports workspace symbols");
        }

        async move {
            let mut symbols = Vec::new();
            // TODO if one symbol request errors, all other requests are discarded (even if they're valid)
            while let Some(mut lsp_items) = futures.try_next().await? {
                symbols.append(&mut lsp_items);
            }
            anyhow::Ok(symbols)
        }
        .boxed()
    };

    let current_url = doc.url();
    let initial_symbols = get_symbols("".to_owned(), cx.editor);

    cx.jobs.callback(async move {
        let symbols = initial_symbols.await?;
        let call = move |_editor: &mut Editor, compositor: &mut Compositor| {
            let picker = sym_picker(symbols, current_url);
            let dyn_picker = DynamicPicker::new(picker, Box::new(get_symbols));
            compositor.push(Box::new(overlaid(dyn_picker)))
        };

        Ok(Callback::EditorCompositor(Box::new(call)))
    });
}

pub fn diagnostics_picker(cx: &mut Context) {
    let doc = doc!(cx.editor);
    if let Some(current_url) = doc.url() {
        let diagnostics = cx
            .editor
            .diagnostics
            .get(&current_url)
            .cloned()
            .unwrap_or_default();
        let picker = diag_picker(
            cx,
            [(current_url.clone(), diagnostics)].into(),
            Some(current_url),
            DiagnosticsFormat::HideSourcePath,
        );
        cx.push_layer(Box::new(overlaid(picker)));
    }
}

pub fn workspace_diagnostics_picker(cx: &mut Context) {
    let doc = doc!(cx.editor);
    let current_url = doc.url();
    // TODO not yet filtered by LanguageServerFeature, need to do something similar as Document::shown_diagnostics here for all open documents
    let diagnostics = cx.editor.diagnostics.clone();
    let picker = diag_picker(
        cx,
        diagnostics,
        current_url,
        DiagnosticsFormat::ShowSourcePath,
    );
    cx.push_layer(Box::new(overlaid(picker)));
}

struct CodeActionOrCommandItem {
    lsp_item: lsp::CodeActionOrCommand,
    language_server_id: usize,
}

impl ui::menu::Item for CodeActionOrCommandItem {
    type Data = ();
    fn format(&self, _data: &Self::Data) -> Row {
        match &self.lsp_item {
            lsp::CodeActionOrCommand::CodeAction(action) => action.title.as_str().into(),
            lsp::CodeActionOrCommand::Command(command) => command.title.as_str().into(),
        }
    }
}

/// Determines the category of the `CodeAction` using the `CodeAction::kind` field.
/// Returns a number that represent these categories.
/// Categories with a lower number should be displayed first.
///
///
/// While the `kind` field is defined as open ended in the LSP spec (any value may be used)
/// in practice a closed set of common values (mostly suggested in the LSP spec) are used.
/// VSCode displays each of these categories separately (separated by a heading in the codeactions picker)
/// to make them easier to navigate. Helix does not display these  headings to the user.
/// However it does sort code actions by their categories to achieve the same order as the VScode picker,
/// just without the headings.
///
/// The order used here is modeled after the [vscode sourcecode](https://github.com/microsoft/vscode/blob/eaec601dd69aeb4abb63b9601a6f44308c8d8c6e/src/vs/editor/contrib/codeAction/browser/codeActionWidget.ts>)
fn action_category(action: &CodeActionOrCommand) -> u32 {
    if let CodeActionOrCommand::CodeAction(CodeAction {
        kind: Some(kind), ..
    }) = action
    {
        let mut components = kind.as_str().split('.');
        match components.next() {
            Some("quickfix") => 0,
            Some("refactor") => match components.next() {
                Some("extract") => 1,
                Some("inline") => 2,
                Some("rewrite") => 3,
                Some("move") => 4,
                Some("surround") => 5,
                _ => 7,
            },
            Some("source") => 6,
            _ => 7,
        }
    } else {
        7
    }
}

fn action_preferred(action: &CodeActionOrCommand) -> bool {
    matches!(
        action,
        CodeActionOrCommand::CodeAction(CodeAction {
            is_preferred: Some(true),
            ..
        })
    )
}

fn action_fixes_diagnostics(action: &CodeActionOrCommand) -> bool {
    matches!(
        action,
        CodeActionOrCommand::CodeAction(CodeAction {
            diagnostics: Some(diagnostics),
            ..
        }) if !diagnostics.is_empty()
    )
}

pub fn code_action(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let selection_range = doc.selection(view.id).primary();

    let mut seen_language_servers = HashSet::new();

    let mut futures: FuturesUnordered<_> = doc
        .language_servers_with_feature(LanguageServerFeature::CodeAction)
        .filter(|ls| seen_language_servers.insert(ls.id()))
        // TODO this should probably already been filtered in something like "language_servers_with_feature"
        .filter_map(|language_server| {
            let offset_encoding = language_server.offset_encoding();
            let language_server_id = language_server.id();
            let range = range_to_lsp_range(doc.text(), selection_range, offset_encoding);
            // Filter and convert overlapping diagnostics
            let code_action_context = lsp::CodeActionContext {
                diagnostics: doc
                    .diagnostics()
                    .iter()
                    .filter(|&diag| {
                        selection_range
                            .overlaps(&helix_core::Range::new(diag.range.start, diag.range.end))
                    })
                    .map(|diag| diagnostic_to_lsp_diagnostic(doc.text(), diag, offset_encoding))
                    .collect(),
                only: None,
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            };
            let code_action_request =
                language_server.code_actions(doc.identifier(), range, code_action_context)?;
            Some((code_action_request, language_server_id))
        })
        .map(|(request, ls_id)| async move {
            let json = request.await?;
            let response: Option<lsp::CodeActionResponse> = serde_json::from_value(json)?;
            let mut actions = match response {
                Some(a) => a,
                None => return anyhow::Ok(Vec::new()),
            };

            // remove disabled code actions
            actions.retain(|action| {
                matches!(
                    action,
                    CodeActionOrCommand::Command(_)
                        | CodeActionOrCommand::CodeAction(CodeAction { disabled: None, .. })
                )
            });

            // Sort codeactions into a useful order. This behaviour is only partially described in the LSP spec.
            // Many details are modeled after vscode because language servers are usually tested against it.
            // VScode sorts the codeaction two times:
            //
            // First the codeactions that fix some diagnostics are moved to the front.
            // If both codeactions fix some diagnostics (or both fix none) the codeaction
            // that is marked with `is_preferred` is shown first. The codeactions are then shown in separate
            // submenus that only contain a certain category (see `action_category`) of actions.
            //
            // Below this done in in a single sorting step
            actions.sort_by(|action1, action2| {
                // sort actions by category
                let order = action_category(action1).cmp(&action_category(action2));
                if order != Ordering::Equal {
                    return order;
                }
                // within the categories sort by relevancy.
                // Modeled after the `codeActionsComparator` function in vscode:
                // https://github.com/microsoft/vscode/blob/eaec601dd69aeb4abb63b9601a6f44308c8d8c6e/src/vs/editor/contrib/codeAction/browser/codeAction.ts

                // if one code action fixes a diagnostic but the other one doesn't show it first
                let order = action_fixes_diagnostics(action1)
                    .cmp(&action_fixes_diagnostics(action2))
                    .reverse();
                if order != Ordering::Equal {
                    return order;
                }

                // if one of the codeactions is marked as preferred show it first
                // otherwise keep the original LSP sorting
                action_preferred(action1)
                    .cmp(&action_preferred(action2))
                    .reverse()
            });

            Ok(actions
                .into_iter()
                .map(|lsp_item| CodeActionOrCommandItem {
                    lsp_item,
                    language_server_id: ls_id,
                })
                .collect())
        })
        .collect();

    if futures.is_empty() {
        cx.editor
            .set_error("No configured language server supports code actions");
        return;
    }

    cx.jobs.callback(async move {
        let mut actions = Vec::new();
        // TODO if one code action request errors, all other requests are ignored (even if they're valid)
        while let Some(mut lsp_items) = futures.try_next().await? {
            actions.append(&mut lsp_items);
        }

        let call = move |editor: &mut Editor, compositor: &mut Compositor| {
            if actions.is_empty() {
                editor.set_error("No code actions available");
                return;
            }
            let mut picker = ui::Menu::new(actions, (), move |editor, action, event| {
                if event != PromptEvent::Validate {
                    return;
                }

                // always present here
                let action = action.unwrap();
                let Some(language_server) = editor.language_server_by_id(action.language_server_id)
                else {
                    editor.set_error("Language Server disappeared");
                    return;
                };
                let offset_encoding = language_server.offset_encoding();

                match &action.lsp_item {
                    lsp::CodeActionOrCommand::Command(command) => {
                        log::debug!("code action command: {:?}", command);
                        execute_lsp_command(editor, action.language_server_id, command.clone());
                    }
                    lsp::CodeActionOrCommand::CodeAction(code_action) => {
                        log::debug!("code action: {:?}", code_action);
                        // we support lsp "codeAction/resolve" for `edit` and `command` fields
                        let mut resolved_code_action = None;
                        if code_action.edit.is_none() || code_action.command.is_none() {
                            if let Some(future) =
                                language_server.resolve_code_action(code_action.clone())
                            {
                                if let Ok(response) = helix_lsp::block_on(future) {
                                    if let Ok(code_action) =
                                        serde_json::from_value::<CodeAction>(response)
                                    {
                                        resolved_code_action = Some(code_action);
                                    }
                                }
                            }
                        }
                        let resolved_code_action =
                            resolved_code_action.as_ref().unwrap_or(code_action);

                        if let Some(ref workspace_edit) = resolved_code_action.edit {
                            log::debug!("edit: {:?}", workspace_edit);
                            let _ = apply_workspace_edit(editor, offset_encoding, workspace_edit);
                        }

                        // if code action provides both edit and command first the edit
                        // should be applied and then the command
                        if let Some(command) = &code_action.command {
                            execute_lsp_command(editor, action.language_server_id, command.clone());
                        }
                    }
                }
            });
            picker.move_down(); // pre-select the first item

            let margin = if editor.menu_border() {
                Margin::vertical(1)
            } else {
                Margin::none()
            };

            let popup = Popup::new("code-action", picker)
                .with_scrollbar(false)
                .margin(margin);

            compositor.replace_or_push("code-action", popup);
        };

        Ok(Callback::EditorCompositor(Box::new(call)))
    });
}

impl ui::menu::Item for lsp::Command {
    type Data = ();
    fn format(&self, _data: &Self::Data) -> Row {
        self.title.as_str().into()
    }
}

pub fn execute_lsp_command(editor: &mut Editor, language_server_id: usize, cmd: lsp::Command) {
    // the command is executed on the server and communicated back
    // to the client asynchronously using workspace edits
    let future = match editor
        .language_server_by_id(language_server_id)
        .and_then(|language_server| language_server.command(cmd))
    {
        Some(future) => future,
        None => {
            editor.set_error("Language server does not support executing commands");
            return;
        }
    };

    tokio::spawn(async move {
        let res = future.await;

        if let Err(e) = res {
            log::error!("execute LSP command: {}", e);
        }
    });
}

pub fn apply_document_resource_op(op: &lsp::ResourceOp) -> std::io::Result<()> {
    use lsp::ResourceOp;
    use std::fs;
    match op {
        ResourceOp::Create(op) => {
            let path = op.uri.to_file_path().unwrap();
            let ignore_if_exists = op.options.as_ref().map_or(false, |options| {
                !options.overwrite.unwrap_or(false) && options.ignore_if_exists.unwrap_or(false)
            });
            if ignore_if_exists && path.exists() {
                Ok(())
            } else {
                // Create directory if it does not exist
                if let Some(dir) = path.parent() {
                    if !dir.is_dir() {
                        fs::create_dir_all(dir)?;
                    }
                }

                fs::write(&path, [])
            }
        }
        ResourceOp::Delete(op) => {
            let path = op.uri.to_file_path().unwrap();
            if path.is_dir() {
                let recursive = op
                    .options
                    .as_ref()
                    .and_then(|options| options.recursive)
                    .unwrap_or(false);

                if recursive {
                    fs::remove_dir_all(&path)
                } else {
                    fs::remove_dir(&path)
                }
            } else if path.is_file() {
                fs::remove_file(&path)
            } else {
                Ok(())
            }
        }
        ResourceOp::Rename(op) => {
            let from = op.old_uri.to_file_path().unwrap();
            let to = op.new_uri.to_file_path().unwrap();
            let ignore_if_exists = op.options.as_ref().map_or(false, |options| {
                !options.overwrite.unwrap_or(false) && options.ignore_if_exists.unwrap_or(false)
            });
            if ignore_if_exists && to.exists() {
                Ok(())
            } else {
                fs::rename(from, &to)
            }
        }
    }
}

#[derive(Debug)]
pub struct ApplyEditError {
    pub kind: ApplyEditErrorKind,
    pub failed_change_idx: usize,
}

#[derive(Debug)]
pub enum ApplyEditErrorKind {
    DocumentChanged,
    FileNotFound,
    UnknownURISchema,
    IoError(std::io::Error),
    // TODO: check edits before applying and propagate failure
    // InvalidEdit,
}

impl ToString for ApplyEditErrorKind {
    fn to_string(&self) -> String {
        match self {
            ApplyEditErrorKind::DocumentChanged => "document has changed".to_string(),
            ApplyEditErrorKind::FileNotFound => "file not found".to_string(),
            ApplyEditErrorKind::UnknownURISchema => "URI schema not supported".to_string(),
            ApplyEditErrorKind::IoError(err) => err.to_string(),
        }
    }
}

///TODO make this transactional (and set failureMode to transactional)
pub fn apply_workspace_edit(
    editor: &mut Editor,
    offset_encoding: OffsetEncoding,
    workspace_edit: &lsp::WorkspaceEdit,
) -> Result<(), ApplyEditError> {
    let mut apply_edits = |uri: &helix_lsp::Url,
                           version: Option<i32>,
                           text_edits: Vec<lsp::TextEdit>|
     -> Result<(), ApplyEditErrorKind> {
        let path = match uri.to_file_path() {
            Ok(path) => path,
            Err(_) => {
                let err = format!("unable to convert URI to filepath: {}", uri);
                log::error!("{}", err);
                editor.set_error(err);
                return Err(ApplyEditErrorKind::UnknownURISchema);
            }
        };

        let current_view_id = view!(editor).id;
        let doc_id = match editor.open(&path, Action::Load) {
            Ok(doc_id) => doc_id,
            Err(err) => {
                let err = format!("failed to open document: {}: {}", uri, err);
                log::error!("{}", err);
                editor.set_error(err);
                return Err(ApplyEditErrorKind::FileNotFound);
            }
        };

        let doc = doc_mut!(editor, &doc_id);
        if let Some(version) = version {
            if version != doc.version() {
                let err = format!("outdated workspace edit for {path:?}");
                log::error!("{err}, expected {} but got {version}", doc.version());
                editor.set_error(err);
                return Err(ApplyEditErrorKind::DocumentChanged);
            }
        }

        // Need to determine a view for apply/append_changes_to_history
        let selections = doc.selections();
        let view_id = if selections.contains_key(&current_view_id) {
            // use current if possible
            current_view_id
        } else {
            // Hack: we take the first available view_id
            selections
                .keys()
                .next()
                .copied()
                .expect("No view_id available")
        };

        let transaction = helix_lsp::util::generate_transaction_from_edits(
            doc.text(),
            text_edits,
            offset_encoding,
        );
        let view = view_mut!(editor, view_id);
        doc.apply(&transaction, view.id);
        doc.append_changes_to_history(view);
        Ok(())
    };

    if let Some(ref document_changes) = workspace_edit.document_changes {
        match document_changes {
            lsp::DocumentChanges::Edits(document_edits) => {
                for (i, document_edit) in document_edits.iter().enumerate() {
                    let edits = document_edit
                        .edits
                        .iter()
                        .map(|edit| match edit {
                            lsp::OneOf::Left(text_edit) => text_edit,
                            lsp::OneOf::Right(annotated_text_edit) => {
                                &annotated_text_edit.text_edit
                            }
                        })
                        .cloned()
                        .collect();
                    apply_edits(
                        &document_edit.text_document.uri,
                        document_edit.text_document.version,
                        edits,
                    )
                    .map_err(|kind| ApplyEditError {
                        kind,
                        failed_change_idx: i,
                    })?;
                }
            }
            lsp::DocumentChanges::Operations(operations) => {
                log::debug!("document changes - operations: {:?}", operations);
                for (i, operation) in operations.iter().enumerate() {
                    match operation {
                        lsp::DocumentChangeOperation::Op(op) => {
                            apply_document_resource_op(op).map_err(|io| ApplyEditError {
                                kind: ApplyEditErrorKind::IoError(io),
                                failed_change_idx: i,
                            })?;
                        }

                        lsp::DocumentChangeOperation::Edit(document_edit) => {
                            let edits = document_edit
                                .edits
                                .iter()
                                .map(|edit| match edit {
                                    lsp::OneOf::Left(text_edit) => text_edit,
                                    lsp::OneOf::Right(annotated_text_edit) => {
                                        &annotated_text_edit.text_edit
                                    }
                                })
                                .cloned()
                                .collect();
                            apply_edits(
                                &document_edit.text_document.uri,
                                document_edit.text_document.version,
                                edits,
                            )
                            .map_err(|kind| ApplyEditError {
                                kind,
                                failed_change_idx: i,
                            })?;
                        }
                    }
                }
            }
        }

        return Ok(());
    }

    if let Some(ref changes) = workspace_edit.changes {
        log::debug!("workspace changes: {:?}", changes);
        for (i, (uri, text_edits)) in changes.iter().enumerate() {
            let text_edits = text_edits.to_vec();
            apply_edits(uri, None, text_edits).map_err(|kind| ApplyEditError {
                kind,
                failed_change_idx: i,
            })?;
        }
    }

    Ok(())
}

fn goto_impl(
    editor: &mut Editor,
    compositor: &mut Compositor,
    locations: Vec<lsp::Location>,
    offset_encoding: OffsetEncoding,
) {
    let cwdir = helix_loader::current_working_dir();

    match locations.as_slice() {
        [location] => {
            jump_to_location(editor, location, offset_encoding, Action::Replace);
        }
        [] => {
            editor.set_error("No definition found.");
        }
        _locations => {
            let picker = Picker::new(locations, cwdir, move |cx, location, action| {
                jump_to_location(cx.editor, location, offset_encoding, action)
            })
            .with_preview(move |_editor, location| Some(location_to_file_location(location)));
            compositor.push(Box::new(overlaid(picker)));
        }
    }
}

fn to_locations(definitions: Option<lsp::GotoDefinitionResponse>) -> Vec<lsp::Location> {
    match definitions {
        Some(lsp::GotoDefinitionResponse::Scalar(location)) => vec![location],
        Some(lsp::GotoDefinitionResponse::Array(locations)) => locations,
        Some(lsp::GotoDefinitionResponse::Link(locations)) => locations
            .into_iter()
            .map(|location_link| lsp::Location {
                uri: location_link.target_uri,
                range: location_link.target_range,
            })
            .collect(),
        None => Vec::new(),
    }
}

fn goto_single_impl<P, F>(cx: &mut Context, feature: LanguageServerFeature, request_provider: P)
where
    P: Fn(&Client, lsp::Position, lsp::TextDocumentIdentifier) -> Option<F>,
    F: Future<Output = helix_lsp::Result<serde_json::Value>> + 'static + Send,
{
    let (view, doc) = current!(cx.editor);

    let language_server = language_server_with_feature!(cx.editor, doc, feature);
    let offset_encoding = language_server.offset_encoding();
    let pos = doc.position(view.id, offset_encoding);
    let future = request_provider(language_server, pos, doc.identifier()).unwrap();

    cx.callback(
        future,
        move |editor, compositor, response: Option<lsp::GotoDefinitionResponse>| {
            let items = to_locations(response);
            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

pub fn goto_declaration(cx: &mut Context) {
    goto_single_impl(
        cx,
        LanguageServerFeature::GotoDeclaration,
        |ls, pos, doc_id| ls.goto_declaration(doc_id, pos, None),
    );
}

pub fn goto_definition(cx: &mut Context) {
    goto_single_impl(
        cx,
        LanguageServerFeature::GotoDefinition,
        |ls, pos, doc_id| ls.goto_definition(doc_id, pos, None),
    );
}

pub fn goto_type_definition(cx: &mut Context) {
    goto_single_impl(
        cx,
        LanguageServerFeature::GotoTypeDefinition,
        |ls, pos, doc_id| ls.goto_type_definition(doc_id, pos, None),
    );
}

pub fn goto_implementation(cx: &mut Context) {
    goto_single_impl(
        cx,
        LanguageServerFeature::GotoImplementation,
        |ls, pos, doc_id| ls.goto_implementation(doc_id, pos, None),
    );
}

pub fn goto_reference(cx: &mut Context) {
    let config = cx.editor.config();
    let (view, doc) = current!(cx.editor);

    // TODO could probably support multiple language servers,
    // not sure if there's a real practical use case for this though
    let language_server =
        language_server_with_feature!(cx.editor, doc, LanguageServerFeature::GotoReference);
    let offset_encoding = language_server.offset_encoding();
    let pos = doc.position(view.id, offset_encoding);
    let future = language_server
        .goto_reference(
            doc.identifier(),
            pos,
            config.lsp.goto_reference_include_declaration,
            None,
        )
        .unwrap();

    cx.callback(
        future,
        move |editor, compositor, response: Option<Vec<lsp::Location>>| {
            let items = response.unwrap_or_default();
            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum SignatureHelpInvoked {
    Manual,
    Automatic,
}

pub fn signature_help(cx: &mut Context) {
    signature_help_impl(cx, SignatureHelpInvoked::Manual)
}

pub fn signature_help_impl(cx: &mut Context, invoked: SignatureHelpInvoked) {
    let (view, doc) = current!(cx.editor);

    // TODO merge multiple language server signature help into one instead of just taking the first language server that supports it
    let future = doc
        .language_servers_with_feature(LanguageServerFeature::SignatureHelp)
        .find_map(|language_server| {
            let pos = doc.position(view.id, language_server.offset_encoding());
            language_server.text_document_signature_help(doc.identifier(), pos, None)
        });

    let Some(future) = future else {
        // Do not show the message if signature help was invoked
        // automatically on backspace, trigger characters, etc.
        if invoked == SignatureHelpInvoked::Manual {
            cx.editor
                .set_error("No configured language server supports signature-help");
        }
        return;
    };
    signature_help_impl_with_future(cx, future.boxed(), invoked);
}

pub fn signature_help_impl_with_future(
    cx: &mut Context,
    future: BoxFuture<'static, helix_lsp::Result<Value>>,
    invoked: SignatureHelpInvoked,
) {
    cx.callback(
        future,
        move |editor, compositor, response: Option<lsp::SignatureHelp>| {
            let config = &editor.config();

            if !(config.lsp.auto_signature_help
                || SignatureHelp::visible_popup(compositor).is_some()
                || invoked == SignatureHelpInvoked::Manual)
            {
                return;
            }

            // If the signature help invocation is automatic, don't show it outside of Insert Mode:
            // it very probably means the server was a little slow to respond and the user has
            // already moved on to something else, making a signature help popup will just be an
            // annoyance, see https://github.com/helix-editor/helix/issues/3112
            if invoked == SignatureHelpInvoked::Automatic && editor.mode != Mode::Insert {
                return;
            }

            let response = match response {
                // According to the spec the response should be None if there
                // are no signatures, but some servers don't follow this.
                Some(s) if !s.signatures.is_empty() => s,
                _ => {
                    compositor.remove(SignatureHelp::ID);
                    return;
                }
            };
            let doc = doc!(editor);
            let language = doc.language_name().unwrap_or("");

            let signature = match response
                .signatures
                .get(response.active_signature.unwrap_or(0) as usize)
            {
                Some(s) => s,
                None => return,
            };
            let mut contents = SignatureHelp::new(
                signature.label.clone(),
                language.to_string(),
                Arc::clone(&editor.syn_loader),
            );

            let signature_doc = if config.lsp.display_signature_help_docs {
                signature.documentation.as_ref().map(|doc| match doc {
                    lsp::Documentation::String(s) => s.clone(),
                    lsp::Documentation::MarkupContent(markup) => markup.value.clone(),
                })
            } else {
                None
            };

            contents.set_signature_doc(signature_doc);

            let active_param_range = || -> Option<(usize, usize)> {
                let param_idx = signature
                    .active_parameter
                    .or(response.active_parameter)
                    .unwrap_or(0) as usize;
                let param = signature.parameters.as_ref()?.get(param_idx)?;
                match &param.label {
                    lsp::ParameterLabel::Simple(string) => {
                        let start = signature.label.find(string.as_str())?;
                        Some((start, start + string.len()))
                    }
                    lsp::ParameterLabel::LabelOffsets([start, end]) => {
                        // LS sends offsets based on utf-16 based string representation
                        // but highlighting in helix is done using byte offset.
                        use helix_core::str_utils::char_to_byte_idx;
                        let from = char_to_byte_idx(&signature.label, *start as usize);
                        let to = char_to_byte_idx(&signature.label, *end as usize);
                        Some((from, to))
                    }
                }
            };
            contents.set_active_param_range(active_param_range());

            let old_popup = compositor.find_id::<Popup<SignatureHelp>>(SignatureHelp::ID);
            let mut popup = Popup::new(SignatureHelp::ID, contents)
                .position(old_popup.and_then(|p| p.get_position()))
                .position_bias(Open::Above)
                .ignore_escape_key(true);

            // Don't create a popup if it intersects the auto-complete menu.
            let size = compositor.size();
            if compositor
                .find::<ui::EditorView>()
                .unwrap()
                .completion
                .as_mut()
                .map(|completion| completion.area(size, editor))
                .filter(|area| area.intersects(popup.area(size, editor)))
                .is_some()
            {
                return;
            }

            compositor.replace_or_push(SignatureHelp::ID, popup);
        },
    );
}

pub fn hover(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    // TODO support multiple language servers (merge UI somehow)
    let language_server =
        language_server_with_feature!(cx.editor, doc, LanguageServerFeature::Hover);
    // TODO: factor out a doc.position_identifier() that returns lsp::TextDocumentPositionIdentifier
    let pos = doc.position(view.id, language_server.offset_encoding());
    let future = language_server
        .text_document_hover(doc.identifier(), pos, None)
        .unwrap();

    cx.callback(
        future,
        move |editor, compositor, response: Option<lsp::Hover>| {
            if let Some(hover) = response {
                // hover.contents / .range <- used for visualizing

                fn marked_string_to_markdown(contents: lsp::MarkedString) -> String {
                    match contents {
                        lsp::MarkedString::String(contents) => contents,
                        lsp::MarkedString::LanguageString(string) => {
                            if string.language == "markdown" {
                                string.value
                            } else {
                                format!("```{}\n{}\n```", string.language, string.value)
                            }
                        }
                    }
                }

                let contents = match hover.contents {
                    lsp::HoverContents::Scalar(contents) => marked_string_to_markdown(contents),
                    lsp::HoverContents::Array(contents) => contents
                        .into_iter()
                        .map(marked_string_to_markdown)
                        .collect::<Vec<_>>()
                        .join("\n\n"),
                    lsp::HoverContents::Markup(contents) => contents.value,
                };

                // skip if contents empty

                let contents = ui::Markdown::new(contents, editor.syn_loader.clone());
                let popup = Popup::new("hover", contents).auto_close(true);
                compositor.replace_or_push("hover", popup);
            }
        },
    );
}

pub fn rename_symbol(cx: &mut Context) {
    fn get_prefill_from_word_boundary(editor: &Editor) -> String {
        let (view, doc) = current_ref!(editor);
        let text = doc.text().slice(..);
        let primary_selection = doc.selection(view.id).primary();
        if primary_selection.len() > 1 {
            primary_selection
        } else {
            use helix_core::textobject::{textobject_word, TextObject};
            textobject_word(text, primary_selection, TextObject::Inside, 1, false)
        }
        .fragment(text)
        .into()
    }

    fn get_prefill_from_lsp_response(
        editor: &Editor,
        offset_encoding: OffsetEncoding,
        response: Option<lsp::PrepareRenameResponse>,
    ) -> Result<String, &'static str> {
        match response {
            Some(lsp::PrepareRenameResponse::Range(range)) => {
                let text = doc!(editor).text();

                Ok(lsp_range_to_range(text, range, offset_encoding)
                    .ok_or("lsp sent invalid selection range for rename")?
                    .fragment(text.slice(..))
                    .into())
            }
            Some(lsp::PrepareRenameResponse::RangeWithPlaceholder { placeholder, .. }) => {
                Ok(placeholder)
            }
            Some(lsp::PrepareRenameResponse::DefaultBehavior { .. }) => {
                Ok(get_prefill_from_word_boundary(editor))
            }
            None => Err("lsp did not respond to prepare rename request"),
        }
    }

    fn create_rename_prompt(
        editor: &Editor,
        prefill: String,
        language_server_id: Option<usize>,
    ) -> Box<ui::Prompt> {
        let prompt = ui::Prompt::new(
            "rename-to:".into(),
            None,
            ui::completers::none,
            move |cx: &mut compositor::Context, input: &str, event: PromptEvent| {
                if event != PromptEvent::Validate {
                    return;
                }
                let (view, doc) = current!(cx.editor);

                let Some(language_server) = doc
                    .language_servers_with_feature(LanguageServerFeature::RenameSymbol)
                    .find(|ls| language_server_id.map_or(true, |id| id == ls.id()))
                else {
                    cx.editor
                        .set_error("No configured language server supports symbol renaming");
                    return;
                };

                let offset_encoding = language_server.offset_encoding();
                let pos = doc.position(view.id, offset_encoding);
                let future = language_server
                    .rename_symbol(doc.identifier(), pos, input.to_string())
                    .unwrap();

                match block_on(future) {
                    Ok(edits) => {
                        let _ = apply_workspace_edit(cx.editor, offset_encoding, &edits);
                    }
                    Err(err) => cx.editor.set_error(err.to_string()),
                }
            },
        )
        .with_line(prefill, editor);

        Box::new(prompt)
    }

    let (view, doc) = current_ref!(cx.editor);

    if doc
        .language_servers_with_feature(LanguageServerFeature::RenameSymbol)
        .next()
        .is_none()
    {
        cx.editor
            .set_error("No configured language server supports symbol renaming");
        return;
    }

    let language_server_with_prepare_rename_support = doc
        .language_servers_with_feature(LanguageServerFeature::RenameSymbol)
        .find(|ls| {
            matches!(
                ls.capabilities().rename_provider,
                Some(lsp::OneOf::Right(lsp::RenameOptions {
                    prepare_provider: Some(true),
                    ..
                }))
            )
        });

    if let Some(language_server) = language_server_with_prepare_rename_support {
        let ls_id = language_server.id();
        let offset_encoding = language_server.offset_encoding();
        let pos = doc.position(view.id, offset_encoding);
        let future = language_server
            .prepare_rename(doc.identifier(), pos)
            .unwrap();
        cx.callback(
            future,
            move |editor, compositor, response: Option<lsp::PrepareRenameResponse>| {
                let prefill = match get_prefill_from_lsp_response(editor, offset_encoding, response)
                {
                    Ok(p) => p,
                    Err(e) => {
                        editor.set_error(e);
                        return;
                    }
                };

                let prompt = create_rename_prompt(editor, prefill, Some(ls_id));

                compositor.push(prompt);
            },
        );
    } else {
        let prefill = get_prefill_from_word_boundary(cx.editor);
        let prompt = create_rename_prompt(cx.editor, prefill, None);
        cx.push_layer(prompt);
    }
}

pub fn select_references_to_symbol_under_cursor(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server =
        language_server_with_feature!(cx.editor, doc, LanguageServerFeature::DocumentHighlight);
    let offset_encoding = language_server.offset_encoding();
    let pos = doc.position(view.id, offset_encoding);
    let future = language_server
        .text_document_document_highlight(doc.identifier(), pos, None)
        .unwrap();

    cx.callback(
        future,
        move |editor, _compositor, response: Option<Vec<lsp::DocumentHighlight>>| {
            let document_highlights = match response {
                Some(highlights) if !highlights.is_empty() => highlights,
                _ => return,
            };
            let (view, doc) = current!(editor);
            let text = doc.text();
            let pos = doc.selection(view.id).primary().cursor(text.slice(..));

            // We must find the range that contains our primary cursor to prevent our primary cursor to move
            let mut primary_index = 0;
            let ranges = document_highlights
                .iter()
                .filter_map(|highlight| lsp_range_to_range(text, highlight.range, offset_encoding))
                .enumerate()
                .map(|(i, range)| {
                    if range.contains(pos) {
                        primary_index = i;
                    }
                    range
                })
                .collect();
            let selection = Selection::new(ranges, primary_index);
            doc.set_selection(view.id, selection);
        },
    );
}

pub fn compute_inlay_hints_for_all_views(editor: &mut Editor, jobs: &mut crate::job::Jobs) {
    if !editor.config().lsp.display_inlay_hints {
        return;
    }

    for (view, _) in editor.tree.views() {
        let doc = match editor.documents.get(&view.doc) {
            Some(doc) => doc,
            None => continue,
        };
        if let Some(callback) = compute_inlay_hints_for_view(view, doc) {
            jobs.callback(callback);
        }
    }
}

fn compute_inlay_hints_for_view(
    view: &View,
    doc: &Document,
) -> Option<std::pin::Pin<Box<impl Future<Output = Result<crate::job::Callback, anyhow::Error>>>>> {
    let view_id = view.id;
    let doc_id = view.doc;

    let language_server = doc
        .language_servers_with_feature(LanguageServerFeature::InlayHints)
        .next()?;

    let doc_text = doc.text();
    let len_lines = doc_text.len_lines();

    // Compute ~3 times the current view height of inlay hints, that way some scrolling
    // will not show half the view with hints and half without while still being faster
    // than computing all the hints for the full file (which could be dozens of time
    // longer than the view is).
    let view_height = view.inner_height();
    let first_visible_line = doc_text.char_to_line(view.offset.anchor.min(doc_text.len_chars()));
    let first_line = first_visible_line.saturating_sub(view_height);
    let last_line = first_visible_line
        .saturating_add(view_height.saturating_mul(2))
        .min(len_lines);

    let new_doc_inlay_hints_id = DocumentInlayHintsId {
        first_line,
        last_line,
    };
    // Don't recompute the annotations in case nothing has changed about the view
    if !doc.inlay_hints_oudated
        && doc
            .inlay_hints(view_id)
            .map_or(false, |dih| dih.id == new_doc_inlay_hints_id)
    {
        return None;
    }

    let doc_slice = doc_text.slice(..);
    let first_char_in_range = doc_slice.line_to_char(first_line);
    let last_char_in_range = doc_slice.line_to_char(last_line);

    let range = helix_lsp::util::range_to_lsp_range(
        doc_text,
        helix_core::Range::new(first_char_in_range, last_char_in_range),
        language_server.offset_encoding(),
    );

    let offset_encoding = language_server.offset_encoding();

    let callback = super::make_job_callback(
        language_server.text_document_range_inlay_hints(doc.identifier(), range, None)?,
        move |editor, _compositor, response: Option<Vec<lsp::InlayHint>>| {
            // The config was modified or the window was closed while the request was in flight
            if !editor.config().lsp.display_inlay_hints || editor.tree.try_get(view_id).is_none() {
                return;
            }

            // Add annotations to relevant document, not the current one (it may have changed in between)
            let doc = match editor.documents.get_mut(&doc_id) {
                Some(doc) => doc,
                None => return,
            };

            // If we have neither hints nor an LSP, empty the inlay hints since they're now oudated
            let mut hints = match response {
                Some(hints) if !hints.is_empty() => hints,
                _ => {
                    doc.set_inlay_hints(
                        view_id,
                        DocumentInlayHints::empty_with_id(new_doc_inlay_hints_id),
                    );
                    doc.inlay_hints_oudated = false;
                    return;
                }
            };

            // Most language servers will already send them sorted but ensure this is the case to
            // avoid errors on our end.
            hints.sort_unstable_by_key(|inlay_hint| inlay_hint.position);

            let mut padding_before_inlay_hints = Vec::new();
            let mut type_inlay_hints = Vec::new();
            let mut parameter_inlay_hints = Vec::new();
            let mut other_inlay_hints = Vec::new();
            let mut padding_after_inlay_hints = Vec::new();

            let doc_text = doc.text();

            for hint in hints {
                let char_idx =
                    match helix_lsp::util::lsp_pos_to_pos(doc_text, hint.position, offset_encoding)
                    {
                        Some(pos) => pos,
                        // Skip inlay hints that have no "real" position
                        None => continue,
                    };

                let label = match hint.label {
                    lsp::InlayHintLabel::String(s) => s,
                    lsp::InlayHintLabel::LabelParts(parts) => parts
                        .into_iter()
                        .map(|p| p.value)
                        .collect::<Vec<_>>()
                        .join(""),
                };

                let inlay_hints_vec = match hint.kind {
                    Some(lsp::InlayHintKind::TYPE) => &mut type_inlay_hints,
                    Some(lsp::InlayHintKind::PARAMETER) => &mut parameter_inlay_hints,
                    // We can't warn on unknown kind here since LSPs are free to set it or not, for
                    // example Rust Analyzer does not: every kind will be `None`.
                    _ => &mut other_inlay_hints,
                };

                if let Some(true) = hint.padding_left {
                    padding_before_inlay_hints.push(InlineAnnotation::new(char_idx, " "));
                }

                inlay_hints_vec.push(InlineAnnotation::new(char_idx, label));

                if let Some(true) = hint.padding_right {
                    padding_after_inlay_hints.push(InlineAnnotation::new(char_idx, " "));
                }
            }

            doc.set_inlay_hints(
                view_id,
                DocumentInlayHints {
                    id: new_doc_inlay_hints_id,
                    type_inlay_hints: type_inlay_hints.into(),
                    parameter_inlay_hints: parameter_inlay_hints.into(),
                    other_inlay_hints: other_inlay_hints.into(),
                    padding_before_inlay_hints: padding_before_inlay_hints.into(),
                    padding_after_inlay_hints: padding_after_inlay_hints.into(),
                },
            );
            doc.inlay_hints_oudated = false;
        },
    );

    Some(callback)
}
