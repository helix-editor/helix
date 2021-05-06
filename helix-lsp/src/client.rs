use crate::{
    transport::{Payload, Transport},
    Call, Error, OffsetEncoding, Result,
};

use helix_core::{ChangeSet, Rope};

// use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};

use jsonrpc_core as jsonrpc;
use lsp_types as lsp;
use serde_json::Value;

use std::process::Stdio;
use tokio::{
    io::{BufReader, BufWriter},
    // prelude::*,
    process::{Child, Command},
    sync::mpsc::{channel, UnboundedReceiver, UnboundedSender},
};

pub struct Client {
    _process: Child,

    outgoing: UnboundedSender<Payload>,
    // pub incoming: Receiver<Call>,
    pub request_counter: AtomicU64,

    capabilities: Option<lsp::ServerCapabilities>,
    offset_encoding: OffsetEncoding,
}

impl Client {
    pub fn start(cmd: &str, args: &[String]) -> Result<(Self, UnboundedReceiver<Call>)> {
        let process = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // make sure the process is reaped on drop
            .kill_on_drop(true)
            .spawn();

        // use std::io::ErrorKind;
        let mut process = match process {
            Ok(process) => process,
            Err(err) => match err.kind() {
                // ErrorKind::NotFound | ErrorKind::PermissionDenied => {
                //     return Err(Error::Other(err.into()))
                // }
                _kind => return Err(Error::Other(err.into())),
            },
        };

        // TODO: do we need bufreader/writer here? or do we use async wrappers on unblock?
        let writer = BufWriter::new(process.stdin.take().expect("Failed to open stdin"));
        let reader = BufReader::new(process.stdout.take().expect("Failed to open stdout"));
        let stderr = BufReader::new(process.stderr.take().expect("Failed to open stderr"));

        let (incoming, outgoing) = Transport::start(reader, writer, stderr);

        let client = Client {
            _process: process,

            outgoing,
            // incoming,
            request_counter: AtomicU64::new(0),

            capabilities: None,
            // diagnostics: HashMap::new(),
            offset_encoding: OffsetEncoding::Utf8,
        };

        // TODO: async client.initialize()
        // maybe use an arc<atomic> flag

        Ok((client, incoming))
    }

    fn next_request_id(&self) -> jsonrpc::Id {
        let id = self.request_counter.fetch_add(1, Ordering::Relaxed);
        jsonrpc::Id::Num(id)
    }

    fn value_into_params(value: Value) -> jsonrpc::Params {
        use jsonrpc::Params;

        match value {
            Value::Null => Params::None,
            Value::Bool(_) | Value::Number(_) | Value::String(_) => Params::Array(vec![value]),
            Value::Array(vec) => Params::Array(vec),
            Value::Object(map) => Params::Map(map),
        }
    }

    pub fn capabilities(&self) -> &lsp::ServerCapabilities {
        self.capabilities
            .as_ref()
            .expect("language server not yet initialized!")
    }

    pub fn offset_encoding(&self) -> OffsetEncoding {
        self.offset_encoding
    }

    /// Execute a RPC request on the language server.
    pub async fn request<R: lsp::request::Request>(&self, params: R::Params) -> Result<R::Result>
    where
        R::Params: serde::Serialize,
        R::Result: core::fmt::Debug, // TODO: temporary
    {
        // a future that resolves into the response
        let future = self.call::<R>(params).await?;
        let json = future.await?;
        let response = serde_json::from_value(json)?;
        Ok(response)
    }

    /// Execute a RPC request on the language server.
    pub async fn call<R: lsp::request::Request>(
        &self,
        params: R::Params,
    ) -> Result<impl Future<Output = Result<Value>>>
    where
        R::Params: serde::Serialize,
    {
        let params = serde_json::to_value(params)?;

        let request = jsonrpc::MethodCall {
            jsonrpc: Some(jsonrpc::Version::V2),
            id: self.next_request_id(),
            method: R::METHOD.to_string(),
            params: Self::value_into_params(params),
        };

        let (tx, mut rx) = channel::<Result<Value>>(1);

        self.outgoing
            .send(Payload::Request {
                chan: tx,
                value: request,
            })
            .map_err(|e| Error::Other(e.into()))?;

        use std::time::Duration;
        use tokio::time::timeout;

        let future = async move {
            timeout(Duration::from_secs(2), rx.recv())
                .await
                .map_err(|e| Error::Timeout)? // return Timeout
                .unwrap() // TODO: None if channel closed
        };

        Ok(future)
    }

