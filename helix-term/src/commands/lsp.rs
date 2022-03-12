use helix_lsp::{
    block_on, lsp,
    util::{lsp_pos_to_pos, lsp_range_to_range, range_to_lsp_range},
    OffsetEncoding,
};

use super::{align_view, push_jump, Align, Context, Editor};

use helix_core::Selection;
use helix_view::editor::Action;

use crate::{
    compositor::{self, Compositor},
    ui::{self, overlay::overlayed, FileLocation, FilePicker, Popup, Prompt, PromptEvent},
};

use std::borrow::Cow;

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

fn location_to_file_location(location: &lsp::Location) -> FileLocation {
    let path = location.uri.to_file_path().unwrap();
    let line = Some((
        location.range.start.line as usize,
        location.range.end.line as usize,
    ));
    (path, line)
}

// TODO: share with symbol picker(symbol.location)
// TODO: need to use push_jump() before?
fn jump_to_location(
    editor: &mut Editor,
    location: &lsp::Location,
    offset_encoding: OffsetEncoding,
    action: Action,
) {
    let path = location
        .uri
        .to_file_path()
        .expect("unable to convert URI to filepath");
    let _id = editor.open(path, action).expect("editor.open failed");
    let (view, doc) = current!(editor);
    let definition_pos = location.range.start;
    // TODO: convert inside server
    let new_pos = if let Some(new_pos) = lsp_pos_to_pos(doc.text(), definition_pos, offset_encoding)
    {
        new_pos
    } else {
        return;
    };
    doc.set_selection(view.id, Selection::point(new_pos));
    align_view(doc, view, Align::Center);
}

fn sym_picker(
    symbols: Vec<lsp::SymbolInformation>,
    current_path: Option<lsp::Url>,
    offset_encoding: OffsetEncoding,
) -> FilePicker<lsp::SymbolInformation> {
    // TODO: drop current_path comparison and instead use workspace: bool flag?
    let current_path2 = current_path.clone();
    let mut picker = FilePicker::new(
        symbols,
        move |_, symbol| {
            if current_path.as_ref() == Some(&symbol.location.uri) {
                symbol.name.as_str().into()
            } else {
                let path = symbol.location.uri.to_file_path().unwrap();
                let relative_path = helix_core::path::get_relative_path(path.as_path())
                    .to_string_lossy()
                    .into_owned();
                format!("{} ({})", &symbol.name, relative_path).into()
            }
        },
        move |cx, symbol, action| {
            if current_path2.as_ref() == Some(&symbol.location.uri) {
                push_jump(cx.editor);
            } else {
                let path = symbol.location.uri.to_file_path().unwrap();
                cx.editor.open(path, action).expect("editor.open failed");
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
    );
    picker.truncate_start = false;
    picker
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

    let future = language_server.document_symbols(doc.identifier());

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::DocumentSymbolResponse>| {
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
                compositor.push(Box::new(overlayed(picker)))
            }
        },
    )
}

pub fn workspace_symbol_picker(cx: &mut Context) {
    let doc = doc!(cx.editor);
    let current_url = doc.url();
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();
    let future = language_server.workspace_symbols("".to_string());

    cx.callback(
        future,
        move |_editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<Vec<lsp::SymbolInformation>>| {
            if let Some(symbols) = response {
                let picker = sym_picker(symbols, current_url, offset_encoding);
                compositor.push(Box::new(overlayed(picker)))
            }
        },
    )
}

impl ui::menu::Item for lsp::CodeActionOrCommand {
    fn label(&self) -> &str {
        match self {
            lsp::CodeActionOrCommand::CodeAction(action) => action.title.as_str(),
            lsp::CodeActionOrCommand::Command(command) => command.title.as_str(),
        }
    }
}

pub fn code_action(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let language_server = language_server!(cx.editor, doc);

    let range = range_to_lsp_range(
        doc.text(),
        doc.selection(view.id).primary(),
        language_server.offset_encoding(),
    );

    let future = language_server.code_actions(doc.identifier(), range);
    let offset_encoding = language_server.offset_encoding();

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::CodeActionResponse>| {
            let actions = match response {
                Some(a) => a,
                None => return,
            };
            if actions.is_empty() {
                editor.set_status("No code actions available");
                return;
            }

            let mut picker = ui::Menu::new(actions, move |editor, code_action, event| {
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
                            apply_workspace_edit(editor, offset_encoding, workspace_edit);
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

            let popup = Popup::new("code-action", picker).margin(helix_view::graphics::Margin {
                vertical: 1,
                horizontal: 1,
            });
            compositor.replace_or_push("code-action", popup);
        },
    )
}
pub fn execute_lsp_command(editor: &mut Editor, cmd: lsp::Command) {
    let doc = doc!(editor);
    let language_server = language_server!(editor, doc);

    // the command is executed on the server and communicated back
    // to the client asynchronously using workspace edits
    let command_future = language_server.command(cmd);
    tokio::spawn(async move {
        let res = command_future.await;

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
                        fs::create_dir_all(&dir)?;
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
                fs::rename(&from, &to)
            }
        }
    }
}

