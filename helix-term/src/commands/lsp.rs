use futures_util::{stream::FuturesOrdered, FutureExt};
use helix_lsp::{
    block_on,
    lsp::{
        self, CodeAction, CodeActionOrCommand, CodeActionTriggerKind, DiagnosticSeverity,
        NumberOrString,
    },
    util::{diagnostic_to_lsp_diagnostic, lsp_range_to_range, range_to_lsp_range},
    Client, LanguageServerId, OffsetEncoding,
};
use tokio_stream::StreamExt;
use tui::{text::Span, widgets::Row};

use super::{align_view, push_jump, Align, Context, Editor};

use helix_core::{
    diagnostic::DiagnosticProvider, syntax::config::LanguageServerFeature,
    text_annotations::InlineAnnotation, Selection, Uri,
};
use helix_stdx::path;
use helix_view::{
    document::{DocumentInlayHints, DocumentInlayHintsId},
    editor::Action,
    handlers::lsp::SignatureHelpInvoked,
    icons::ICONS,
    theme::Style,
    Document, View,
};

use crate::{
    compositor::{self, Compositor},
    job::Callback,
    ui::{self, overlay::overlaid, FileLocation, Picker, Popup, PromptEvent},
};

use std::{cmp::Ordering, collections::HashSet, fmt::Display, future::Future, path::Path};

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
                $editor.set_error(format!(
                    "No configured language server supports {}",
                    $feature
                ));
                return;
            }
        }
    }};
}

/// A wrapper around `lsp::Location` that swaps out the LSP URI for `helix_core::Uri` and adds
/// the server's  offset encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Location {
    uri: Uri,
    range: lsp::Range,
    offset_encoding: OffsetEncoding,
}

fn lsp_location_to_location(
    location: lsp::Location,
    offset_encoding: OffsetEncoding,
) -> Option<Location> {
    let uri = match location.uri.try_into() {
        Ok(uri) => uri,
        Err(err) => {
            log::warn!("discarding invalid or unsupported URI: {err}");
            return None;
        }
    };
    Some(Location {
        uri,
        range: location.range,
        offset_encoding,
    })
}

struct SymbolInformationItem {
    location: Location,
    symbol: lsp::SymbolInformation,
}

struct DiagnosticStyles {
    hint: Style,
    info: Style,
    warning: Style,
    error: Style,
}

struct PickerDiagnostic {
    location: Location,
    diag: lsp::Diagnostic,
}

fn location_to_file_location(location: &Location) -> Option<FileLocation> {
    let path = location.uri.as_path()?;
    let line = Some((
        location.range.start.line as usize,
        location.range.end.line as usize,
    ));
    Some((path.into(), line))
}

fn jump_to_location(editor: &mut Editor, location: &Location, action: Action) {
    let (view, doc) = current!(editor);
    push_jump(view, doc);

    let Some(path) = location.uri.as_path() else {
        let err = format!("unable to convert URI to filepath: {:?}", location.uri);
        editor.set_error(err);
        return;
    };
    jump_to_position(
        editor,
        path,
        location.range,
        location.offset_encoding,
        action,
    );
}

fn jump_to_position(
    editor: &mut Editor,
    path: &Path,
    range: lsp::Range,
    offset_encoding: OffsetEncoding,
    action: Action,
) {
    let doc = match editor.open(path, action) {
        Ok(id) => doc_mut!(editor, &id),
        Err(err) => {
            let err = format!("failed to open path: {:?}: {:?}", path, err);
            editor.set_error(err);
            return;
        }
    };
    let view = view_mut!(editor);
    // TODO: convert inside server
    let new_range = if let Some(new_range) = lsp_range_to_range(doc.text(), range, offset_encoding)
    {
        new_range
    } else {
        log::warn!("lsp position out of bounds - {:?}", range);
        return;
    };
    // we flip the range so that the cursor sits on the start of the symbol
    // (for example start of the function).
    doc.set_selection(view.id, Selection::single(new_range.head, new_range.anchor));
    if action.align_view(view, doc.id()) {
        align_view(doc, view, Align::Center);
    }
}