    /// Send a RPC notification to the language server.
    pub fn notify<R: lsp::notification::Notification>(
        &self,
        params: R::Params,
    ) -> impl Future<Output = Result<()>>
    where
        R::Params: serde::Serialize,
    {
        let outgoing = self.outgoing.clone();

        async move {
            let params = serde_json::to_value(params)?;

            let notification = jsonrpc::Notification {
                jsonrpc: Some(jsonrpc::Version::V2),
                method: R::METHOD.to_string(),
                params: Self::value_into_params(params),
            };

            outgoing
                .send(Payload::Notification(notification))
                .map_err(|e| Error::Other(e.into()))?;

            Ok(())
        }
    }

    /// Reply to a language server RPC call.
    pub async fn reply(
        &self,
        id: jsonrpc::Id,
        result: core::result::Result<Value, jsonrpc::Error>,
    ) -> Result<()> {
        use jsonrpc::{Failure, Output, Success, Version};

        let output = match result {
            Ok(result) => Output::Success(Success {
                jsonrpc: Some(Version::V2),
                id,
                result,
            }),
            Err(error) => Output::Failure(Failure {
                jsonrpc: Some(Version::V2),
                id,
                error,
            }),
        };

        self.outgoing
            .send(Payload::Response(output))
            .map_err(|e| Error::Other(e.into()))?;

        Ok(())
    }

    // -------------------------------------------------------------------------------------------
    // General messages
    // -------------------------------------------------------------------------------------------

