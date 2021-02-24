use crate::{
    transport::{Payload, Transport},
    Call, Error,
};

type Result<T> = core::result::Result<T, Error>;

use helix_core::{ChangeSet, Rope};

// use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use jsonrpc_core as jsonrpc;
use lsp_types as lsp;
use serde_json::Value;

use smol::{
    channel::{Receiver, Sender},
    io::{BufReader, BufWriter},
    // prelude::*,
    process::{Child, Command, Stdio},
    Executor,
};

pub struct Client {
    _process: Child,

    outgoing: Sender<Payload>,
    // pub incoming: Receiver<Call>,
    pub request_counter: AtomicU64,

    capabilities: Option<lsp::ServerCapabilities>,
    // TODO: handle PublishDiagnostics Version
    // diagnostics: HashMap<lsp::Url, Vec<lsp::Diagnostic>>,
}

impl Client {
    pub fn start(ex: &Executor, cmd: &str, args: &[String]) -> (Self, Receiver<Call>) {
        let mut process = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start language server");
        // smol makes sure the process is reaped on drop, but using kill_on_drop(true) maybe?

        // TODO: do we need bufreader/writer here? or do we use async wrappers on unblock?
        let writer = BufWriter::new(process.stdin.take().expect("Failed to open stdin"));
        let reader = BufReader::new(process.stdout.take().expect("Failed to open stdout"));
        let stderr = BufReader::new(process.stderr.take().expect("Failed to open stderr"));

        let (incoming, outgoing) = Transport::start(ex, reader, writer, stderr);

        let client = Client {
            _process: process,

            outgoing,
            // incoming,
            request_counter: AtomicU64::new(0),

            capabilities: None,
            // diagnostics: HashMap::new(),
        };

        // TODO: async client.initialize()
        // maybe use an arc<atomic> flag

        (client, incoming)
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

    /// Execute a RPC request on the language server.
    pub async fn request<R: lsp::request::Request>(&self, params: R::Params) -> Result<R::Result>
    where
        R::Params: serde::Serialize,
        R::Result: core::fmt::Debug, // TODO: temporary
    {
        let params = serde_json::to_value(params)?;

        let request = jsonrpc::MethodCall {
            jsonrpc: Some(jsonrpc::Version::V2),
            id: self.next_request_id(),
            method: R::METHOD.to_string(),
            params: Self::value_into_params(params),
        };

        let (tx, rx) = smol::channel::bounded::<Result<Value>>(1);

        self.outgoing
            .send(Payload::Request {
                chan: tx,
                value: request,
            })
            .await
            .map_err(|e| Error::Other(e.into()))?;

        use smol_timeout::TimeoutExt;
        use std::time::Duration;

        let response = match rx.recv().timeout(Duration::from_secs(2)).await {
            Some(response) => response,
            None => return Err(Error::Timeout),
        }
        .map_err(|e| Error::Other(e.into()))??;

        let response = serde_json::from_value(response)?;

        Ok(response)
    }

    /// Send a RPC notification to the language server.
    pub async fn notify<R: lsp::notification::Notification>(&self, params: R::Params) -> Result<()>
    where
        R::Params: serde::Serialize,
    {
        let params = serde_json::to_value(params)?;

        let notification = jsonrpc::Notification {
            jsonrpc: Some(jsonrpc::Version::V2),
            method: R::METHOD.to_string(),
            params: Self::value_into_params(params),
        };

        self.outgoing
            .send(Payload::Notification(notification))
            .await
            .map_err(|e| Error::Other(e.into()))?;

        Ok(())
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
            .await
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
                            snippet_support: Some(false), // TODO
                            ..Default::default()
                        }),
                        completion_item_kind: Some(lsp::CompletionItemKindCapability {
                            ..Default::default()
                        }),
                        context_support: None, // additional context information Some(true)
                        ..Default::default()
                    }),
                    // { completion: {
                    //      dynamic_registration: bool
                    //      completion_item: { snippet, documentation_format, ... }
                    //      completion_item_kind: {  }
                    // } }
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
    ) -> Result<()> {
        self.notify::<lsp::notification::DidOpenTextDocument>(lsp::DidOpenTextDocumentParams {
            text_document: lsp::TextDocumentItem {
                uri,
                language_id: "rust".to_string(), // TODO: hardcoded for now
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

            // TODO: there should be a better way here
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
        let new_text = new_text.slice(..);

        // TODO: verify this function, specifically line num counting

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
                    let start = pos_to_lsp_pos(new_text, new_pos);
                    let end = traverse(start, old_text.slice(old_pos..old_end));

                    // deletion
                    changes.push(lsp::TextDocumentContentChangeEvent {
                        range: Some(lsp::Range::new(start, end)),
                        text: "".to_string(),
                        range_length: None,
                    });
                }
                Insert(s) => {
                    let start = pos_to_lsp_pos(new_text, new_pos);

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

    // TODO: trigger any time history.commit_revision happens
    pub async fn text_document_did_change(
        &self,
        text_document: lsp::VersionedTextDocumentIdentifier,
        old_text: &Rope,
        new_text: &Rope,
        changes: &ChangeSet,
    ) -> Result<()> {
        // figure out what kind of sync the server supports

        let capabilities = self.capabilities.as_ref().unwrap(); // TODO: needs post init

        let sync_capabilities = match capabilities.text_document_sync {
            Some(lsp::TextDocumentSyncCapability::Kind(kind)) => kind,
            Some(lsp::TextDocumentSyncCapability::Options(lsp::TextDocumentSyncOptions {
                change: Some(kind),
                ..
            })) => kind,
            // None | SyncOptions { changes: None }
            _ => return Ok(()),
        };

        let changes = match sync_capabilities {
            lsp::TextDocumentSyncKind::Full => {
                vec![lsp::TextDocumentContentChangeEvent {
                    // range = None -> whole document
                    range: None,        //Some(Range)
                    range_length: None, // u64 apparently deprecated
                    text: "".to_string(),
                }] // TODO: probably need old_state here too?
            }
            lsp::TextDocumentSyncKind::Incremental => {
                Self::changeset_to_changes(old_text, new_text, changes)
            }
            lsp::TextDocumentSyncKind::None => return Ok(()),
        };

        self.notify::<lsp::notification::DidChangeTextDocument>(lsp::DidChangeTextDocumentParams {
            text_document,
            content_changes: changes,
        })
        .await
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
    ) -> Result<()> {
        self.notify::<lsp::notification::DidSaveTextDocument>(lsp::DidSaveTextDocumentParams {
            text_document,
            text: None, // TODO:
        })
        .await
    }

    pub async fn completion(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
    ) -> Result<Vec<lsp::CompletionItem>> {
        // TODO: figure out what should happen when you complete with multiple cursors

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

        let response = self.request::<lsp::request::Completion>(params).await?;

        let items = match response {
            Some(lsp::CompletionResponse::Array(items)) => items,
            // TODO: do something with is_incomplete
            Some(lsp::CompletionResponse::List(lsp::CompletionList {
                is_incomplete: _is_incomplete,
                items,
            })) => items,
            None => Vec::new(),
        };

        Ok(items)
    }
}
