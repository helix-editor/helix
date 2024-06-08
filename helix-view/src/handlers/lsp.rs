use crate::editor::Action;
use crate::Editor;
use crate::{DocumentId, ViewId};
use helix_lsp::util::generate_transaction_from_edits;
use helix_lsp::{lsp, OffsetEncoding};

pub enum CompletionEvent {
    /// Auto completion was triggered by typing a word char
    AutoTrigger {
        cursor: usize,
        doc: DocumentId,
        view: ViewId,
    },
    /// Auto completion was triggered by typing a trigger char
    /// specified by the LSP
    TriggerChar {
        cursor: usize,
        doc: DocumentId,
        view: ViewId,
    },
    /// A completion was manually requested (c-x)
    ManualTrigger {
        cursor: usize,
        doc: DocumentId,
        view: ViewId,
    },
    /// Some text was deleted and the cursor is now at `pos`
    DeleteText { cursor: usize },
    /// Invalidate the current auto completion trigger
    Cancel,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SignatureHelpInvoked {
    Automatic,
    Manual,
}

pub enum SignatureHelpEvent {
    Invoked,
    Trigger,
    ReTrigger,
    Cancel,
    RequestComplete { open: bool },
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

impl Editor {
    fn apply_text_edits(
        &mut self,
        uri: &helix_lsp::Url,
        version: Option<i32>,
        text_edits: Vec<lsp::TextEdit>,
        offset_encoding: OffsetEncoding,
    ) -> Result<(), ApplyEditErrorKind> {
        let path = match uri.to_file_path() {
            Ok(path) => path,
            Err(_) => {
                let err = format!("unable to convert URI to filepath: {}", uri);
                log::error!("{}", err);
                self.set_error(err);
                return Err(ApplyEditErrorKind::UnknownURISchema);
            }
        };

        let doc_id = match self.open(&path, Action::Load) {
            Ok(doc_id) => doc_id,
            Err(err) => {
                let err = format!("failed to open document: {}: {}", uri, err);
                log::error!("{}", err);
                self.set_error(err);
                return Err(ApplyEditErrorKind::FileNotFound);
            }
        };

        let doc = doc_mut!(self, &doc_id);
        if let Some(version) = version {
            if version != doc.version() {
                let err = format!("outdated workspace edit for {path:?}");
                log::error!("{err}, expected {} but got {version}", doc.version());
                self.set_error(err);
                return Err(ApplyEditErrorKind::DocumentChanged);
            }
        }

        // Need to determine a view for apply/append_changes_to_history
        let view_id = self.get_synced_view_id(doc_id);
        let doc = doc_mut!(self, &doc_id);

        let transaction = generate_transaction_from_edits(doc.text(), text_edits, offset_encoding);
        let view = view_mut!(self, view_id);
        doc.apply(&transaction, view.id);
        doc.append_changes_to_history(view);
        Ok(())
    }

    // TODO make this transactional (and set failureMode to transactional)
    pub fn apply_workspace_edit(
        &mut self,
        offset_encoding: OffsetEncoding,
        workspace_edit: &lsp::WorkspaceEdit,
    ) -> Result<(), ApplyEditError> {
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
                        self.apply_text_edits(
                            &document_edit.text_document.uri,
                            document_edit.text_document.version,
                            edits,
                            offset_encoding,
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
                                self.apply_document_resource_op(op).map_err(|io| {
                                    ApplyEditError {
                                        kind: ApplyEditErrorKind::IoError(io),
                                        failed_change_idx: i,
                                    }
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
                                self.apply_text_edits(
                                    &document_edit.text_document.uri,
                                    document_edit.text_document.version,
                                    edits,
                                    offset_encoding,
                                )
                                .map_err(|kind| {
                                    ApplyEditError {
                                        kind,
                                        failed_change_idx: i,
                                    }
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
                self.apply_text_edits(uri, None, text_edits, offset_encoding)
                    .map_err(|kind| ApplyEditError {
                        kind,
                        failed_change_idx: i,
                    })?;
            }
        }

        Ok(())
    }

    fn apply_document_resource_op(&mut self, op: &lsp::ResourceOp) -> std::io::Result<()> {
        use lsp::ResourceOp;
        use std::fs;
        match op {
            ResourceOp::Create(op) => {
                let path = op.uri.to_file_path().unwrap();
                let ignore_if_exists = op.options.as_ref().map_or(false, |options| {
                    !options.overwrite.unwrap_or(false) && options.ignore_if_exists.unwrap_or(false)
                });
                if !ignore_if_exists || !path.exists() {
                    // Create directory if it does not exist
                    if let Some(dir) = path.parent() {
                        if !dir.is_dir() {
                            fs::create_dir_all(dir)?;
                        }
                    }

                    fs::write(&path, [])?;
                    self.language_servers.file_event_handler.file_changed(path);
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
                        fs::remove_dir_all(&path)?
                    } else {
                        fs::remove_dir(&path)?
                    }
                    self.language_servers.file_event_handler.file_changed(path);
                } else if path.is_file() {
                    fs::remove_file(&path)?;
                }
            }
            ResourceOp::Rename(op) => {
                let from = op.old_uri.to_file_path().unwrap();
                let to = op.new_uri.to_file_path().unwrap();
                let ignore_if_exists = op.options.as_ref().map_or(false, |options| {
                    !options.overwrite.unwrap_or(false) && options.ignore_if_exists.unwrap_or(false)
                });
                if !ignore_if_exists || !to.exists() {
                    self.move_path(&from, &to)?;
                }
            }
        }
        Ok(())
    }
}
