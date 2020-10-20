mod transport;

use transport::{Payload, Transport};

// use std::collections::HashMap;

use jsonrpc_core as jsonrpc;
use lsp_types as lsp;
use serde_json::Value;

use serde::{Deserialize, Serialize};

pub use lsp::Position;
pub use lsp::Url;

use smol::prelude::*;
use smol::{
    channel::{Receiver, Sender},
    io::{BufReader, BufWriter},
    process::{Child, ChildStderr, Command, Stdio},
    Executor,
};

/// A type representing all possible values sent from the server to the client.
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
enum Message {
    /// A regular JSON-RPC request output (single response).
    Output(jsonrpc::Output),
    /// A notification.
    Notification(jsonrpc::Notification),
    /// A JSON-RPC request
    Call(jsonrpc::Call),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Notification {
    PublishDiagnostics(lsp::PublishDiagnosticsParams),
}

impl Notification {
    pub fn parse(method: &str, params: jsonrpc::Params) -> Notification {
        use lsp::notification::Notification as _;

        match method {
            lsp::notification::PublishDiagnostics::METHOD => {
                let params: lsp::PublishDiagnosticsParams = params
                    .parse()
                    .expect("Failed to parse PublishDiagnostics params");

                // TODO: need to loop over diagnostics and distinguish them by URI
                Notification::PublishDiagnostics(params)
            }
            _ => unimplemented!("unhandled notification: {}", method),
        }
    }
}

pub struct Client {
    process: Child,
    stderr: BufReader<ChildStderr>,

    outgoing: Sender<Payload>,
    pub incoming: Receiver<Notification>,

    pub request_counter: u64,

    capabilities: Option<lsp::ServerCapabilities>,
    // TODO: handle PublishDiagnostics Version
    // diagnostics: HashMap<lsp::Url, Vec<lsp::Diagnostic>>,
}

impl Client {
    pub fn start(ex: &Executor, cmd: &str, args: &[String]) -> Self {
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

        Client {
            process,
            stderr,

            outgoing,
            incoming,

            request_counter: 0,

            capabilities: None,
            // diagnostics: HashMap::new(),
        }
    }

    fn next_request_id(&mut self) -> jsonrpc::Id {
        let id = jsonrpc::Id::Num(self.request_counter);
        self.request_counter += 1;
        id
    }

    fn to_params(value: Value) -> anyhow::Result<jsonrpc::Params> {
        use jsonrpc::Params;

        let params = match value {
            Value::Null => Params::None,
            Value::Bool(_) | Value::Number(_) | Value::String(_) => Params::Array(vec![value]),
            Value::Array(vec) => Params::Array(vec),
            Value::Object(map) => Params::Map(map),
        };

        Ok(params)
    }

    pub async fn request<R: lsp::request::Request>(
        &mut self,
        params: R::Params,
    ) -> anyhow::Result<R::Result>
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

        let (tx, rx) = smol::channel::bounded::<anyhow::Result<Value>>(1);

        self.outgoing
            .send(Payload::Request {
                chan: tx,
                value: request,
            })
            .await?;

        let response = rx.recv().await??;

        let response = serde_json::from_value(response)?;

        // TODO: we should pass request to a sender thread via a channel
        // so it can't be interleaved

        // TODO: responses can be out of order, we need to register a single shot response channel

        Ok(response)
    }

    pub async fn notify<R: lsp::notification::Notification>(
        &mut self,
        params: R::Params,
    ) -> anyhow::Result<()>
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
            .await?;

        Ok(())
    }

    // -------------------------------------------------------------------------------------------
    // General messages
    // -------------------------------------------------------------------------------------------

    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        // TODO: delay any requests that are triggered prior to initialize

        #[allow(deprecated)]
        let params = lsp::InitializeParams {
            process_id: Some(u64::from(std::process::id())),
            root_path: None,
            // root_uri: Some(lsp_types::Url::parse("file://localhost/")?),
            root_uri: None, // set to project root in the future
            initialization_options: None,
            capabilities: lsp::ClientCapabilities::default(),
            trace: None,
            workspace_folders: None,
            client_info: None,
        };

        let response = self.request::<lsp::request::Initialize>(params).await?;
        self.capabilities = Some(response.capabilities);

        // next up, notify<initialized>
        self.notify::<lsp::notification::Initialized>(lsp::InitializedParams {})
            .await?;

        Ok(())
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.request::<lsp::request::Shutdown>(()).await
    }

    pub async fn exit(&mut self) -> anyhow::Result<()> {
        self.notify::<lsp::notification::Exit>(()).await
    }

    // -------------------------------------------------------------------------------------------
    // Text document
    // -------------------------------------------------------------------------------------------

    pub async fn text_document_did_open(
        &mut self,
        state: &helix_core::State,
    ) -> anyhow::Result<()> {
        self.notify::<lsp::notification::DidOpenTextDocument>(lsp::DidOpenTextDocumentParams {
            text_document: lsp::TextDocumentItem {
                uri: lsp::Url::from_file_path(state.path().unwrap()).unwrap(),
                language_id: "rust".to_string(), // TODO: hardcoded for now
                version: 0,
                text: String::from(&state.doc),
            },
        })
        .await
    }

    // TODO: trigger any time history.commit_revision happens
    pub async fn text_document_did_change(
        &mut self,
        state: &helix_core::State,
    ) -> anyhow::Result<()> {
        self.notify::<lsp::notification::DidChangeTextDocument>(lsp::DidChangeTextDocumentParams {
            text_document: lsp::VersionedTextDocumentIdentifier::new(
                lsp::Url::from_file_path(state.path().unwrap()).unwrap(),
                0, // TODO: version
            ),
            content_changes: vec![], // TODO:
        })
        .await
    }

    pub async fn text_document_did_close(&mut self) -> anyhow::Result<()> {
        unimplemented!()
    }

    // will_save / will_save_wait_until

    pub async fn text_document_did_save(&mut self) -> anyhow::Result<()> {
        unimplemented!()
    }
}
