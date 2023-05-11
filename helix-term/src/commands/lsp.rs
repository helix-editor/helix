use futures_util::FutureExt;
use helix_lsp::{
    block_on,
    lsp::{
        self, CodeAction, CodeActionOrCommand, CodeActionTriggerKind, DiagnosticSeverity,
        NumberOrString,
    },
    util::{diagnostic_to_lsp_diagnostic, lsp_range_to_range, range_to_lsp_range},
    OffsetEncoding,
};
use tui::{
    text::{Span, Spans},
    widgets::Row,
};

use super::{align_view, push_jump, Align, Context, Editor, Open};

use helix_core::{path, text_annotations::InlineAnnotation, Selection};
use helix_view::{
    document::{DocumentInlayHints, DocumentInlayHintsId, Mode},
    editor::Action,
    theme::Style,
    Document, Theme, View,
};

use crate::{
    compositor::{self, Compositor},
    ui::{
        self, lsp::SignatureHelp, overlay::overlaid, DynamicPicker, FileLocation, FilePicker,
        Popup, PromptEvent,
    },
};

use std::{
    cmp::Ordering, collections::BTreeMap, fmt::Write, future::Future, path::PathBuf, sync::Arc,
};

/// Gets the language server that is attached to a document, and
/// if it's not active displays a status message. Using this macro
/// in a context where the editor automatically queries the LSP
/// (instead of when the user explicitly does so via a keybind like
/// `gd`) will spam the "LSP inactive" status message confusingly.
#[macro_export]
macro_rules! language_server {
    ($editor:expr, $doc:expr) => {
        match $doc.language_server() {
            Some(language_server) => language_server,
            None => {
                $editor.set_status("Language server not active for current buffer");
                return;
            }
        }
    };
}

impl ui::menu::Item for lsp::Location {
    /// Current working directory.
    type Data = PathBuf;