    pub async fn initialize(&mut self) -> Result<()> {
        // TODO: delay any requests that are triggered prior to initialize

        #[allow(deprecated)]
        let params = lsp::InitializeParams {
            process_id: Some(std::process::id()),
            root_path: None,
            // root_uri: Some(lsp_types::Url::parse("file://localhost/")?),
            root_uri: None, // set to project root in the future
            initialization_options: None,
            capabilities: lsp::ClientCapabilities {
                text_document: Some(lsp::TextDocumentClientCapabilities {
                    completion: Some(lsp::CompletionClientCapabilities {
                        completion_item: Some(lsp::CompletionItemCapability {
                            snippet_support: Some(false),
                            ..Default::default()
                        }),
                        completion_item_kind: Some(lsp::CompletionItemKindCapability {
                            ..Default::default()
                        }),
                        context_support: None, // additional context information Some(true)
                        ..Default::default()
                    }),
                    hover: Some(lsp::HoverClientCapabilities {
                        // if not specified, rust-analyzer returns plaintext marked as markdown but
                        // badly formatted.
                        content_format: Some(vec![lsp::MarkupKind::Markdown]),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            trace: None,
            workspace_folders: None,
            client_info: None,
            locale: None, // TODO
        };

        let response = self.request::<lsp::request::Initialize>(params).await?;
        self.capabilities = Some(response.capabilities);

        // next up, notify<initialized>
        self.notify::<lsp::notification::Initialized>(lsp::InitializedParams {})
            .await?;

        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.request::<lsp::request::Shutdown>(()).await
    }

    pub async fn exit(&self) -> Result<()> {
        self.notify::<lsp::notification::Exit>(()).await
    }

    // -------------------------------------------------------------------------------------------
    // Text document
    // -------------------------------------------------------------------------------------------

    pub async fn text_document_did_open(
        &self,
        uri: lsp::Url,
        version: i32,
        doc: &Rope,
        language_id: String,
    ) -> Result<()> {
        self.notify::<lsp::notification::DidOpenTextDocument>(lsp::DidOpenTextDocumentParams {
            text_document: lsp::TextDocumentItem {
                uri,
                language_id,
                version,
                text: String::from(doc),
            },
        })
        .await
    }

    pub fn changeset_to_changes(
        old_text: &Rope,
        new_text: &Rope,
        changeset: &ChangeSet,
        offset_encoding: OffsetEncoding,
    ) -> Vec<lsp::TextDocumentContentChangeEvent> {
        let mut iter = changeset.changes().iter().peekable();
        let mut old_pos = 0;
        let mut new_pos = 0;

        let mut changes = Vec::new();

        use crate::util::pos_to_lsp_pos;
        use helix_core::Operation::*;

        // this is dumb. TextEdit describes changes to the initial doc (concurrent), but
        // TextDocumentContentChangeEvent describes a series of changes (sequential).
        // So S -> S1 -> S2, meaning positioning depends on the previous edits.
        //
        // Calculation is therefore a bunch trickier.

        // TODO: stolen from syntax.rs, share
        use helix_core::RopeSlice;
        fn traverse(pos: lsp::Position, text: RopeSlice) -> lsp::Position {
            let lsp::Position {
                mut line,
                mut character,
            } = pos;

            for ch in text.chars() {
                if ch == '\n' {
                    line += 1;
                    character = 0;
                } else {
                    character += ch.len_utf16() as u32;
                }
            }
            lsp::Position { line, character }
        }

        let old_text = old_text.slice(..);

        while let Some(change) = iter.next() {
            let len = match change {
                Delete(i) | Retain(i) => *i,
                Insert(_) => 0,
            };
            let mut old_end = old_pos + len;

            match change {
                Retain(i) => {
                    new_pos += i;
                }
                Delete(_) => {
                    let start = pos_to_lsp_pos(new_text, new_pos, offset_encoding);
                    let end = traverse(start, old_text.slice(old_pos..old_end));

                    // deletion
                    changes.push(lsp::TextDocumentContentChangeEvent {
                        range: Some(lsp::Range::new(start, end)),
                        text: "".to_string(),
                        range_length: None,
                    });
                }
                Insert(s) => {
                    let start = pos_to_lsp_pos(new_text, new_pos, offset_encoding);

                    new_pos += s.chars().count();

                    // a subsequent delete means a replace, consume it
                    let end = if let Some(Delete(len)) = iter.peek() {
                        old_end = old_pos + len;
                        let end = traverse(start, old_text.slice(old_pos..old_end));

                        iter.next();

                        // replacement
                        end
                    } else {
                        // insert
                        start
                    };

                    changes.push(lsp::TextDocumentContentChangeEvent {
                        range: Some(lsp::Range::new(start, end)),
                        text: s.into(),
                        range_length: None,
                    });
                }
            }
            old_pos = old_end;
        }

        changes
    }

    pub fn text_document_did_change(
        &self,
        text_document: lsp::VersionedTextDocumentIdentifier,
        old_text: &Rope,
        new_text: &Rope,
        changes: &ChangeSet,
    ) -> Option<impl Future<Output = Result<()>>> {
        // figure out what kind of sync the server supports

        let capabilities = self.capabilities.as_ref().unwrap();

        let sync_capabilities = match capabilities.text_document_sync {
            Some(lsp::TextDocumentSyncCapability::Kind(kind))
            | Some(lsp::TextDocumentSyncCapability::Options(lsp::TextDocumentSyncOptions {
                change: Some(kind),
                ..
            })) => kind,
            // None | SyncOptions { changes: None }
            _ => return None,
        };

        let changes = match sync_capabilities {
            lsp::TextDocumentSyncKind::Full => {
                vec![lsp::TextDocumentContentChangeEvent {
                    // range = None -> whole document
                    range: None,        //Some(Range)
                    range_length: None, // u64 apparently deprecated
                    text: "".to_string(),
                }]
            }
            lsp::TextDocumentSyncKind::Incremental => {
                Self::changeset_to_changes(old_text, new_text, changes, self.offset_encoding)
            }
            lsp::TextDocumentSyncKind::None => return None,
        };

        Some(self.notify::<lsp::notification::DidChangeTextDocument>(
            lsp::DidChangeTextDocumentParams {
                text_document,
                content_changes: changes,
            },
        ))
    }

    pub async fn text_document_did_close(
        &self,
        text_document: lsp::TextDocumentIdentifier,
    ) -> Result<()> {
        self.notify::<lsp::notification::DidCloseTextDocument>(lsp::DidCloseTextDocumentParams {
            text_document,
        })
        .await
    }

    // will_save / will_save_wait_until

    pub async fn text_document_did_save(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        text: &Rope,
    ) -> Result<()> {
        let capabilities = self.capabilities.as_ref().unwrap();

        let include_text = match &capabilities.text_document_sync {
            Some(lsp::TextDocumentSyncCapability::Options(lsp::TextDocumentSyncOptions {
                save: Some(options),
                ..
            })) => match options {
                lsp::TextDocumentSyncSaveOptions::Supported(true) => false,
                lsp::TextDocumentSyncSaveOptions::SaveOptions(lsp_types::SaveOptions {
                    include_text,
                }) => include_text.unwrap_or(false),
                // Supported(false)
                _ => return Ok(()),
            },
            // unsupported
            _ => return Ok(()),
        };

        self.notify::<lsp::notification::DidSaveTextDocument>(lsp::DidSaveTextDocumentParams {
            text_document,
            text: include_text.then(|| text.into()),
        })
        .await
    }

    pub async fn completion(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
    ) -> Result<impl Future<Output = Result<Value>>> {
        // ) -> Result<Vec<lsp::CompletionItem>> {
        let params = lsp::CompletionParams {
            text_document_position: lsp::TextDocumentPositionParams {
                text_document,
                position,
            },
            // TODO: support these tokens by async receiving and updating the choice list
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: lsp::PartialResultParams {
                partial_result_token: None,
            },
            context: None,
            // lsp::CompletionContext { trigger_kind: , trigger_character: Some(), }
        };

        self.call::<lsp::request::Completion>(params).await
    }

    pub async fn text_document_signature_help(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
    ) -> anyhow::Result<Option<lsp::SignatureHelp>> {
        let params = lsp::SignatureHelpParams {
            text_document_position_params: lsp::TextDocumentPositionParams {
                text_document,
                position,
            },
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: None,
            },
            context: None,
            // lsp::SignatureHelpContext
        };

        let response = self
            .request::<lsp::request::SignatureHelpRequest>(params)
            .await?;

        Ok(response)
    }

    pub async fn text_document_hover(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
    ) -> anyhow::Result<Option<lsp::Hover>> {
        let params = lsp::HoverParams {
            text_document_position_params: lsp::TextDocumentPositionParams {
                text_document,
                position,
            },
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: None,
            },
            // lsp::SignatureHelpContext
        };

        let response = self.request::<lsp::request::HoverRequest>(params).await?;

        Ok(response)
    }

    // formatting

    pub async fn text_document_formatting(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        options: lsp::FormattingOptions,
    ) -> anyhow::Result<Vec<lsp::TextEdit>> {
        let capabilities = self.capabilities.as_ref().unwrap();

        // check if we're able to format
        match capabilities.document_formatting_provider {
            Some(lsp::OneOf::Left(true)) | Some(lsp::OneOf::Right(_)) => (),
            // None | Some(false)
            _ => return Ok(Vec::new()),
        };
        // TODO: return err::unavailable so we can fall back to tree sitter formatting

        let params = lsp::DocumentFormattingParams {
            text_document,
            options,
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: None,
            },
        };

        let response = self.request::<lsp::request::Formatting>(params).await?;

        Ok(response.unwrap_or_default())
    }

    pub async fn text_document_range_formatting(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        range: lsp::Range,
        options: lsp::FormattingOptions,
    ) -> anyhow::Result<Vec<lsp::TextEdit>> {
        let capabilities = self.capabilities.as_ref().unwrap();

        // check if we're able to format
        match capabilities.document_range_formatting_provider {
            Some(lsp::OneOf::Left(true)) | Some(lsp::OneOf::Right(_)) => (),
            // None | Some(false)
            _ => return Ok(Vec::new()),
        };
        // TODO: return err::unavailable so we can fall back to tree sitter formatting

        let params = lsp::DocumentRangeFormattingParams {
            text_document,
            range,
            options,
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: None,
            },
        };

        let response = self
            .request::<lsp::request::RangeFormatting>(params)
            .await?;

        Ok(response.unwrap_or_default())
    }

    async fn goto_request<
        T: lsp::request::Request<
            Params = lsp::GotoDefinitionParams,
            Result = Option<lsp::GotoDefinitionResponse>,
        >,
    >(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
    ) -> anyhow::Result<Vec<lsp::Location>> {
        let params = lsp::GotoDefinitionParams {
            text_document_position_params: lsp::TextDocumentPositionParams {
                text_document,
                position,
            },
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: lsp::PartialResultParams {
                partial_result_token: None,
            },
        };

        let response = self.request::<T>(params).await?;

        let items = match response {
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
        };

        Ok(items)
    }

    pub async fn goto_definition(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
    ) -> anyhow::Result<Vec<lsp::Location>> {
        self.goto_request::<lsp::request::GotoDefinition>(text_document, position)
            .await
    }

    pub async fn goto_type_definition(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
    ) -> anyhow::Result<Vec<lsp::Location>> {
        self.goto_request::<lsp::request::GotoTypeDefinition>(text_document, position)
            .await
    }

    pub async fn goto_implementation(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
    ) -> anyhow::Result<Vec<lsp::Location>> {
        self.goto_request::<lsp::request::GotoImplementation>(text_document, position)
            .await
    }

    pub async fn goto_reference(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
    ) -> anyhow::Result<Vec<lsp::Location>> {
        let params = lsp::ReferenceParams {
            text_document_position: lsp::TextDocumentPositionParams {
                text_document,
                position,
            },
            context: lsp::ReferenceContext {
                include_declaration: true,
            },
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: lsp::PartialResultParams {
                partial_result_token: None,
            },
        };

        let response = self.request::<lsp::request::References>(params).await?;

        Ok(response.unwrap_or_default())
    }
}
