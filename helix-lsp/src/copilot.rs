//! A dedicated client for the GitHub Copilot language server
//! (`copilot-language-server`).
//!
//! Copilot speaks the Language Server Protocol over stdio with the standard
//! `Content-Length` framing, so this module reuses Helix's [`Transport`] for
//! the wire handshake. On top of that it implements the Copilot specific
//! requests and notifications (`checkStatus`, `signIn`, `signOut`,
//! `textDocument/inlineCompletion`, ...) which are not part of the regular LSP
//! surface and therefore do not belong in the general purpose [`crate::Client`].
//!
//! The client is intentionally self contained: it answers the handful of
//! server to client requests Copilot relies on (`workspace/configuration`,
//! `window/workDoneProgress/create`, `window/showDocument`) from an internal
//! background task so that the editor never has to route Copilot traffic
//! through the main language server registry.

use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{BufReader, BufWriter};
use tokio::process::{Child, Command};
use tokio::sync::mpsc::{channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Notify;
use tokio::time::timeout;

use crate::jsonrpc;
use crate::transport::{Payload, Transport};
use crate::{lsp, Error, LanguageServerId, Result};

pub use lsp::{InlineCompletionItem, Url};

/// Default timeout (in seconds) for ordinary Copilot requests.
const REQUEST_TIMEOUT: u64 = 20;
/// Generous timeout for the interactive device-flow sign-in, which only
/// resolves once the user authorizes the device code in their browser.
const SIGN_IN_TIMEOUT: u64 = 1800;

/// The result of a `checkStatus` / `signOut` request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    /// One of `OK`, `MaybeOk`, `NotAuthorized`, `NotSignedIn`.
    pub status: String,
    #[serde(default)]
    pub user: Option<String>,
}

impl StatusResponse {
    pub fn signed_in(&self) -> bool {
        matches!(self.status.as_str(), "OK" | "MaybeOk" | "AlreadySignedIn")
    }
}

/// The result of a `signIn` request. When the account is not yet authorized the
/// server returns a `PromptUserDeviceFlow` payload carrying the device code the
/// user has to enter at [`Self::verification_uri`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignInResponse {
    pub status: String,
    #[serde(default)]
    pub user_code: Option<String>,
    #[serde(default)]
    pub verification_uri: Option<String>,
    #[serde(default)]
    pub expires_in: Option<i64>,
    #[serde(default)]
    pub interval: Option<i64>,
    #[serde(default)]
    pub user: Option<String>,
    /// The follow-up command (`github.copilot.finishDeviceFlow`) that must be
    /// executed to wait for the user to complete the device flow.
    #[serde(default)]
    pub command: Option<lsp::Command>,
}

/// Formatting hints sent alongside an inline completion request.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormattingOptions {
    pub tab_size: u32,
    pub insert_spaces: bool,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
        }
    }
}

#[derive(Debug, Deserialize)]
struct InlineCompletionResult {
    #[serde(default)]
    items: Vec<InlineCompletionItem>,
}