    fn format(&self, cwdir: &Self::Data, _theme: Option<&Theme>) -> Row {
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

impl ui::menu::Item for lsp::SymbolInformation {
    /// Path to currently focussed document
    type Data = Option<lsp::Url>;

    fn format(&self, current_doc_path: &Self::Data, _theme: Option<&Theme>) -> Row {
        if current_doc_path.as_ref() == Some(&self.location.uri) {
            self.name.as_str().into()
        } else {
            match self.location.uri.to_file_path() {
                Ok(path) => {
                    let get_relative_path = path::get_relative_path(path.as_path());
                    format!("{} ({})", &self.name, get_relative_path.to_string_lossy()).into()
                }
                Err(_) => format!("{} ({})", &self.name, &self.location.uri).into(),
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
}

impl ui::menu::Item for PickerDiagnostic {
    type Data = (DiagnosticStyles, DiagnosticsFormat);

    fn format(&self, (styles, format): &Self::Data, _theme: Option<&Theme>) -> Row {
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

// TODO: share with symbol picker(symbol.location)
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
    match editor.open(&path, action) {
        Ok(_) => (),
        Err(err) => {
            let err = format!("failed to open path: {:?}: {:?}", location.uri, err);
            editor.set_error(err);
            return;
        }
    }
    let (view, doc) = current!(editor);
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
    align_view(doc, view, Align::Center);
}

fn sym_picker(
    symbols: Vec<lsp::SymbolInformation>,
    current_path: Option<lsp::Url>,
    offset_encoding: OffsetEncoding,
) -> FilePicker<lsp::SymbolInformation> {
    // TODO: drop current_path comparison and instead use workspace: bool flag?
    FilePicker::new(
        symbols,
        current_path.clone(),
        move |cx, symbol, action| {
            let (view, doc) = current!(cx.editor);
            push_jump(view, doc);

            if current_path.as_ref() != Some(&symbol.location.uri) {
                let uri = &symbol.location.uri;
                let path = match uri.to_file_path() {
                    Ok(path) => path,
                    Err(_) => {
                        let err = format!("unable to convert URI to filepath: {}", uri);
                        cx.editor.set_error(err);
                        return;
                    }
                };
                if let Err(err) = cx.editor.open(&path, action) {
                    let err = format!("failed to open document: {}: {}", uri, err);
                    log::error!("{}", err);
                    cx.editor.set_error(err);
                    return;
                }
            }

            let (view, doc) = current!(cx.editor);

            if let Some(range) =
                lsp_range_to_range(doc.text(), symbol.location.range, offset_encoding)
            {
                // we flip the range so that the cursor sits on the start of the symbol
                // (for example start of the function).
                doc.set_selection(view.id, Selection::single(range.head, range.anchor));
                align_view(doc, view, Align::Center);
            }
        },
        move |_editor, symbol| Some(location_to_file_location(&symbol.location)),
    )
    .truncate_start(false)
}

#[derive(Copy, Clone, PartialEq)]
enum DiagnosticsFormat {
    ShowSourcePath,
    HideSourcePath,
}

fn diag_picker(
    cx: &Context,
    diagnostics: BTreeMap<lsp::Url, Vec<lsp::Diagnostic>>,
    current_path: Option<lsp::Url>,
    format: DiagnosticsFormat,
    offset_encoding: OffsetEncoding,
) -> FilePicker<PickerDiagnostic> {
    // TODO: drop current_path comparison and instead use workspace: bool flag?

    // flatten the map to a vec of (url, diag) pairs
    let mut flat_diag = Vec::new();
    for (url, diags) in diagnostics {
        flat_diag.reserve(diags.len());
        for diag in diags {
            flat_diag.push(PickerDiagnostic {
                url: url.clone(),
                diag,
            });
        }
    }

    let styles = DiagnosticStyles {
        hint: cx.editor.theme.get("hint"),
        info: cx.editor.theme.get("info"),
        warning: cx.editor.theme.get("warning"),
        error: cx.editor.theme.get("error"),
    };

    FilePicker::new(
        flat_diag,
        (styles, format),
        move |cx, PickerDiagnostic { url, diag }, action| {
            if current_path.as_ref() == Some(url) {
                let (view, doc) = current!(cx.editor);
                push_jump(view, doc);
            } else {
                let path = url.to_file_path().unwrap();
                cx.editor.open(&path, action).expect("editor.open failed");
            }

            let (view, doc) = current!(cx.editor);

            if let Some(range) = lsp_range_to_range(doc.text(), diag.range, offset_encoding) {
                // we flip the range so that the cursor sits on the start of the symbol
                // (for example start of the function).
                doc.set_selection(view.id, Selection::single(range.head, range.anchor));
                align_view(doc, view, Align::Center);
            }
        },
        move |_editor, PickerDiagnostic { url, diag }| {
            let location = lsp::Location::new(url.clone(), diag.range);
            Some(location_to_file_location(&location))
        },
    )
    .truncate_start(false)
}

pub fn symbol_picker(cx: &mut Context) {
    fn nested_to_flat(
        list: &mut Vec<lsp::SymbolInformation>,
        file: &lsp::TextDocumentIdentifier,
        symbol: lsp::DocumentSymbol,
    ) {
        #[allow(deprecated)]
        list.push(lsp::SymbolInformation {
            name: symbol.name,
            kind: symbol.kind,
            tags: symbol.tags,
            deprecated: symbol.deprecated,
            location: lsp::Location::new(file.uri.clone(), symbol.selection_range),
            container_name: None,
        });
        for child in symbol.children.into_iter().flatten() {
            nested_to_flat(list, file, child);
        }
    }
    let doc = doc!(cx.editor);

    let language_server = language_server!(cx.editor, doc);
    let current_url = doc.url();
    let offset_encoding = language_server.offset_encoding();

    let future = match language_server.document_symbols(doc.identifier()) {
        Some(future) => future,
        None => {
            cx.editor
                .set_error("Language server does not support document symbols");
            return;
        }
    };

    cx.callback(
        future,
        move |editor, compositor, response: Option<lsp::DocumentSymbolResponse>| {
            if let Some(symbols) = response {
                // lsp has two ways to represent symbols (flat/nested)
                // convert the nested variant to flat, so that we have a homogeneous list
                let symbols = match symbols {
                    lsp::DocumentSymbolResponse::Flat(symbols) => symbols,
                    lsp::DocumentSymbolResponse::Nested(symbols) => {
                        let doc = doc!(editor);
                        let mut flat_symbols = Vec::new();
                        for symbol in symbols {
                            nested_to_flat(&mut flat_symbols, &doc.identifier(), symbol)
                        }
                        flat_symbols
                    }
                };

                let picker = sym_picker(symbols, current_url, offset_encoding);
                compositor.push(Box::new(overlaid(picker)))
            }
        },
    )
}

pub fn workspace_symbol_picker(cx: &mut Context) {
    let doc = doc!(cx.editor);
    let current_url = doc.url();
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();
    let future = match language_server.workspace_symbols("".to_string()) {
        Some(future) => future,
        None => {
            cx.editor
                .set_error("Language server does not support workspace symbols");
            return;
        }
    };

    cx.callback(
        future,
        move |_editor, compositor, response: Option<Vec<lsp::SymbolInformation>>| {
            let symbols = response.unwrap_or_default();
            let picker = sym_picker(symbols, current_url, offset_encoding);
            let get_symbols = |query: String, editor: &mut Editor| {
                let doc = doc!(editor);
                let language_server = match doc.language_server() {
                    Some(s) => s,
                    None => {
                        // This should not generally happen since the picker will not
                        // even open in the first place if there is no server.
                        return async move { Err(anyhow::anyhow!("LSP not active")) }.boxed();
                    }
                };
                let symbol_request = match language_server.workspace_symbols(query) {
                    Some(future) => future,
                    None => {
                        // This should also not happen since the language server must have
                        // supported workspace symbols before to reach this block.
                        return async move {
                            Err(anyhow::anyhow!(
                                "Language server does not support workspace symbols"
                            ))
                        }
                        .boxed();
                    }
                };

                let future = async move {
                    let json = symbol_request.await?;
                    let response: Option<Vec<lsp::SymbolInformation>> =
                        serde_json::from_value(json)?;

                    Ok(response.unwrap_or_default())
                };
                future.boxed()
            };
            let dyn_picker = DynamicPicker::new(picker, Box::new(get_symbols));
            compositor.push(Box::new(overlaid(dyn_picker)))
        },
    )
}

pub fn diagnostics_picker(cx: &mut Context) {
    let doc = doc!(cx.editor);
    let language_server = language_server!(cx.editor, doc);
    if let Some(current_url) = doc.url() {
        let offset_encoding = language_server.offset_encoding();
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
            offset_encoding,
        );
        cx.push_layer(Box::new(overlaid(picker)));
    }
}

pub fn workspace_diagnostics_picker(cx: &mut Context) {
    let doc = doc!(cx.editor);
    let language_server = language_server!(cx.editor, doc);
    let current_url = doc.url();
    let offset_encoding = language_server.offset_encoding();
    let diagnostics = cx.editor.diagnostics.clone();
    let picker = diag_picker(
        cx,
        diagnostics,
        current_url,
        DiagnosticsFormat::ShowSourcePath,
        offset_encoding,
    );
    cx.push_layer(Box::new(overlaid(picker)));
}

impl ui::menu::Item for lsp::CodeActionOrCommand {
    type Data = ();
    fn format(&self, _data: &Self::Data, _theme: Option<&Theme>) -> Row {
        match self {
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

    let language_server = language_server!(cx.editor, doc);

    let selection_range = doc.selection(view.id).primary();
    let offset_encoding = language_server.offset_encoding();

    let range = range_to_lsp_range(doc.text(), selection_range, offset_encoding);

    let future = match language_server.code_actions(
        doc.identifier(),
        range,
        // Filter and convert overlapping diagnostics
        lsp::CodeActionContext {
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
        },
    ) {
        Some(future) => future,
        None => {
            cx.editor
                .set_error("Language server does not support code actions");
            return;
        }
    };

    cx.callback(
        future,
        move |editor, compositor, response: Option<lsp::CodeActionResponse>| {
            let mut actions = match response {
                Some(a) => a,
                None => return,
            };

            // remove disabled code actions
            actions.retain(|action| {
                matches!(
                    action,
                    CodeActionOrCommand::Command(_)
                        | CodeActionOrCommand::CodeAction(CodeAction { disabled: None, .. })
                )
            });

            if actions.is_empty() {
                editor.set_status("No code actions available");
                return;
            }

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

            let mut picker = ui::Menu::new(actions, (), move |editor, code_action, event| {
                if event != PromptEvent::Validate {
                    return;
                }

                // always present here
                let code_action = code_action.unwrap();

                match code_action {
                    lsp::CodeActionOrCommand::Command(command) => {
                        log::debug!("code action command: {:?}", command);
                        execute_lsp_command(editor, command.clone());
                    }
                    lsp::CodeActionOrCommand::CodeAction(code_action) => {
                        log::debug!("code action: {:?}", code_action);
                        if let Some(ref workspace_edit) = code_action.edit {
                            log::debug!("edit: {:?}", workspace_edit);
                            let _ = apply_workspace_edit(editor, offset_encoding, workspace_edit);
                        }

                        // if code action provides both edit and command first the edit
                        // should be applied and then the command
                        if let Some(command) = &code_action.command {
                            execute_lsp_command(editor, command.clone());
                        }
                    }
                }
            });
            picker.move_down(); // pre-select the first item

            let popup = Popup::new("code-action", picker).with_scrollbar(false);
            compositor.replace_or_push("code-action", popup);
        },
    )
}

impl ui::menu::Item for lsp::Command {
    type Data = ();
    fn format(&self, _data: &Self::Data, _theme: Option<&Theme>) -> Row {
        self.title.as_str().into()
    }
}

pub fn execute_lsp_command(editor: &mut Editor, cmd: lsp::Command) {
    let doc = doc!(editor);
    let language_server = language_server!(editor, doc);

    // the command is executed on the server and communicated back
    // to the client asynchronously using workspace edits
    let future = match language_server.command(cmd) {
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
    let cwdir = std::env::current_dir().unwrap_or_default();

    match locations.as_slice() {
        [location] => {
            jump_to_location(editor, location, offset_encoding, Action::Replace);
        }
        [] => {
            editor.set_error("No definition found.");
        }
        _locations => {
            let picker = FilePicker::new(
                locations,
                cwdir,
                move |cx, location, action| {
                    jump_to_location(cx.editor, location, offset_encoding, action)
                },
                move |_editor, location| Some(location_to_file_location(location)),
            );
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

pub fn goto_declaration(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();

    let pos = doc.position(view.id, offset_encoding);

    let future = match language_server.goto_declaration(doc.identifier(), pos, None) {
        Some(future) => future,
        None => {
            cx.editor
                .set_error("Language server does not support goto-declaration");
            return;
        }
    };

    cx.callback(
        future,
        move |editor, compositor, response: Option<lsp::GotoDefinitionResponse>| {
            let items = to_locations(response);
            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

pub fn goto_definition(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();

    let pos = doc.position(view.id, offset_encoding);

    let future = match language_server.goto_definition(doc.identifier(), pos, None) {
        Some(future) => future,
        None => {
            cx.editor
                .set_error("Language server does not support goto-definition");
            return;
        }
    };

    cx.callback(
        future,
        move |editor, compositor, response: Option<lsp::GotoDefinitionResponse>| {
            let items = to_locations(response);
            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

pub fn goto_type_definition(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();

    let pos = doc.position(view.id, offset_encoding);

    let future = match language_server.goto_type_definition(doc.identifier(), pos, None) {
        Some(future) => future,
        None => {
            cx.editor
                .set_error("Language server does not support goto-type-definition");
            return;
        }
    };

    cx.callback(
        future,
        move |editor, compositor, response: Option<lsp::GotoDefinitionResponse>| {
            let items = to_locations(response);
            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

pub fn goto_implementation(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();

    let pos = doc.position(view.id, offset_encoding);

    let future = match language_server.goto_implementation(doc.identifier(), pos, None) {
        Some(future) => future,
        None => {
            cx.editor
                .set_error("Language server does not support goto-implementation");
            return;
        }
    };

    cx.callback(
        future,
        move |editor, compositor, response: Option<lsp::GotoDefinitionResponse>| {
            let items = to_locations(response);
            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

pub fn goto_reference(cx: &mut Context) {
    let config = cx.editor.config();
    let (view, doc) = current!(cx.editor);
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();

    let pos = doc.position(view.id, offset_encoding);

    let future = match language_server.goto_reference(
        doc.identifier(),
        pos,
        config.lsp.goto_reference_include_declaration,
        None,
    ) {
        Some(future) => future,
        None => {
            cx.editor
                .set_error("Language server does not support goto-reference");
            return;
        }
    };

    cx.callback(
        future,
        move |editor, compositor, response: Option<Vec<lsp::Location>>| {
            let items = response.unwrap_or_default();
            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

#[derive(PartialEq, Eq)]
pub enum SignatureHelpInvoked {
    Manual,
    Automatic,
}

pub fn signature_help(cx: &mut Context) {
    signature_help_impl(cx, SignatureHelpInvoked::Manual)
}

pub fn signature_help_impl(cx: &mut Context, invoked: SignatureHelpInvoked) {
    let (view, doc) = current!(cx.editor);
    let was_manually_invoked = invoked == SignatureHelpInvoked::Manual;

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => {
            // Do not show the message if signature help was invoked
            // automatically on backspace, trigger characters, etc.
            if was_manually_invoked {
                cx.editor
                    .set_status("Language server not active for current buffer");
            }
            return;
        }
    };
    let offset_encoding = language_server.offset_encoding();

    let pos = doc.position(view.id, offset_encoding);

    let future = match language_server.text_document_signature_help(doc.identifier(), pos, None) {
        Some(f) => f,
        None => {
            if was_manually_invoked {
                cx.editor
                    .set_error("Language server does not support signature-help");
            }
            return;
        }
    };

    cx.callback(
        future,
        move |editor, compositor, response: Option<lsp::SignatureHelp>| {
            let config = &editor.config();

            if !(config.lsp.auto_signature_help
                || SignatureHelp::visible_popup(compositor).is_some()
                || was_manually_invoked)
            {
                return;
            }

            // If the signature help invocation is automatic, don't show it outside of Insert Mode:
            // it very probably means the server was a little slow to respond and the user has
            // already moved on to something else, making a signature help popup will just be an
            // annoyance, see https://github.com/helix-editor/helix/issues/3112
            if !was_manually_invoked && editor.mode != Mode::Insert {
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
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();

    // TODO: factor out a doc.position_identifier() that returns lsp::TextDocumentPositionIdentifier

    let pos = doc.position(view.id, offset_encoding);

    let future = match language_server.text_document_hover(doc.identifier(), pos, None) {
        Some(future) => future,
        None => {
            cx.editor
                .set_error("Language server does not support hover");
            return;
        }
    };

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

    fn create_rename_prompt(editor: &Editor, prefill: String) -> Box<ui::Prompt> {
        let prompt = ui::Prompt::new(
            "rename-to:".into(),
            None,
            ui::completers::none,
            move |cx: &mut compositor::Context, input: &str, event: PromptEvent| {
                if event != PromptEvent::Validate {
                    return;
                }

                let (view, doc) = current!(cx.editor);
                let language_server = language_server!(cx.editor, doc);
                let offset_encoding = language_server.offset_encoding();

                let pos = doc.position(view.id, offset_encoding);

                let future =
                    match language_server.rename_symbol(doc.identifier(), pos, input.to_string()) {
                        Some(future) => future,
                        None => {
                            cx.editor
                                .set_error("Language server does not support symbol renaming");
                            return;
                        }
                    };
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

    let (view, doc) = current!(cx.editor);
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();

    if !language_server.supports_rename() {
        cx.editor
            .set_error("Language server does not support symbol renaming");
        return;
    }

    let pos = doc.position(view.id, offset_encoding);

    match language_server.prepare_rename(doc.identifier(), pos) {
        // Language server supports textDocument/prepareRename, use it.
        Some(future) => cx.callback(
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

                let prompt = create_rename_prompt(editor, prefill);

                compositor.push(prompt);
            },
        ),
        // Language server does not support textDocument/prepareRename, fall back
        // to word boundary selection.
        None => {
            let prefill = get_prefill_from_word_boundary(cx.editor);

            let prompt = create_rename_prompt(cx.editor, prefill);

            cx.push_layer(prompt);
        }
    };
}

pub fn select_references_to_symbol_under_cursor(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();

    let pos = doc.position(view.id, offset_encoding);

    let future = match language_server.text_document_document_highlight(doc.identifier(), pos, None)
    {
        Some(future) => future,
        None => {
            cx.editor
                .set_error("Language server does not support document highlight");
            return;
        }
    };

    cx.callback(
        future,
        move |editor, _compositor, response: Option<Vec<lsp::DocumentHighlight>>| {
            let document_highlights = match response {
                Some(highlights) if !highlights.is_empty() => highlights,
                _ => return,
            };
            let (view, doc) = current!(editor);
            let language_server = language_server!(editor, doc);
            let offset_encoding = language_server.offset_encoding();
            let text = doc.text();
            let pos = doc.selection(view.id).primary().head;

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

    let language_server = doc.language_server()?;

    let capabilities = language_server.capabilities();

    let (future, new_doc_inlay_hints_id) = match capabilities.inlay_hint_provider {
        Some(
            lsp::OneOf::Left(true)
            | lsp::OneOf::Right(lsp::InlayHintServerCapabilities::Options(_)),
        ) => {
            let doc_text = doc.text();
            let len_lines = doc_text.len_lines();

            // Compute ~3 times the current view height of inlay hints, that way some scrolling
            // will not show half the view with hints and half without while still being faster
            // than computing all the hints for the full file (which could be dozens of time
            // longer than the view is).
            let view_height = view.inner_height();
            let first_visible_line =
                doc_text.char_to_line(view.offset.anchor.min(doc_text.len_chars()));
            let first_line = first_visible_line.saturating_sub(view_height);
            let last_line = first_visible_line
                .saturating_add(view_height.saturating_mul(2))
                .min(len_lines);

            let new_doc_inlay_hint_id = DocumentInlayHintsId {
                first_line,
                last_line,
            };
            // Don't recompute the annotations in case nothing has changed about the view
            if !doc.inlay_hints_oudated
                && doc
                    .inlay_hints(view_id)
                    .map_or(false, |dih| dih.id == new_doc_inlay_hint_id)
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

            (
                language_server.text_document_range_inlay_hints(doc.identifier(), range, None),
                new_doc_inlay_hint_id,
            )
        }
        _ => return None,
    };

    let callback = super::make_job_callback(
        future?,
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
            let (mut hints, offset_encoding) = match (response, doc.language_server()) {
                (Some(h), Some(ls)) if !h.is_empty() => (h, ls.offset_encoding()),
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
