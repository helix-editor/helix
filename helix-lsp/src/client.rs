use crate::{
    transport::{Payload, Transport},
    Error, Notification,
};

type Result<T> = core::result::Result<T, Error>;

use helix_core::{State, Transaction};

// use std::collections::HashMap;

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

pub struct Client {
    _process: Child,
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
            _process: process,
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

    pub async fn request<R: lsp::request::Request>(
        &mut self,
        params: R::Params,
    ) -> Result<R::Result>
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

    pub async fn notify<R: lsp::notification::Notification>(
        &mut self,
        params: R::Params,
    ) -> Result<()>
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

    // -------------------------------------------------------------------------------------------
    // General messages
    // -------------------------------------------------------------------------------------------

    pub async fn initialize(&mut self) -> Result<()> {
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

    pub async fn shutdown(&mut self) -> Result<()> {
        self.request::<lsp::request::Shutdown>(()).await
    }

    pub async fn exit(&mut self) -> Result<()> {
        self.notify::<lsp::notification::Exit>(()).await
    }

    // -------------------------------------------------------------------------------------------
    // Text document
    // -------------------------------------------------------------------------------------------

    pub async fn text_document_did_open(&mut self, state: &State) -> Result<()> {
        self.notify::<lsp::notification::DidOpenTextDocument>(lsp::DidOpenTextDocumentParams {
            text_document: lsp::TextDocumentItem {
                uri: lsp::Url::from_file_path(state.path().unwrap()).unwrap(),
                language_id: "rust".to_string(), // TODO: hardcoded for now
                version: state.version,
                text: String::from(&state.doc),
            },
        })
        .await
    }

    // TODO: trigger any time history.commit_revision happens
    pub async fn text_document_did_change(
        &mut self,
        state: &State,
        transaction: &Transaction,
    ) -> Result<()> {
        self.notify::<lsp::notification::DidChangeTextDocument>(lsp::DidChangeTextDocumentParams {
            text_document: lsp::VersionedTextDocumentIdentifier::new(
                lsp::Url::from_file_path(state.path().unwrap()).unwrap(),
                state.version,
            ),
            content_changes: vec![lsp::TextDocumentContentChangeEvent {
                // range = None -> whole document
                range: None,        //Some(Range)
                range_length: None, // u64 apparently deprecated
                text: "".to_string(),
            }], // TODO: probably need old_state here too?
        })
        .await
    }

    // TODO: impl into() TextDocumentIdentifier / VersionedTextDocumentIdentifier for State.

    pub async fn text_document_did_close(&mut self, state: &State) -> Result<()> {
        self.notify::<lsp::notification::DidCloseTextDocument>(lsp::DidCloseTextDocumentParams {
            text_document: lsp::TextDocumentIdentifier::new(
                lsp::Url::from_file_path(state.path().unwrap()).unwrap(),
            ),
        })
        .await
    }

    // will_save / will_save_wait_until

    pub async fn text_document_did_save(&mut self) -> anyhow::Result<()> {
        unimplemented!()
    }
}