/// A live, initialized connection to a `copilot-language-server` process.
pub struct Client {
    name: String,
    _process: Child,
    server_tx: UnboundedSender<Payload>,
    request_counter: AtomicU64,
    initialize_notify: Arc<Notify>,
    /// The last sign-in status reported by the server via `didChangeStatus`.
    status: Arc<Mutex<Option<String>>>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("copilot::Client")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl Client {
    /// Spawn the Copilot language server and wire up the transport. The returned
    /// client still needs to be [`initialize`](Self::initialize)d before any
    /// other request is issued.
    pub fn start(cmd: &str, args: &[String]) -> Result<Arc<Self>> {
        let cmd = helix_stdx::env::which(cmd)?;
        log::info!("starting copilot language server: {cmd:?}");

        let mut process = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let writer = BufWriter::new(process.stdin.take().expect("Failed to open stdin"));
        let reader = BufReader::new(process.stdout.take().expect("Failed to open stdout"));
        let stderr = BufReader::new(process.stderr.take().expect("Failed to open stderr"));

        let (server_rx, server_tx, initialize_notify, _shutdown_flushed) = Transport::start(
            reader,
            writer,
            stderr,
            LanguageServerId::default(),
            "copilot".to_string(),
        );

        let status = Arc::new(Mutex::new(None));

        // Answer the server-to-client requests Copilot relies on and keep track
        // of status notifications without involving the editor event loop.
        tokio::spawn(Self::run_dispatch(
            server_rx,
            server_tx.clone(),
            status.clone(),
        ));

        Ok(Arc::new(Self {
            name: "copilot".to_string(),
            _process: process,
            server_tx,
            request_counter: AtomicU64::new(0),
            initialize_notify,
            status,
        }))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The last status string the server pushed via `didChangeStatus`, if any.
    pub fn last_status(&self) -> Option<String> {
        self.status.lock().clone()
    }

    fn next_request_id(&self) -> jsonrpc::Id {
        jsonrpc::Id::Num(self.request_counter.fetch_add(1, Ordering::Relaxed))
    }

    fn value_into_params(value: Value) -> jsonrpc::Params {
        match value {
            Value::Null => jsonrpc::Params::None,
            Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                jsonrpc::Params::Array(vec![value])
            }
            Value::Array(vec) => jsonrpc::Params::Array(vec),
            Value::Object(map) => jsonrpc::Params::Map(map),
        }
    }

    fn request<R: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> impl std::future::Future<Output = Result<R>> {
        self.request_with_timeout(method, params, REQUEST_TIMEOUT)
    }

    fn request_with_timeout<R: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
        timeout_secs: u64,
    ) -> impl std::future::Future<Output = Result<R>> {
        let server_tx = self.server_tx.clone();
        let id = self.next_request_id();
        let method = method.to_string();

        // Build and queue the request eagerly so request ordering is preserved
        // even if the returned future is polled later.
        let rx = {
            let (tx, rx) = channel::<Result<Value>>(1);
            let request = jsonrpc::MethodCall {
                jsonrpc: Some(jsonrpc::Version::V2),
                id: id.clone(),
                method,
                params: Self::value_into_params(params),
            };
            server_tx
                .send(Payload::Request {
                    chan: tx,
                    value: request,
                })
                .map_err(|e| Error::Other(e.into()))
                .map(|_| rx)
        };

        async move {
            let mut rx = rx?;
            let value = timeout(Duration::from_secs(timeout_secs), rx.recv())
                .await
                .map_err(|_| Error::Timeout(id))?
                .ok_or(Error::StreamClosed)??;
            serde_json::from_value(value).map_err(Into::into)
        }
    }

    fn notify(&self, method: &str, params: Value) {
        let notification = jsonrpc::Notification {
            jsonrpc: Some(jsonrpc::Version::V2),
            method: method.to_string(),
            params: Self::value_into_params(params),
        };
        if let Err(err) = self.server_tx.send(Payload::Notification(notification)) {
            log::error!("failed to send copilot notification '{method}': {err}");
        }
    }

    // ----------------------------------------------------------------------
    // Lifecycle
    // ----------------------------------------------------------------------

    /// Run the Copilot `initialize`/`initialized` handshake and return the
    /// server capabilities (which include `inlineCompletionProvider` when the
    /// server supports inline completions).
    pub async fn initialize(
        &self,
        editor_name: &str,
        editor_version: &str,
        plugin_version: &str,
        workspace: Option<&Path>,
    ) -> Result<lsp::ServerCapabilities> {
        let (root_uri, workspace_folders) =
            match workspace.and_then(|p| Url::from_file_path(p).ok()) {
                Some(uri) => {
                    let name = workspace
                        .and_then(|p| p.file_name())
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    (
                        Value::String(uri.to_string()),
                        json!([{ "uri": uri.to_string(), "name": name }]),
                    )
                }
                None => (Value::Null, json!([])),
            };

        let params = json!({
            "processId": std::process::id(),
            "clientInfo": { "name": editor_name, "version": editor_version },
            "rootUri": root_uri,
            "workspaceFolders": workspace_folders,
            "capabilities": {
                "workspace": { "workspaceFolders": true, "configuration": true },
                "textDocument": { "inlineCompletion": {} },
                "window": { "workDoneProgress": true, "showDocument": { "support": true } },
            },
            "initializationOptions": {
                "editorInfo": { "name": editor_name, "version": editor_version },
                "editorPluginInfo": { "name": "helix-copilot", "version": plugin_version },
            },
        });

        let result: lsp::InitializeResult = self.request("initialize", params).await?;
        self.notify("initialized", json!({}));
        // Release any requests that were queued while initialization was pending.
        self.initialize_notify.notify_one();
        Ok(result.capabilities)
    }

    pub async fn shutdown(&self) -> Result<()> {
        let _: Value = self.request("shutdown", Value::Null).await?;
        Ok(())
    }

    pub fn exit(&self) {
        self.notify("exit", Value::Null);
    }

    // ----------------------------------------------------------------------
    // Authentication
    // ----------------------------------------------------------------------

    pub async fn check_status(&self) -> Result<StatusResponse> {
        self.request("checkStatus", json!({})).await
    }

    pub async fn sign_in(&self) -> Result<SignInResponse> {
        self.request("signIn", json!({})).await
    }

    pub async fn sign_out(&self) -> Result<StatusResponse> {
        self.request("signOut", json!({})).await
    }

