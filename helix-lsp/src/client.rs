use crate::{
    transport::{Payload, Transport},
    Call, Error,
};

type Result<T> = core::result::Result<T, Error>;

use helix_core::{ChangeSet, Transaction};
use helix_view::Document;

// use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use jsonrpc_core as jsonrpc;
use lsp_types as lsp;
use serde_json::Value;

use smol::{
    channel::{Receiver, Sender},
    io::{BufReader, BufWriter},
    // prelude::*,
    process::{Child, ChildStderr, Command, Stdio},
    Executor,
};

fn text_document_identifier(doc: &Document) -> lsp::TextDocumentIdentifier {
    lsp::TextDocumentIdentifier::new(lsp::Url::from_file_path(doc.path().unwrap()).unwrap())
}

pub struct Client {
    _process: Child,
    stderr: BufReader<ChildStderr>,

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

        let (incoming, outgoing) = Transport::start(ex, reader, writer);

        let client = Client {
            _process: process,
            stderr,

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

    fn to_params(value: Value) -> Result<jsonrpc::Params> {
        use jsonrpc::Params;

        let params = match value {
            Value::Null => Params::None,
            Value::Bool(_) | Value::Number(_) | Value::String(_) => Params::Array(vec![value]),
            Value::Array(vec) => Params::Array(vec),
            Value::Object(map) => Params::Map(map),
        };

        Ok(params)
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
            params: Self::to_params(params)?,
        };

        let (tx, rx) = smol::channel::bounded::<Result<Value>>(1);

        self.outgoing
            .send(Payload::Request {
                chan: tx,
                value: request,
            })
            .await
            .map_err(|e| Error::Other(e.into()))?;

        let response = rx.recv().await.map_err(|e| Error::Other(e.into()))??;

        let response = serde_json::from_value(response)?;

        // TODO: we should pass request to a sender thread via a channel
        // so it can't be interleaved

        // TODO: responses can be out of order, we need to register a single shot response channel

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
            params: Self::to_params(params)?,
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

    pub async fn text_document_did_open(&self, doc: &Document) -> Result<()> {
        self.notify::<lsp::notification::DidOpenTextDocument>(lsp::DidOpenTextDocumentParams {
            text_document: lsp::TextDocumentItem {
                uri: lsp::Url::from_file_path(doc.path().unwrap()).unwrap(),
                language_id: "rust".to_string(), // TODO: hardcoded for now
                version: doc.version,
                text: String::from(doc.text()),
            },
        })
        .await
    }

    fn to_changes(changeset: &ChangeSet) -> Vec<lsp::TextDocumentContentChangeEvent> {
        let mut iter = changeset.changes().iter().peekable();
        let mut old_pos = 0;

        let mut changes = Vec::new();

        use crate::util::pos_to_lsp_pos;
        use helix_core::Operation::*;

        // TEMP
        let rope = helix_core::Rope::from("");
        let old_text = rope.slice(..);

        while let Some(change) = iter.next() {
            let len = match change {
                Delete(i) | Retain(i) => *i,
                Insert(_) => 0,
            };
            let old_end = old_pos + len;

            match change {
                Retain(_) => {}
                Delete(_) => {
                    let start = pos_to_lsp_pos(&old_text, old_pos);
                    let end = pos_to_lsp_pos(&old_text, old_end);

                    // a subsequent ins means a replace, consume it
                    if let Some(Insert(s)) = iter.peek() {
                        iter.next();

                        // replacement
                        changes.push(lsp::TextDocumentContentChangeEvent {
                            range: Some(lsp::Range::new(start, end)),
                            text: s.into(),
                            range_length: None,
                        });
                    } else {
                        // deletion
                        changes.push(lsp::TextDocumentContentChangeEvent {
                            range: Some(lsp::Range::new(start, end)),
                            text: "".to_string(),
                            range_length: None,
                        });
                    };
                }
                Insert(s) => {
                    let start = pos_to_lsp_pos(&old_text, old_pos);

                    // insert
                    changes.push(lsp::TextDocumentContentChangeEvent {
                        range: Some(lsp::Range::new(start, start)),
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
        doc: &Document,
        transaction: &Transaction,
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
            lsp::TextDocumentSyncKind::Incremental => Self::to_changes(transaction.changes()),
            lsp::TextDocumentSyncKind::None => return Ok(()),
        };

        self.notify::<lsp::notification::DidChangeTextDocument>(lsp::DidChangeTextDocumentParams {
            text_document: lsp::VersionedTextDocumentIdentifier::new(
                // TODO: doc.into() Url
                lsp::Url::from_file_path(doc.path().unwrap()).unwrap(),
                doc.version,
            ),
            content_changes: changes,
        })
        .await
    }

    // TODO: impl into() TextDocumentIdentifier / VersionedTextDocumentIdentifier for Document.

    pub async fn text_document_did_close(&self, doc: &Document) -> Result<()> {
        self.notify::<lsp::notification::DidCloseTextDocument>(lsp::DidCloseTextDocumentParams {
            text_document: text_document_identifier(doc),
        })
        .await
    }

    // will_save / will_save_wait_until

    pub async fn text_document_did_save(&self) -> anyhow::Result<()> {
        unimplemented!()
    }
}