fn display_symbol_kind(kind: lsp::SymbolKind) -> &'static str {
    match kind {
        lsp::SymbolKind::FILE => "file",
        lsp::SymbolKind::MODULE => "module",
        lsp::SymbolKind::NAMESPACE => "namespace",
        lsp::SymbolKind::PACKAGE => "package",
        lsp::SymbolKind::CLASS => "class",
        lsp::SymbolKind::METHOD => "method",
        lsp::SymbolKind::PROPERTY => "property",
        lsp::SymbolKind::FIELD => "field",
        lsp::SymbolKind::CONSTRUCTOR => "construct",
        lsp::SymbolKind::ENUM => "enum",
        lsp::SymbolKind::INTERFACE => "interface",
        lsp::SymbolKind::FUNCTION => "function",
        lsp::SymbolKind::VARIABLE => "variable",
        lsp::SymbolKind::CONSTANT => "constant",
        lsp::SymbolKind::STRING => "string",
        lsp::SymbolKind::NUMBER => "number",
        lsp::SymbolKind::BOOLEAN => "boolean",
        lsp::SymbolKind::ARRAY => "array",
        lsp::SymbolKind::OBJECT => "object",
        lsp::SymbolKind::KEY => "key",
        lsp::SymbolKind::NULL => "null",
        lsp::SymbolKind::ENUM_MEMBER => "enum_member",
        lsp::SymbolKind::STRUCT => "struct",
        lsp::SymbolKind::EVENT => "event",
        lsp::SymbolKind::OPERATOR => "operator",
        lsp::SymbolKind::TYPE_PARAMETER => "typeparam",
        _ => {
            log::warn!("Unknown symbol kind: {:?}", kind);
            ""
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum DiagnosticsFormat {
    ShowSourcePath,
    HideSourcePath,
}

type DiagnosticsPicker = Picker<PickerDiagnostic, DiagnosticStyles>;

fn diag_picker(
    cx: &Context,
    diagnostics: impl IntoIterator<Item = (Uri, Vec<(lsp::Diagnostic, DiagnosticProvider)>)>,
    format: DiagnosticsFormat,
) -> DiagnosticsPicker {
    // TODO: drop current_path comparison and instead use workspace: bool flag?

    // flatten the map to a vec of (url, diag) pairs
    let mut flat_diag = Vec::new();
    for (uri, diags) in diagnostics {
        flat_diag.reserve(diags.len());

        for (diag, provider) in diags {
            if let Some(ls) = provider
                .language_server_id()
                .and_then(|id| cx.editor.language_server_by_id(id))
            {
                flat_diag.push(PickerDiagnostic {
                    location: Location {
                        uri: uri.clone(),
                        range: diag.range,
                        offset_encoding: ls.offset_encoding(),
                    },
                    diag,
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

    let mut columns = vec![
        ui::PickerColumn::new(
            "severity",
            |item: &PickerDiagnostic, styles: &DiagnosticStyles| {
                let icons = ICONS.load();
                match item.diag.severity {
                    Some(DiagnosticSeverity::HINT) => {
                        Span::styled(format!("{} HINT", icons.diagnostic().hint()), styles.hint)
                    }
                    Some(DiagnosticSeverity::INFORMATION) => {
                        Span::styled(format!("{} INFO", icons.diagnostic().info()), styles.info)
                    }
                    Some(DiagnosticSeverity::WARNING) => Span::styled(
                        format!("{} WARN", icons.diagnostic().warning()),
                        styles.warning,
                    ),
                    Some(DiagnosticSeverity::ERROR) => Span::styled(
                        format!("{} ERROR", icons.diagnostic().error()),
                        styles.error,
                    ),
                    _ => Span::raw(""),
                }
                .into()
            },
        ),
        ui::PickerColumn::new("source", |item: &PickerDiagnostic, _| {
            item.diag.source.as_deref().unwrap_or("").into()
        }),
        ui::PickerColumn::new("code", |item: &PickerDiagnostic, _| {
            match item.diag.code.as_ref() {
                Some(NumberOrString::Number(n)) => n.to_string().into(),
                Some(NumberOrString::String(s)) => s.as_str().into(),
                None => "".into(),
            }
        }),
        ui::PickerColumn::new("message", |item: &PickerDiagnostic, _| {
            item.diag.message.as_str().into()
        }),
    ];
    let mut primary_column = 3; // message

    if format == DiagnosticsFormat::ShowSourcePath {
        columns.insert(
            // between message code and message
            3,
            ui::PickerColumn::new("path", |item: &PickerDiagnostic, _| {
                if let Some(path) = item.location.uri.as_path() {
                    path::get_truncated_path(path)
                        .to_string_lossy()
                        .to_string()
                        .into()
                } else {
                    Default::default()
                }
            }),
        );
        primary_column += 1;
    }

    Picker::new(
        columns,
        primary_column,
        flat_diag,
        styles,
        move |cx, diag, action| {
            jump_to_location(cx.editor, &diag.location, action);
            let (view, doc) = current!(cx.editor);
            view.diagnostics_handler
                .immediately_show_diagnostic(doc, view.id);
        },
    )
    .with_preview(move |_editor, diag| location_to_file_location(&diag.location))
    .truncate_start(false)
}

pub fn symbol_picker(cx: &mut Context) {
    fn nested_to_flat(
        list: &mut Vec<SymbolInformationItem>,
        file: &lsp::TextDocumentIdentifier,
        uri: &Uri,
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
            location: Location {
                uri: uri.clone(),
                range: symbol.selection_range,
                offset_encoding,
            },
        });
        for child in symbol.children.into_iter().flatten() {
            nested_to_flat(list, file, uri, child, offset_encoding);
        }
    }
    let doc = doc!(cx.editor);

    let mut seen_language_servers = HashSet::new();

    let mut futures: FuturesOrdered<_> = doc
        .language_servers_with_feature(LanguageServerFeature::DocumentSymbols)
        .filter(|ls| seen_language_servers.insert(ls.id()))
        .map(|language_server| {
            let request = language_server.document_symbols(doc.identifier()).unwrap();
            let offset_encoding = language_server.offset_encoding();
            let doc_id = doc.identifier();
            let doc_uri = doc
                .uri()
                .expect("docs with active language servers must be backed by paths");

            async move {
                let symbols = match request.await? {
                    Some(symbols) => symbols,
                    None => return anyhow::Ok(vec![]),
                };
                // lsp has two ways to represent symbols (flat/nested)
                // convert the nested variant to flat, so that we have a homogeneous list
                let symbols = match symbols {
                    lsp::DocumentSymbolResponse::Flat(symbols) => symbols
                        .into_iter()
                        .map(|symbol| SymbolInformationItem {
                            location: Location {
                                uri: doc_uri.clone(),
                                range: symbol.location.range,
                                offset_encoding,
                            },
                            symbol,
                        })
                        .collect(),
                    lsp::DocumentSymbolResponse::Nested(symbols) => {
                        let mut flat_symbols = Vec::new();
                        for symbol in symbols {
                            nested_to_flat(
                                &mut flat_symbols,
                                &doc_id,
                                &doc_uri,
                                symbol,
                                offset_encoding,
                            )
                        }
                        flat_symbols
                    }
                };
                Ok(symbols)
            }
        })
        .collect();

    if futures.is_empty() {
        cx.editor
            .set_error("No configured language server supports document symbols");
        return;
    }

    cx.jobs.callback(async move {
        let mut symbols = Vec::new();
        while let Some(response) = futures.next().await {
            match response {
                Ok(mut items) => symbols.append(&mut items),
                Err(err) => log::error!("Error requesting document symbols: {err}"),
            }
        }
        let call = move |_editor: &mut Editor, compositor: &mut Compositor| {
            let columns = [
                ui::PickerColumn::new("kind", |item: &SymbolInformationItem, _| {
                    let icons = ICONS.load();
                    let name = display_symbol_kind(item.symbol.kind);

                    if let Some(icon) = icons.kind().get(name) {
                        if let Some(color) = icon.color() {
                            Span::styled(
                                format!("{}  {name}", icon.glyph()),
                                Style::default().fg(color),
                            )
                            .into()
                        } else {
                            format!("{}  {name}", icon.glyph()).into()
                        }
                    } else {
                        name.into()
                    }
                }),
                // Some symbols in the document symbol picker may have a URI that isn't
                // the current file. It should be rare though, so we concatenate that
                // URI in with the symbol name in this picker.
                ui::PickerColumn::new("name", |item: &SymbolInformationItem, _| {
                    item.symbol.name.as_str().into()
                }),
                ui::PickerColumn::new("container", |item: &SymbolInformationItem, _| {
                    item.symbol
                        .container_name
                        .as_deref()
                        .unwrap_or_default()
                        .into()
                }),
            ];

            let picker = Picker::new(
                columns,
                1, // name column
                symbols,
                (),
                move |cx, item, action| {
                    jump_to_location(cx.editor, &item.location, action);
                },
            )
            .with_preview(move |_editor, item| location_to_file_location(&item.location))
            .truncate_start(false);

            compositor.push(Box::new(overlaid(picker)))
        };

        Ok(Callback::EditorCompositor(Box::new(call)))
    });
}

pub fn workspace_symbol_picker(cx: &mut Context) {
    use crate::ui::picker::Injector;

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

    let get_symbols = |pattern: &str, editor: &mut Editor, _data, injector: &Injector<_, _>| {
        let doc = doc!(editor);
        let mut seen_language_servers = HashSet::new();
        let mut futures: FuturesOrdered<_> = doc
            .language_servers_with_feature(LanguageServerFeature::WorkspaceSymbols)
            .filter(|ls| seen_language_servers.insert(ls.id()))
            .map(|language_server| {
                let request = language_server
                    .workspace_symbols(pattern.to_string())
                    .unwrap();
                let offset_encoding = language_server.offset_encoding();
                async move {
                    let symbols = request
                        .await?
                        .and_then(|resp| match resp {
                            lsp::WorkspaceSymbolResponse::Flat(symbols) => Some(symbols),
                            lsp::WorkspaceSymbolResponse::Nested(_) => None,
                        })
                        .unwrap_or_default();

                    let response: Vec<_> = symbols
                        .into_iter()
                        .filter_map(|symbol| {
                            let uri = match Uri::try_from(&symbol.location.uri) {
                                Ok(uri) => uri,
                                Err(err) => {
                                    log::warn!("discarding symbol with invalid URI: {err}");
                                    return None;
                                }
                            };
                            Some(SymbolInformationItem {
                                location: Location {
                                    uri,
                                    range: symbol.location.range,
                                    offset_encoding,
                                },
                                symbol,
                            })
                        })
                        .collect();

                    anyhow::Ok(response)
                }
            })
            .collect();

        if futures.is_empty() {
            editor.set_error("No configured language server supports workspace symbols");
        }

        let injector = injector.clone();
        async move {
            while let Some(response) = futures.next().await {
                match response {
                    Ok(items) => {
                        for item in items {
                            injector.push(item)?;
                        }
                    }
                    Err(err) => log::error!("Error requesting workspace symbols: {err}"),
                }
            }
            Ok(())
        }
        .boxed()
    };
    let columns = [
        ui::PickerColumn::new("kind", |item: &SymbolInformationItem, _| {
            let icons = ICONS.load();
            let name = display_symbol_kind(item.symbol.kind);

            if let Some(icon) = icons.kind().get(name) {
                if let Some(color) = icon.color() {
                    Span::styled(
                        format!("{}  {name}", icon.glyph()),
                        Style::default().fg(color),
                    )
                    .into()
                } else {
                    format!("{}  {name}", icon.glyph()).into()
                }
            } else {
                name.into()
            }
        }),
        ui::PickerColumn::new("name", |item: &SymbolInformationItem, _| {
            item.symbol.name.as_str().into()
        })
        .without_filtering(),
        ui::PickerColumn::new("container", |item: &SymbolInformationItem, _| {
            item.symbol
                .container_name
                .as_deref()
                .unwrap_or_default()
                .into()
        }),
        ui::PickerColumn::new("path", |item: &SymbolInformationItem, _| {
            if let Some(path) = item.location.uri.as_path() {
                path::get_relative_path(path)
                    .to_string_lossy()
                    .to_string()
                    .into()
            } else {
                item.symbol.location.uri.to_string().into()
            }
        }),
    ];

    let picker = Picker::new(
        columns,
        1, // name column
        [],
        (),
        move |cx, item, action| {
            jump_to_location(cx.editor, &item.location, action);
        },
    )
    .with_preview(|_editor, item| location_to_file_location(&item.location))
    .with_dynamic_query(get_symbols, None)
    .truncate_start(false);

    cx.push_layer(Box::new(overlaid(picker)));
}

pub fn diagnostics_picker(cx: &mut Context) {
    let doc = doc!(cx.editor);
    if let Some(uri) = doc.uri() {
        let diagnostics = cx.editor.diagnostics.get(&uri).cloned().unwrap_or_default();
        let picker = diag_picker(cx, [(uri, diagnostics)], DiagnosticsFormat::HideSourcePath);
        cx.push_layer(Box::new(overlaid(picker)));
    }
}

pub fn workspace_diagnostics_picker(cx: &mut Context) {
    // TODO not yet filtered by LanguageServerFeature, need to do something similar as Document::shown_diagnostics here for all open documents
    let diagnostics = cx.editor.diagnostics.clone();
    let picker = diag_picker(cx, diagnostics, DiagnosticsFormat::ShowSourcePath);
    cx.push_layer(Box::new(overlaid(picker)));
}

struct CodeActionOrCommandItem {
    lsp_item: lsp::CodeActionOrCommand,
    language_server_id: LanguageServerId,
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

    let mut futures: FuturesOrdered<_> = doc
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
            let Some(mut actions) = request.await? else {
                return anyhow::Ok(Vec::new());
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

        while let Some(output) = futures.next().await {
            match output {
                Ok(mut lsp_items) => actions.append(&mut lsp_items),
                Err(err) => log::error!("while gathering code actions: {err}"),
            }
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
                        editor.execute_lsp_command(command.clone(), action.language_server_id);
                    }
                    lsp::CodeActionOrCommand::CodeAction(code_action) => {
                        log::debug!("code action: {:?}", code_action);
                        // we support lsp "codeAction/resolve" for `edit` and `command` fields
                        let mut resolved_code_action = None;
                        if code_action.edit.is_none() || code_action.command.is_none() {
                            if let Some(future) = language_server.resolve_code_action(code_action) {
                                if let Ok(code_action) = helix_lsp::block_on(future) {
                                    resolved_code_action = Some(code_action);
                                }
                            }
                        }
                        let resolved_code_action =
                            resolved_code_action.as_ref().unwrap_or(code_action);

                        if let Some(ref workspace_edit) = resolved_code_action.edit {
                            let _ = editor.apply_workspace_edit(offset_encoding, workspace_edit);
                        }

                        // if code action provides both edit and command first the edit
                        // should be applied and then the command
                        if let Some(command) = &code_action.command {
                            editor.execute_lsp_command(command.clone(), action.language_server_id);
                        }
                    }
                }
            });
            picker.move_down(); // pre-select the first item

            let popup = Popup::new("code-action", picker)
                .with_scrollbar(false)
                .auto_close(true);

            compositor.replace_or_push("code-action", popup);
        };

        Ok(Callback::EditorCompositor(Box::new(call)))
    });
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

impl Display for ApplyEditErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplyEditErrorKind::DocumentChanged => f.write_str("document has changed"),
            ApplyEditErrorKind::FileNotFound => f.write_str("file not found"),
            ApplyEditErrorKind::UnknownURISchema => f.write_str("URI schema not supported"),
            ApplyEditErrorKind::IoError(err) => f.write_str(&format!("{err}")),
        }
    }
}