    /// Execute the `github.copilot.finishDeviceFlow` command returned by
    /// [`sign_in`](Self::sign_in). This future only resolves once the user has
    /// authorized the device code in their browser (or the code expires), so it
    /// is given a long timeout.
    pub async fn finish_device_flow(&self) -> Result<Value> {
        self.request_with_timeout(
            "workspace/executeCommand",
            json!({ "command": "github.copilot.finishDeviceFlow", "arguments": [] }),
            SIGN_IN_TIMEOUT,
        )
        .await
    }

    // ----------------------------------------------------------------------
    // Document synchronization (Copilot uses full-text sync)
    // ----------------------------------------------------------------------

    pub fn did_open(&self, uri: &Url, version: i32, language_id: &str, text: &str) {
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri.to_string(),
                    "languageId": language_id,
                    "version": version,
                    "text": text,
                }
            }),
        );
    }

    pub fn did_change(&self, uri: &Url, version: i32, text: &str) {
        self.notify(
            "textDocument/didChange",
            json!({
                "textDocument": { "uri": uri.to_string(), "version": version },
                "contentChanges": [{ "text": text }],
            }),
        );
    }

    pub fn did_close(&self, uri: &Url) {
        self.notify(
            "textDocument/didClose",
            json!({ "textDocument": { "uri": uri.to_string() } }),
        );
    }

    // ----------------------------------------------------------------------
    // Inline completion
    // ----------------------------------------------------------------------

    pub async fn inline_completion(
        &self,
        uri: &Url,
        version: i32,
        line: u32,
        character: u32,
        formatting: FormattingOptions,
    ) -> Result<Vec<InlineCompletionItem>> {
        let params = json!({
            "textDocument": { "uri": uri.to_string(), "version": version },
            "position": { "line": line, "character": character },
            "context": { "triggerKind": 2 },
            "formattingOptions": formatting,
        });
        let result: Option<InlineCompletionResult> = self
            .request("textDocument/inlineCompletion", params)
            .await?;
        Ok(result.map(|r| r.items).unwrap_or_default())
    }

    /// Telemetry: report that a completion item was shown to the user.
    pub fn notify_shown(&self, item_id: &str) {
        self.notify(
            "textDocument/didShowCompletion",
            json!({ "item": { "command": { "arguments": [item_id] } } }),
        );
    }

    /// Telemetry: report that a completion item was accepted by the user.
    pub fn notify_accepted(&self, item_id: &str) {
        self.notify(
            "workspace/executeCommand",
            json!({
                "command": "github.copilot.didAcceptCompletionItem",
                "arguments": [item_id],
            }),
        );
    }

    // ----------------------------------------------------------------------
    // Server -> client dispatch
    // ----------------------------------------------------------------------

    async fn run_dispatch(
        mut server_rx: UnboundedReceiver<(LanguageServerId, jsonrpc::Call)>,
        server_tx: UnboundedSender<Payload>,
        status: Arc<Mutex<Option<String>>>,
    ) {
        while let Some((_id, call)) = server_rx.recv().await {
            match call {
                jsonrpc::Call::MethodCall(method_call) => {
                    let result = Self::handle_server_request(&method_call);
                    let output = jsonrpc::Output::Success(jsonrpc::Success {
                        jsonrpc: Some(jsonrpc::Version::V2),
                        id: method_call.id,
                        result,
                    });
                    if server_tx.send(Payload::Response(output)).is_err() {
                        break;
                    }
                }
                jsonrpc::Call::Notification(notification) => {
                    Self::handle_server_notification(&notification, &status);
                }
                jsonrpc::Call::Invalid { .. } => {}
            }
        }
    }

    fn handle_server_request(method_call: &jsonrpc::MethodCall) -> Value {
        match method_call.method.as_str() {
            // Copilot asks for editor configuration; reply with one empty object
            // per requested item (an empty config is acceptable).
            "workspace/configuration" => {
                let len = match &method_call.params {
                    jsonrpc::Params::Map(map) => map
                        .get("items")
                        .and_then(Value::as_array)
                        .map(|items| items.len())
                        .unwrap_or(1),
                    _ => 1,
                };
                Value::Array(vec![json!({}); len.max(1)])
            }
            "window/workDoneProgress/create" => Value::Null,
            "window/showDocument" => json!({ "success": true }),
            _ => Value::Null,
        }
    }

    fn handle_server_notification(
        notification: &jsonrpc::Notification,
        status: &Arc<Mutex<Option<String>>>,
    ) {
        if let "didChangeStatus" | "statusNotification" = notification.method.as_str() {
            if let jsonrpc::Params::Map(map) = &notification.params {
                if let Some(kind) = map.get("kind").and_then(Value::as_str) {
                    *status.lock() = Some(kind.to_string());
                }
            }
        }
    }
}