pub fn apply_workspace_edit(
    editor: &mut Editor,
    offset_encoding: OffsetEncoding,
    workspace_edit: &lsp::WorkspaceEdit,
) {
    let mut apply_edits = |uri: &helix_lsp::Url, text_edits: Vec<lsp::TextEdit>| {
        let path = uri
            .to_file_path()
            .expect("unable to convert URI to filepath");

        let current_view_id = view!(editor).id;
        let doc_id = editor.open(path, Action::Load).unwrap();
        let doc = editor
            .document_mut(doc_id)
            .expect("Document for document_changes not found");

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
        doc.apply(&transaction, view_id);
        doc.append_changes_to_history(view_id);
    };

    if let Some(ref changes) = workspace_edit.changes {
        log::debug!("workspace changes: {:?}", changes);
        for (uri, text_edits) in changes {
            let text_edits = text_edits.to_vec();
            apply_edits(uri, text_edits);
        }
        return;
        // Not sure if it works properly, it'll be safer to just panic here to avoid breaking some parts of code on which code actions will be used
        // TODO: find some example that uses workspace changes, and test it
        // for (url, edits) in changes.iter() {
        //     let file_path = url.origin().ascii_serialization();
        //     let file_path = std::path::PathBuf::from(file_path);
        //     let file = std::fs::File::open(file_path).unwrap();
        //     let mut text = Rope::from_reader(file).unwrap();
        //     let transaction = edits_to_changes(&text, edits);
        //     transaction.apply(&mut text);
        // }
    }

    if let Some(ref document_changes) = workspace_edit.document_changes {
        match document_changes {
            lsp::DocumentChanges::Edits(document_edits) => {
                for document_edit in document_edits {
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
                    apply_edits(&document_edit.text_document.uri, edits);
                }
            }
            lsp::DocumentChanges::Operations(operations) => {
                log::debug!("document changes - operations: {:?}", operations);
                for operateion in operations {
                    match operateion {
                        lsp::DocumentChangeOperation::Op(op) => {
                            apply_document_resource_op(op).unwrap();
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
                            apply_edits(&document_edit.text_document.uri, edits);
                        }
                    }
                }
            }
        }
    }
}
fn goto_impl(
    editor: &mut Editor,
    compositor: &mut Compositor,
    locations: Vec<lsp::Location>,
    offset_encoding: OffsetEncoding,
) {
    push_jump(editor);

    let cwdir = std::env::current_dir().expect("couldn't determine current directory");

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
                move |_, location| {
                    let file: Cow<'_, str> = (location.uri.scheme() == "file")
                        .then(|| {
                            location
                                .uri
                                .to_file_path()
                                .map(|path| {
                                    // strip root prefix
                                    path.strip_prefix(&cwdir)
                                        .map(|path| path.to_path_buf())
                                        .unwrap_or(path)
                                })
                                .map(|path| Cow::from(path.to_string_lossy().into_owned()))
                                .ok()
                        })
                        .flatten()
                        .unwrap_or_else(|| location.uri.as_str().into());
                    let line = location.range.start.line;
                    format!("{}:{}", file, line).into()
                },
                move |cx, location, action| {
                    jump_to_location(cx.editor, location, offset_encoding, action)
                },
                move |_editor, location| Some(location_to_file_location(location)),
            );
            compositor.push(Box::new(overlayed(picker)));
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

pub fn goto_definition(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();

    let pos = doc.position(view.id, offset_encoding);

    let future = language_server.goto_definition(doc.identifier(), pos, None);

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

    let future = language_server.goto_type_definition(doc.identifier(), pos, None);

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

    let future = language_server.goto_implementation(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor, compositor, response: Option<lsp::GotoDefinitionResponse>| {
            let items = to_locations(response);
            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

pub fn goto_reference(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();

    let pos = doc.position(view.id, offset_encoding);

    let future = language_server.goto_reference(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor, compositor, response: Option<Vec<lsp::Location>>| {
            let items = response.unwrap_or_default();
            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

pub fn signature_help(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();

    let pos = doc.position(view.id, offset_encoding);

    let future = language_server.text_document_signature_help(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |_editor, _compositor, response: Option<lsp::SignatureHelp>| {
            if let Some(signature_help) = response {
                log::info!("{:?}", signature_help);
                // signatures
                // active_signature
                // active_parameter
                // render as:

                // signature
                // ----------
                // doc

                // with active param highlighted
            }
        },
    );
}
pub fn hover(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = language_server!(cx.editor, doc);
    let offset_encoding = language_server.offset_encoding();

    // TODO: factor out a doc.position_identifier() that returns lsp::TextDocumentPositionIdentifier

    let pos = doc.position(view.id, offset_encoding);

    let future = language_server.text_document_hover(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor: &mut Editor, compositor: &mut Compositor, response: Option<lsp::Hover>| {
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
    let prompt = Prompt::new(
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

            let task = language_server.rename_symbol(doc.identifier(), pos, input.to_string());
            let edits = block_on(task).unwrap_or_default();
            apply_workspace_edit(cx.editor, offset_encoding, &edits);
        },
    );
    cx.push_layer(Box::new(prompt));
}