/// Precondition: `locations` should be non-empty.
fn goto_impl(editor: &mut Editor, compositor: &mut Compositor, locations: Vec<Location>) {
    let cwdir = helix_stdx::env::current_working_dir();

    match locations.as_slice() {
        [location] => {
            jump_to_location(editor, location, Action::Replace);
        }
        [] => unreachable!("`locations` should be non-empty for `goto_impl`"),
        _locations => {
            let columns = [ui::PickerColumn::new(
                "location",
                |item: &Location, cwdir: &std::path::PathBuf| {
                    let path = if let Some(path) = item.uri.as_path() {
                        path.strip_prefix(cwdir).unwrap_or(path).to_string_lossy()
                    } else {
                        item.uri.to_string().into()
                    };

                    format!("{path}:{}", item.range.start.line + 1).into()
                },
            )];

            let picker = Picker::new(columns, 0, locations, cwdir, |cx, location, action| {
                jump_to_location(cx.editor, location, action)
            })
            .with_preview(|_editor, location| location_to_file_location(location));
            compositor.push(Box::new(overlaid(picker)));
        }
    }
}

fn goto_single_impl<P, F>(cx: &mut Context, feature: LanguageServerFeature, request_provider: P)
where
    P: Fn(&Client, lsp::Position, lsp::TextDocumentIdentifier) -> Option<F>,
    F: Future<Output = helix_lsp::Result<Option<lsp::GotoDefinitionResponse>>> + 'static + Send,
{
    let (view, doc) = current_ref!(cx.editor);
    let mut futures: FuturesOrdered<_> = doc
        .language_servers_with_feature(feature)
        .map(|language_server| {
            let offset_encoding = language_server.offset_encoding();
            let pos = doc.position(view.id, offset_encoding);
            let future = request_provider(language_server, pos, doc.identifier()).unwrap();
            async move { anyhow::Ok((future.await?, offset_encoding)) }
        })
        .collect();

    cx.jobs.callback(async move {
        let mut locations = Vec::new();
        while let Some(response) = futures.next().await {
            match response {
                Ok((response, offset_encoding)) => match response {
                    Some(lsp::GotoDefinitionResponse::Scalar(lsp_location)) => {
                        locations.extend(lsp_location_to_location(lsp_location, offset_encoding));
                    }
                    Some(lsp::GotoDefinitionResponse::Array(lsp_locations)) => {
                        locations.extend(lsp_locations.into_iter().flat_map(|location| {
                            lsp_location_to_location(location, offset_encoding)
                        }));
                    }
                    Some(lsp::GotoDefinitionResponse::Link(lsp_locations)) => {
                        locations.extend(
                            lsp_locations
                                .into_iter()
                                .map(|location_link| {
                                    lsp::Location::new(
                                        location_link.target_uri,
                                        location_link.target_range,
                                    )
                                })
                                .flat_map(|location| {
                                    lsp_location_to_location(location, offset_encoding)
                                }),
                        );
                    }
                    None => (),
                },
                Err(err) => log::error!("Error requesting locations: {err}"),
            }
        }
        let call = move |editor: &mut Editor, compositor: &mut Compositor| {
            if locations.is_empty() {
                editor.set_error("No definition found.");
            } else {
                goto_impl(editor, compositor, locations);
            }
        };
        Ok(Callback::EditorCompositor(Box::new(call)))
    });
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
    let (view, doc) = current_ref!(cx.editor);

    let mut futures: FuturesOrdered<_> = doc
        .language_servers_with_feature(LanguageServerFeature::GotoReference)
        .map(|language_server| {
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
            async move { anyhow::Ok((future.await?, offset_encoding)) }
        })
        .collect();

    cx.jobs.callback(async move {
        let mut locations = Vec::new();
        while let Some(response) = futures.next().await {
            match response {
                Ok((lsp_locations, offset_encoding)) => locations.extend(
                    lsp_locations
                        .into_iter()
                        .flatten()
                        .flat_map(|location| lsp_location_to_location(location, offset_encoding)),
                ),
                Err(err) => log::error!("Error requesting references: {err}"),
            }
        }
        let call = move |editor: &mut Editor, compositor: &mut Compositor| {
            if locations.is_empty() {
                editor.set_error("No references found.");
            } else {
                goto_impl(editor, compositor, locations);
            }
        };
        Ok(Callback::EditorCompositor(Box::new(call)))
    });
}

pub fn signature_help(cx: &mut Context) {
    cx.editor
        .handlers
        .trigger_signature_help(SignatureHelpInvoked::Manual, cx.editor)
}

pub fn hover(cx: &mut Context) {
    use ui::lsp::hover::Hover;

    let (view, doc) = current!(cx.editor);
    if doc
        .language_servers_with_feature(LanguageServerFeature::Hover)
        .count()
        == 0
    {
        cx.editor
            .set_error("No configured language server supports hover");
        return;
    }

    let mut seen_language_servers = HashSet::new();
    let mut futures: FuturesOrdered<_> = doc
        .language_servers_with_feature(LanguageServerFeature::Hover)
        .filter(|ls| seen_language_servers.insert(ls.id()))
        .map(|language_server| {
            let server_name = language_server.name().to_string();
            // TODO: factor out a doc.position_identifier() that returns lsp::TextDocumentPositionIdentifier
            let pos = doc.position(view.id, language_server.offset_encoding());
            let request = language_server
                .text_document_hover(doc.identifier(), pos, None)
                .unwrap();

            async move { anyhow::Ok((server_name, request.await?)) }
        })
        .collect();

    cx.jobs.callback(async move {
        let mut hovers: Vec<(String, lsp::Hover)> = Vec::new();

        while let Some(response) = futures.next().await {
            match response {
                Ok((server_name, Some(hover))) => hovers.push((server_name, hover)),
                Ok(_) => (),
                Err(err) => log::error!("Error requesting hover: {err}"),
            }
        }

        let call = move |editor: &mut Editor, compositor: &mut Compositor| {
            if hovers.is_empty() {
                editor.set_status("No hover results available.");
                return;
            }

            // create new popup
            let contents = Hover::new(hovers, editor.syn_loader.clone());
            let popup = Popup::new(Hover::ID, contents).auto_close(true);
            compositor.replace_or_push(Hover::ID, popup);
        };
        Ok(Callback::EditorCompositor(Box::new(call)))
    });
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
        history_register: Option<char>,
        language_server_id: Option<LanguageServerId>,
    ) -> Box<ui::Prompt> {
        let prompt = ui::Prompt::new(
            "rename-to:".into(),
            history_register,
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
                        let _ = cx
                            .editor
                            .apply_workspace_edit(offset_encoding, &edits.unwrap_or_default());
                    }
                    Err(err) => cx.editor.set_error(err.to_string()),
                }
            },
        )
        .with_line(prefill, editor);

        Box::new(prompt)
    }

    let (view, doc) = current_ref!(cx.editor);
    let history_register = cx.register;

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

                let prompt = create_rename_prompt(editor, prefill, history_register, Some(ls_id));

                compositor.push(prompt);
            },
        );
    } else {
        let prefill = get_prefill_from_word_boundary(cx.editor);
        let prompt = create_rename_prompt(cx.editor, prefill, history_register, None);
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
    let first_visible_line =
        doc_text.char_to_line(doc.view_offset(view_id).anchor.min(doc_text.len_chars()));
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
            .is_some_and(|dih| dih.id == new_doc_inlay_hints_id)
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
            hints.sort_by_key(|inlay_hint| inlay_hint.position);

            let mut padding_before_inlay_hints = Vec::new();
            let mut type_inlay_hints = Vec::new();
            let mut parameter_inlay_hints = Vec::new();
            let mut other_inlay_hints = Vec::new();
            let mut padding_after_inlay_hints = Vec::new();

            let doc_text = doc.text();
            let inlay_hints_length_limit = doc.config.load().lsp.inlay_hints_length_limit;

            for hint in hints {
                let char_idx =
                    match helix_lsp::util::lsp_pos_to_pos(doc_text, hint.position, offset_encoding)
                    {
                        Some(pos) => pos,
                        // Skip inlay hints that have no "real" position
                        None => continue,
                    };

                let mut label = match hint.label {
                    lsp::InlayHintLabel::String(s) => s,
                    lsp::InlayHintLabel::LabelParts(parts) => parts
                        .into_iter()
                        .map(|p| p.value)
                        .collect::<Vec<_>>()
                        .join(""),
                };
                // Truncate the hint if too long
                if let Some(limit) = inlay_hints_length_limit {
                    // Limit on displayed width
                    use helix_core::unicode::{
                        segmentation::UnicodeSegmentation, width::UnicodeWidthStr,
                    };

                    let width = label.width();
                    let limit = limit.get().into();
                    if width > limit {
                        let mut floor_boundary = 0;
                        let mut acc = 0;
                        for (i, grapheme_cluster) in label.grapheme_indices(true) {
                            acc += grapheme_cluster.width();

                            if acc > limit {
                                floor_boundary = i;
                                break;
                            }
                        }

                        label.truncate(floor_boundary);
                        label.push('…');
                    }
                }

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
                    type_inlay_hints,
                    parameter_inlay_hints,
                    other_inlay_hints,
                    padding_before_inlay_hints,
                    padding_after_inlay_hints,
                },
            );
            doc.inlay_hints_oudated = false;
        },
    );

    Some(callback)
}
