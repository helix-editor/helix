use crate::{
    file_operations::FileOperationsInterest,
    find_lsp_workspace, jsonrpc,
    transport::{Payload, Transport},
    Call, Error, LanguageServerId, OffsetEncoding, Result,
};

use crate::lsp::{
    self, notification::DidChangeWorkspaceFolders, CodeActionCapabilityResolveSupport,
    DidChangeWorkspaceFoldersParams, OneOf, PositionEncodingKind, SignatureHelp, Url,
    WorkspaceFolder, WorkspaceFoldersChangeEvent,
};
use helix_core::{find_workspace, syntax::config::LanguageServerFeature, ChangeSet, Rope};
use helix_loader::VERSION_AND_GIT_HASH;
use helix_stdx::path;
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf};
use std::{
    ffi::OsStr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use std::{future::Future, sync::OnceLock};
use std::{path::Path, process::Stdio};
use tokio::{
    io::{BufReader, BufWriter},
    process::{Child, Command},
    sync::{
        mpsc::{channel, UnboundedReceiver, UnboundedSender},
        Notify, OnceCell,
    },
};

fn workspace_for_uri(uri: lsp::Url) -> WorkspaceFolder {
    lsp::WorkspaceFolder {
        name: uri
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .map(|basename| basename.to_string())
            .unwrap_or_default(),
        uri,
    }
}

#[derive(Debug)]
pub struct Client {
    id: LanguageServerId,
    name: String,
    _process: Child,
    server_tx: UnboundedSender<Payload>,
    request_counter: AtomicU64,
    pub(crate) capabilities: OnceCell<lsp::ServerCapabilities>,
    pub(crate) file_operation_interest: OnceLock<FileOperationsInterest>,
    config: Option<Value>,
    root_path: std::path::PathBuf,
    root_uri: Option<lsp::Url>,
    workspace_folders: Mutex<Vec<lsp::WorkspaceFolder>>,
    initialize_notify: Arc<Notify>,
    /// workspace folders added while the server is still initializing
    req_timeout: u64,
}

impl Client {
    pub fn try_add_doc(
        self: &Arc<Self>,
        root_markers: &[String],
        manual_roots: &[PathBuf],
        doc_path: Option<&std::path::PathBuf>,
        may_support_workspace: bool,
    ) -> bool {
        let (workspace, workspace_is_cwd) = find_workspace();
        let workspace = path::normalize(workspace);
        let root = find_lsp_workspace(
            doc_path
                .and_then(|x| x.parent().and_then(|x| x.to_str()))
                .unwrap_or("."),
            root_markers,
            manual_roots,
            &workspace,
            workspace_is_cwd,
        );
        let root_uri = root
            .as_ref()
            .and_then(|root| lsp::Url::from_file_path(root).ok());

        if self.root_path == root.unwrap_or(workspace)
            || root_uri.as_ref().is_some_and(|root_uri| {
                self.workspace_folders
                    .lock()
                    .iter()
                    .any(|workspace| &workspace.uri == root_uri)
            })
        {
            // workspace URI is already registered so we can use this client
            return true;
        }

        // this server definitely doesn't support multiple workspace, no need to check capabilities
        if !may_support_workspace {
            return false;
        }

        let Some(capabilities) = self.capabilities.get() else {
            let client = Arc::clone(self);
            // initialization hasn't finished yet, deal with this new root later
            // TODO: In the edgecase that a **new root** is added
            // for an LSP that **doesn't support workspace_folders** before initaliation is finished
            // the new roots are ignored.
            // That particular edgecase would require retroactively spawning new LSP
            // clients and therefore also require us to retroactively update the corresponding
            // documents LSP client handle. It's doable but a pretty weird edgecase so let's
            // wait and see if anyone ever runs into it.
            tokio::spawn(async move {
                client.initialize_notify.notified().await;
                if let Some(workspace_folders_caps) = client
                    .capabilities()
                    .workspace
                    .as_ref()
                    .and_then(|cap| cap.workspace_folders.as_ref())
                    .filter(|cap| cap.supported.unwrap_or(false))
                {
                    client.add_workspace_folder(
                        root_uri,
                        workspace_folders_caps.change_notifications.as_ref(),
                    );
                }
            });
            return true;
        };

        if let Some(workspace_folders_caps) = capabilities
            .workspace
            .as_ref()
            .and_then(|cap| cap.workspace_folders.as_ref())
            .filter(|cap| cap.supported.unwrap_or(false))
        {
            self.add_workspace_folder(
                root_uri,
                workspace_folders_caps.change_notifications.as_ref(),
            );
            true
        } else {
            // the server doesn't support multi workspaces, we need a new client
            false
        }
    }

    fn add_workspace_folder(
        &self,
        root_uri: Option<lsp::Url>,
        change_notifications: Option<&OneOf<bool, String>>,
    ) {
        // root_uri is None just means that there isn't really any LSP workspace
        // associated with this file. For servers that support multiple workspaces
        // there is just one server so we can always just use that shared instance.
        // No need to add a new workspace root here as there is no logical root for this file
        // let the server deal with this
        let Some(root_uri) = root_uri else {
            return;
        };

        // server supports workspace folders, let's add the new root to the list
        self.workspace_folders
            .lock()
            .push(workspace_for_uri(root_uri.clone()));
        if Some(&OneOf::Left(false)) == change_notifications {
            // server specifically opted out of DidWorkspaceChange notifications
            // let's assume the server will request the workspace folders itself
            // and that we can therefore reuse the client (but are done now)
            return;
        }
        self.did_change_workspace(vec![workspace_for_uri(root_uri)], Vec::new())
    }

    /// Merge FormattingOptions with 'config.format' and return it
    fn get_merged_formatting_options(
        &self,
        options: lsp::FormattingOptions,
    ) -> lsp::FormattingOptions {
        let config_format = self
            .config
            .as_ref()
            .and_then(|cfg| cfg.get("format"))
            .and_then(|fmt| HashMap::<String, lsp::FormattingProperty>::deserialize(fmt).ok());

        if let Some(mut properties) = config_format {
            // passed in options take precedence over 'config.format'
            properties.extend(options.properties);
            lsp::FormattingOptions {
                properties,
                ..options
            }
        } else {
            options
        }
    }

    #[allow(clippy::type_complexity, clippy::too_many_arguments)]
    pub fn start(
        cmd: &str,
        args: &[String],
        config: Option<Value>,
        server_environment: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
        root_path: PathBuf,
        root_uri: Option<lsp::Url>,
        id: LanguageServerId,
        name: String,
        req_timeout: u64,
    ) -> Result<(
        Self,
        UnboundedReceiver<(LanguageServerId, Call)>,
        Arc<Notify>,
    )> {
        // Resolve path to the binary
        let cmd = helix_stdx::env::which(cmd)?;

        let process = Command::new(cmd)
            .envs(server_environment)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&root_path)
            // make sure the process is reaped on drop
            .kill_on_drop(true)
            .spawn();

        let mut process = process?;

        // TODO: do we need bufreader/writer here? or do we use async wrappers on unblock?
        let writer = BufWriter::new(process.stdin.take().expect("Failed to open stdin"));
        let reader = BufReader::new(process.stdout.take().expect("Failed to open stdout"));
        let stderr = BufReader::new(process.stderr.take().expect("Failed to open stderr"));

        let (server_rx, server_tx, initialize_notify) =
            Transport::start(reader, writer, stderr, id, name.clone());

        let workspace_folders = root_uri
            .clone()
            .map(|root| vec![workspace_for_uri(root)])
            .unwrap_or_default();

        let client = Self {
            id,
            name,
            _process: process,
            server_tx,
            request_counter: AtomicU64::new(0),
            capabilities: OnceCell::new(),
            file_operation_interest: OnceLock::new(),
            config,
            req_timeout,
            root_path,
            root_uri,
            workspace_folders: Mutex::new(workspace_folders),
            initialize_notify: initialize_notify.clone(),
        };

        Ok((client, server_rx, initialize_notify))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> LanguageServerId {
        self.id
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

    pub fn is_initialized(&self) -> bool {
        self.capabilities.get().is_some()
    }

    pub fn capabilities(&self) -> &lsp::ServerCapabilities {
        self.capabilities
            .get()
            .expect("language server not yet initialized!")
    }

    pub(crate) fn file_operations_intests(&self) -> &FileOperationsInterest {
        self.file_operation_interest
            .get_or_init(|| FileOperationsInterest::new(self.capabilities()))
    }

    /// Client has to be initialized otherwise this function panics
    #[inline]
    pub fn supports_feature(&self, feature: LanguageServerFeature) -> bool {
        let capabilities = self.capabilities();

        use lsp::*;
        match feature {
            LanguageServerFeature::Format => matches!(
                capabilities.document_formatting_provider,
                Some(OneOf::Left(true) | OneOf::Right(_))
            ),
            LanguageServerFeature::GotoDeclaration => matches!(
                capabilities.declaration_provider,
                Some(
                    DeclarationCapability::Simple(true)
                        | DeclarationCapability::RegistrationOptions(_)
                        | DeclarationCapability::Options(_),
                )
            ),
            LanguageServerFeature::GotoDefinition => matches!(
                capabilities.definition_provider,
                Some(OneOf::Left(true) | OneOf::Right(_))
            ),
            LanguageServerFeature::GotoTypeDefinition => matches!(
                capabilities.type_definition_provider,
                Some(
                    TypeDefinitionProviderCapability::Simple(true)
                        | TypeDefinitionProviderCapability::Options(_),
                )
            ),
            LanguageServerFeature::GotoReference => matches!(
                capabilities.references_provider,
                Some(OneOf::Left(true) | OneOf::Right(_))
            ),
            LanguageServerFeature::GotoImplementation => matches!(
                capabilities.implementation_provider,
                Some(
                    ImplementationProviderCapability::Simple(true)
                        | ImplementationProviderCapability::Options(_),
                )
            ),
            LanguageServerFeature::SignatureHelp => capabilities.signature_help_provider.is_some(),
            LanguageServerFeature::Hover => matches!(
                capabilities.hover_provider,
                Some(HoverProviderCapability::Simple(true) | HoverProviderCapability::Options(_),)
            ),
            LanguageServerFeature::DocumentHighlight => matches!(
                capabilities.document_highlight_provider,
                Some(OneOf::Left(true) | OneOf::Right(_))
            ),
            LanguageServerFeature::Completion => capabilities.completion_provider.is_some(),
            LanguageServerFeature::CodeAction => matches!(
                capabilities.code_action_provider,
                Some(
                    CodeActionProviderCapability::Simple(true)
                        | CodeActionProviderCapability::Options(_),
                )
            ),
            LanguageServerFeature::WorkspaceCommand => {
                capabilities.execute_command_provider.is_some()
            }
            LanguageServerFeature::DocumentSymbols => matches!(
                capabilities.document_symbol_provider,
                Some(OneOf::Left(true) | OneOf::Right(_))
            ),
            LanguageServerFeature::WorkspaceSymbols => matches!(
                capabilities.workspace_symbol_provider,
                Some(OneOf::Left(true) | OneOf::Right(_))
            ),
            LanguageServerFeature::Diagnostics => true, // there's no extra server capability
            LanguageServerFeature::PullDiagnostics => capabilities.diagnostic_provider.is_some(),
            LanguageServerFeature::RenameSymbol => matches!(
                capabilities.rename_provider,
                Some(OneOf::Left(true)) | Some(OneOf::Right(_))
            ),
            LanguageServerFeature::InlayHints => matches!(
                capabilities.inlay_hint_provider,
                Some(OneOf::Left(true) | OneOf::Right(InlayHintServerCapabilities::Options(_)))
            ),
            LanguageServerFeature::DocumentColors => matches!(
                capabilities.color_provider,
                Some(
                    ColorProviderCapability::Simple(true)
                        | ColorProviderCapability::ColorProvider(_)
                        | ColorProviderCapability::Options(_)
                )
            ),
        }
    }

    pub fn offset_encoding(&self) -> OffsetEncoding {
        self.capabilities()
            .position_encoding
            .as_ref()
            .and_then(|encoding| match encoding.as_str() {
                "utf-8" => Some(OffsetEncoding::Utf8),
                "utf-16" => Some(OffsetEncoding::Utf16),
                "utf-32" => Some(OffsetEncoding::Utf32),
                encoding => {
                    log::error!("Server provided invalid position encoding {encoding}, defaulting to utf-16");
                    None
                },
            })
            .unwrap_or_default()
    }

    pub fn config(&self) -> Option<&Value> {
        self.config.as_ref()
    }

    pub async fn workspace_folders(
        &self,
    ) -> parking_lot::MutexGuard<'_, Vec<lsp::WorkspaceFolder>> {
        self.workspace_folders.lock()
    }

    /// Execute a RPC request on the language server.
    fn call<R: lsp::request::Request>(
        &self,
        params: R::Params,
    ) -> impl Future<Output = Result<R::Result>>
    where
        R::Params: serde::Serialize,
    {
        self.call_with_ref::<R>(&params)
    }

    fn call_with_ref<R: lsp::request::Request>(
        &self,
        params: &R::Params,
    ) -> impl Future<Output = Result<R::Result>>
    where
        R::Params: serde::Serialize,
    {
        self.call_with_timeout::<R>(params, self.req_timeout)
    }

    fn call_with_timeout<R: lsp::request::Request>(
        &self,
        params: &R::Params,
        timeout_secs: u64,
    ) -> impl Future<Output = Result<R::Result>>
    where
        R::Params: serde::Serialize,
    {
        let server_tx = self.server_tx.clone();
        let id = self.next_request_id();

        // It's important that this is not part of the future so that it gets executed right away
        // and the request order stays consistent.
        let rx = serde_json::to_value(params)
            .map_err(Error::from)
            .and_then(|params| {
                let request = jsonrpc::MethodCall {
                    jsonrpc: Some(jsonrpc::Version::V2),
                    id: id.clone(),
                    method: R::METHOD.to_string(),
                    params: Self::value_into_params(params),
                };
                let (tx, rx) = channel::<Result<Value>>(1);
                server_tx
                    .send(Payload::Request {
                        chan: tx,
                        value: request,
                    })
                    .map_err(|e| Error::Other(e.into()))?;
                Ok(rx)
            });

        async move {
            use std::time::Duration;
            use tokio::time::timeout;
            // TODO: delay other calls until initialize success
            timeout(Duration::from_secs(timeout_secs), rx?.recv())
                .await
                .map_err(|_| Error::Timeout(id))? // return Timeout
                .ok_or(Error::StreamClosed)?
                .and_then(|value| serde_json::from_value(value).map_err(Into::into))
        }
    }

    /// Send a RPC notification to the language server.
    pub fn notify<R: lsp::notification::Notification>(&self, params: R::Params)
    where
        R::Params: serde::Serialize,
    {
        let server_tx = self.server_tx.clone();

        let params = match serde_json::to_value(params) {
            Ok(params) => params,
            Err(err) => {
                log::error!(
                    "Failed to serialize params for notification '{}' for server '{}': {err}",
                    R::METHOD,
                    self.name,
                );
                return;
            }
        };

        let notification = jsonrpc::Notification {
            jsonrpc: Some(jsonrpc::Version::V2),
            method: R::METHOD.to_string(),
            params: Self::value_into_params(params),
        };

        if let Err(err) = server_tx.send(Payload::Notification(notification)) {
            log::error!(
                "Failed to send notification '{}' to server '{}': {err}",
                R::METHOD,
                self.name
            );
        }
    }

    /// Reply to a language server RPC call.
    pub fn reply(
        &self,
        id: jsonrpc::Id,
        result: core::result::Result<Value, jsonrpc::Error>,
    ) -> Result<()> {
        use jsonrpc::{Failure, Output, Success, Version};

        let server_tx = self.server_tx.clone();

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

        server_tx
            .send(Payload::Response(output))
            .map_err(|e| Error::Other(e.into()))?;

        Ok(())
    }

    // -------------------------------------------------------------------------------------------
    // General messages
    // -------------------------------------------------------------------------------------------

    pub(crate) async fn initialize(&self, enable_snippets: bool) -> Result<lsp::InitializeResult> {
        if let Some(config) = &self.config {
            log::info!("Using custom LSP config: {}", config);
        }

        #[allow(deprecated)]
        let params = lsp::InitializeParams {
            process_id: Some(std::process::id()),
            workspace_folders: Some(self.workspace_folders.lock().clone()),
            // root_path is obsolete, but some clients like pyright still use it so we specify both.
            // clients will prefer _uri if possible
            root_path: self.root_path.to_str().map(|path| path.to_owned()),
            root_uri: self.root_uri.clone(),
            initialization_options: self.config.clone(),
            capabilities: lsp::ClientCapabilities {
                workspace: Some(lsp::WorkspaceClientCapabilities {
                    configuration: Some(true),
                    did_change_configuration: Some(lsp::DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    workspace_folders: Some(true),
                    apply_edit: Some(true),
                    symbol: Some(lsp::WorkspaceSymbolClientCapabilities {
                        dynamic_registration: Some(false),
                        ..Default::default()
                    }),
                    execute_command: Some(lsp::DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    inlay_hint: Some(lsp::InlayHintWorkspaceClientCapabilities {
                        refresh_support: Some(false),
                    }),
                    workspace_edit: Some(lsp::WorkspaceEditClientCapabilities {
                        document_changes: Some(true),
                        resource_operations: Some(vec![
                            lsp::ResourceOperationKind::Create,
                            lsp::ResourceOperationKind::Rename,
                            lsp::ResourceOperationKind::Delete,
                        ]),
                        failure_handling: Some(lsp::FailureHandlingKind::Abort),
                        normalizes_line_endings: Some(false),
                        change_annotation_support: None,
                    }),
                    did_change_watched_files: Some(lsp::DidChangeWatchedFilesClientCapabilities {
                        dynamic_registration: Some(true),
                        relative_pattern_support: Some(true),
                    }),
                    file_operations: Some(lsp::WorkspaceFileOperationsClientCapabilities {
                        will_rename: Some(true),
                        did_rename: Some(true),
                        ..Default::default()
                    }),
                    diagnostic: Some(lsp::DiagnosticWorkspaceClientCapabilities {
                        refresh_support: Some(true),
                    }),
                    ..Default::default()
                }),
                text_document: Some(lsp::TextDocumentClientCapabilities {
                    completion: Some(lsp::CompletionClientCapabilities {
                        completion_item: Some(lsp::CompletionItemCapability {
                            snippet_support: Some(enable_snippets),
                            resolve_support: Some(lsp::CompletionItemCapabilityResolveSupport {
                                properties: vec![
                                    String::from("documentation"),
                                    String::from("detail"),
                                    String::from("additionalTextEdits"),
                                ],
                            }),
                            insert_replace_support: Some(true),
                            deprecated_support: Some(true),
                            tag_support: Some(lsp::TagSupport {
                                value_set: vec![lsp::CompletionItemTag::DEPRECATED],
                            }),
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
                    signature_help: Some(lsp::SignatureHelpClientCapabilities {
                        signature_information: Some(lsp::SignatureInformationSettings {
                            documentation_format: Some(vec![lsp::MarkupKind::Markdown]),
                            parameter_information: Some(lsp::ParameterInformationSettings {
                                label_offset_support: Some(true),
                            }),
                            active_parameter_support: Some(true),
                        }),
                        ..Default::default()
                    }),
                    rename: Some(lsp::RenameClientCapabilities {
                        dynamic_registration: Some(false),
                        prepare_support: Some(true),
                        prepare_support_default_behavior: None,
                        honors_change_annotations: Some(false),
                    }),
                    formatting: Some(lsp::DocumentFormattingClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    code_action: Some(lsp::CodeActionClientCapabilities {
                        code_action_literal_support: Some(lsp::CodeActionLiteralSupport {
                            code_action_kind: lsp::CodeActionKindLiteralSupport {
                                value_set: [
                                    lsp::CodeActionKind::EMPTY,
                                    lsp::CodeActionKind::QUICKFIX,
                                    lsp::CodeActionKind::REFACTOR,
                                    lsp::CodeActionKind::REFACTOR_EXTRACT,
                                    lsp::CodeActionKind::REFACTOR_INLINE,
                                    lsp::CodeActionKind::REFACTOR_REWRITE,
                                    lsp::CodeActionKind::SOURCE,
                                    lsp::CodeActionKind::SOURCE_ORGANIZE_IMPORTS,
                                    lsp::CodeActionKind::SOURCE_FIX_ALL,
                                ]
                                .iter()
                                .map(|kind| kind.as_str().to_string())
                                .collect(),
                            },
                        }),
                        is_preferred_support: Some(true),
                        disabled_support: Some(true),
                        data_support: Some(true),
                        resolve_support: Some(CodeActionCapabilityResolveSupport {
                            properties: vec!["edit".to_owned(), "command".to_owned()],
                        }),
                        ..Default::default()
                    }),
                    diagnostic: Some(lsp::DiagnosticClientCapabilities {
                        dynamic_registration: Some(false),
                        related_document_support: Some(true),
                    }),
                    publish_diagnostics: Some(lsp::PublishDiagnosticsClientCapabilities {
                        version_support: Some(true),
                        tag_support: Some(lsp::TagSupport {
                            value_set: vec![
                                lsp::DiagnosticTag::UNNECESSARY,
                                lsp::DiagnosticTag::DEPRECATED,
                            ],
                        }),
                        ..Default::default()
                    }),
                    inlay_hint: Some(lsp::InlayHintClientCapabilities {
                        dynamic_registration: Some(false),
                        resolve_support: None,
                    }),
                    ..Default::default()
                }),
                window: Some(lsp::WindowClientCapabilities {
                    work_done_progress: Some(true),
                    show_document: Some(lsp::ShowDocumentClientCapabilities { support: true }),
                    ..Default::default()
                }),
                general: Some(lsp::GeneralClientCapabilities {
                    position_encodings: Some(vec![
                        PositionEncodingKind::UTF8,
                        PositionEncodingKind::UTF32,
                        PositionEncodingKind::UTF16,
                    ]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            trace: None,
            client_info: Some(lsp::ClientInfo {
                name: String::from("helix"),
                version: Some(String::from(VERSION_AND_GIT_HASH)),
            }),
            locale: None, // TODO
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        };

        self.call::<lsp::request::Initialize>(params).await
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.call::<lsp::request::Shutdown>(()).await
    }

    pub fn exit(&self) {
        self.notify::<lsp::notification::Exit>(())
    }

    /// Tries to shut down the language server but returns
    /// early if server responds with an error.
    pub async fn shutdown_and_exit(&self) -> Result<()> {
        self.shutdown().await?;
        self.exit();
        Ok(())
    }

    /// Forcefully shuts down the language server ignoring any errors.
    pub async fn force_shutdown(&self) -> Result<()> {
        if let Err(e) = self.shutdown().await {
            log::warn!("language server failed to terminate gracefully - {}", e);
        }
        self.exit();
        Ok(())
    }

    // -------------------------------------------------------------------------------------------
    // Workspace
    // -------------------------------------------------------------------------------------------

    pub fn did_change_configuration(&self, settings: Value) {
        self.notify::<lsp::notification::DidChangeConfiguration>(
            lsp::DidChangeConfigurationParams { settings },
        )
    }

    pub fn did_change_workspace(&self, added: Vec<WorkspaceFolder>, removed: Vec<WorkspaceFolder>) {
        self.notify::<DidChangeWorkspaceFolders>(DidChangeWorkspaceFoldersParams {
            event: WorkspaceFoldersChangeEvent { added, removed },
        })
    }

    pub fn will_rename(
        &self,
        old_path: &Path,
        new_path: &Path,
        is_dir: bool,
    ) -> Option<impl Future<Output = Result<Option<lsp::WorkspaceEdit>>>> {
        let capabilities = self.file_operations_intests();
        if !capabilities.will_rename.has_interest(old_path, is_dir) {
            return None;
        }
        let url_from_path = |path| {
            let url = if is_dir {
                Url::from_directory_path(path)
            } else {
                Url::from_file_path(path)
            };
            Some(url.ok()?.to_string())
        };
        let files = vec![lsp::FileRename {
            old_uri: url_from_path(old_path)?,
            new_uri: url_from_path(new_path)?,
        }];
        Some(self.call_with_timeout::<lsp::request::WillRenameFiles>(
            &lsp::RenameFilesParams { files },
            5,
        ))
    }

    pub fn did_rename(&self, old_path: &Path, new_path: &Path, is_dir: bool) -> Option<()> {
        let capabilities = self.file_operations_intests();
        if !capabilities.did_rename.has_interest(new_path, is_dir) {
            return None;
        }
        let url_from_path = |path| {
            let url = if is_dir {
                Url::from_directory_path(path)
            } else {
                Url::from_file_path(path)
            };
            Some(url.ok()?.to_string())
        };

        let files = vec![lsp::FileRename {
            old_uri: url_from_path(old_path)?,
            new_uri: url_from_path(new_path)?,
        }];
        self.notify::<lsp::notification::DidRenameFiles>(lsp::RenameFilesParams { files });
        Some(())
    }

    // -------------------------------------------------------------------------------------------
    // Text document
    // -------------------------------------------------------------------------------------------

    pub fn text_document_did_open(
        &self,
        uri: lsp::Url,
        version: i32,
        doc: &Rope,
        language_id: String,
    ) {
        self.notify::<lsp::notification::DidOpenTextDocument>(lsp::DidOpenTextDocumentParams {
            text_document: lsp::TextDocumentItem {
                uri,
                language_id,
                version,
                text: String::from(doc),
            },
        })
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

        use helix_core::RopeSlice;
        fn traverse(
            pos: lsp::Position,
            text: RopeSlice,
            offset_encoding: OffsetEncoding,
        ) -> lsp::Position {
            let lsp::Position {
                mut line,
                mut character,
            } = pos;

            let mut chars = text.chars().peekable();
            while let Some(ch) = chars.next() {
                // LSP only considers \n, \r or \r\n as line endings
                if ch == '\n' || ch == '\r' {
                    // consume a \r\n
                    if ch == '\r' && chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    line += 1;
                    character = 0;
                } else {
                    character += match offset_encoding {
                        OffsetEncoding::Utf8 => ch.len_utf8() as u32,
                        OffsetEncoding::Utf16 => ch.len_utf16() as u32,
                        OffsetEncoding::Utf32 => 1,
                    };
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
                    let end = traverse(start, old_text.slice(old_pos..old_end), offset_encoding);

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
                        let end =
                            traverse(start, old_text.slice(old_pos..old_end), offset_encoding);

                        iter.next();

                        // replacement
                        end
                    } else {
                        // insert
                        start
                    };

                    changes.push(lsp::TextDocumentContentChangeEvent {
                        range: Some(lsp::Range::new(start, end)),
                        text: s.to_string(),
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
    ) -> Option<()> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support document sync.
        let sync_capabilities = match capabilities.text_document_sync {
            Some(
                lsp::TextDocumentSyncCapability::Kind(kind)
                | lsp::TextDocumentSyncCapability::Options(lsp::TextDocumentSyncOptions {
                    change: Some(kind),
                    ..
                }),
            ) => kind,
            // None | SyncOptions { changes: None }
            _ => return None,
        };

        let changes = match sync_capabilities {
            lsp::TextDocumentSyncKind::FULL => {
                vec![lsp::TextDocumentContentChangeEvent {
                    // range = None -> whole document
                    range: None,        //Some(Range)
                    range_length: None, // u64 apparently deprecated
                    text: new_text.to_string(),
                }]
            }
            lsp::TextDocumentSyncKind::INCREMENTAL => {
                Self::changeset_to_changes(old_text, new_text, changes, self.offset_encoding())
            }
            lsp::TextDocumentSyncKind::NONE => return None,
            kind => unimplemented!("{:?}", kind),
        };

        self.notify::<lsp::notification::DidChangeTextDocument>(lsp::DidChangeTextDocumentParams {
            text_document,
            content_changes: changes,
        });
        Some(())
    }

    pub fn text_document_did_close(&self, text_document: lsp::TextDocumentIdentifier) {
        self.notify::<lsp::notification::DidCloseTextDocument>(lsp::DidCloseTextDocumentParams {
            text_document,
        })
    }

    // will_save / will_save_wait_until

    pub fn text_document_did_save(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        text: &Rope,
    ) -> Option<()> {
        let capabilities = self.capabilities.get().unwrap();

        let include_text = match &capabilities.text_document_sync.as_ref()? {
            lsp::TextDocumentSyncCapability::Options(lsp::TextDocumentSyncOptions {
                save: options,
                ..
            }) => match options.as_ref()? {
                lsp::TextDocumentSyncSaveOptions::Supported(true) => false,
                lsp::TextDocumentSyncSaveOptions::SaveOptions(lsp::SaveOptions {
                    include_text,
                }) => include_text.unwrap_or(false),
                lsp::TextDocumentSyncSaveOptions::Supported(false) => return None,
            },
            // see: https://github.com/microsoft/language-server-protocol/issues/288
            lsp::TextDocumentSyncCapability::Kind(..) => false,
        };

        self.notify::<lsp::notification::DidSaveTextDocument>(lsp::DidSaveTextDocumentParams {
            text_document,
            text: include_text.then_some(text.into()),
        });
        Some(())
    }

    pub fn completion(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
        work_done_token: Option<lsp::ProgressToken>,
        context: lsp::CompletionContext,
    ) -> Option<impl Future<Output = Result<Option<lsp::CompletionResponse>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support completion.
        capabilities.completion_provider.as_ref()?;

        let params = lsp::CompletionParams {
            text_document_position: lsp::TextDocumentPositionParams {
                text_document,
                position,
            },
            context: Some(context),
            // TODO: support these tokens by async receiving and updating the choice list
            work_done_progress_params: lsp::WorkDoneProgressParams { work_done_token },
            partial_result_params: lsp::PartialResultParams {
                partial_result_token: None,
            },
        };

        Some(self.call::<lsp::request::Completion>(params))
    }

    pub fn resolve_completion_item(
        &self,
        completion_item: &lsp::CompletionItem,
    ) -> impl Future<Output = Result<lsp::CompletionItem>> {
        self.call_with_ref::<lsp::request::ResolveCompletionItem>(completion_item)
    }

    pub fn resolve_code_action(
        &self,
        code_action: &lsp::CodeAction,
    ) -> Option<impl Future<Output = Result<lsp::CodeAction>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support resolving code actions.
        match capabilities.code_action_provider {
            Some(lsp::CodeActionProviderCapability::Options(lsp::CodeActionOptions {
                resolve_provider: Some(true),
                ..
            })) => (),
            _ => return None,
        }

        Some(self.call_with_ref::<lsp::request::CodeActionResolveRequest>(code_action))
    }

    pub fn text_document_signature_help(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> Option<impl Future<Output = Result<Option<SignatureHelp>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support signature help.
        capabilities.signature_help_provider.as_ref()?;

        let params = lsp::SignatureHelpParams {
            text_document_position_params: lsp::TextDocumentPositionParams {
                text_document,
                position,
            },
            work_done_progress_params: lsp::WorkDoneProgressParams { work_done_token },
            context: None,
            // lsp::SignatureHelpContext
        };

        Some(self.call::<lsp::request::SignatureHelpRequest>(params))
    }

    pub fn text_document_range_inlay_hints(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        range: lsp::Range,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> Option<impl Future<Output = Result<Option<Vec<lsp::InlayHint>>>>> {
        let capabilities = self.capabilities.get().unwrap();

        match capabilities.inlay_hint_provider {
            Some(
                lsp::OneOf::Left(true)
                | lsp::OneOf::Right(lsp::InlayHintServerCapabilities::Options(_)),
            ) => (),
            _ => return None,
        }

        let params = lsp::InlayHintParams {
            text_document,
            range,
            work_done_progress_params: lsp::WorkDoneProgressParams { work_done_token },
        };

        Some(self.call::<lsp::request::InlayHintRequest>(params))
    }

    pub fn text_document_document_color(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> Option<impl Future<Output = Result<Vec<lsp::ColorInformation>>>> {
        self.capabilities.get().unwrap().color_provider.as_ref()?;
        let params = lsp::DocumentColorParams {
            text_document,
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: work_done_token.clone(),
            },
            partial_result_params: helix_lsp_types::PartialResultParams {
                partial_result_token: work_done_token,
            },
        };

        Some(self.call::<lsp::request::DocumentColor>(params))
    }

    pub fn text_document_hover(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> Option<impl Future<Output = Result<Option<lsp::Hover>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support hover.
        match capabilities.hover_provider {
            Some(
                lsp::HoverProviderCapability::Simple(true)
                | lsp::HoverProviderCapability::Options(_),
            ) => (),
            _ => return None,
        }

        let params = lsp::HoverParams {
            text_document_position_params: lsp::TextDocumentPositionParams {
                text_document,
                position,
            },
            work_done_progress_params: lsp::WorkDoneProgressParams { work_done_token },
            // lsp::SignatureHelpContext
        };

        Some(self.call::<lsp::request::HoverRequest>(params))
    }

    // formatting

    pub fn text_document_formatting(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        options: lsp::FormattingOptions,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> Option<impl Future<Output = Result<Option<Vec<lsp::TextEdit>>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support formatting.
        match capabilities.document_formatting_provider {
            Some(lsp::OneOf::Left(true) | lsp::OneOf::Right(_)) => (),
            _ => return None,
        };

        let options = self.get_merged_formatting_options(options);

        let params = lsp::DocumentFormattingParams {
            text_document,
            options,
            work_done_progress_params: lsp::WorkDoneProgressParams { work_done_token },
        };

        Some(self.call::<lsp::request::Formatting>(params))
    }

    pub fn text_document_range_formatting(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        range: lsp::Range,
        options: lsp::FormattingOptions,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> Option<impl Future<Output = Result<Option<Vec<lsp::TextEdit>>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support range formatting.
        match capabilities.document_range_formatting_provider {
            Some(lsp::OneOf::Left(true) | lsp::OneOf::Right(_)) => (),
            _ => return None,
        };

        let options = self.get_merged_formatting_options(options);

        let params = lsp::DocumentRangeFormattingParams {
            text_document,
            range,
            options,
            work_done_progress_params: lsp::WorkDoneProgressParams { work_done_token },
        };

        Some(self.call::<lsp::request::RangeFormatting>(params))
    }

    pub fn text_document_diagnostic(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        previous_result_id: Option<String>,
    ) -> Option<impl Future<Output = Result<lsp::DocumentDiagnosticReportResult>>> {
        let capabilities = self.capabilities();

        // Return early if the server does not support pull diagnostic.
        let identifier = match capabilities.diagnostic_provider.as_ref()? {
            lsp::DiagnosticServerCapabilities::Options(cap) => cap.identifier.clone(),
            lsp::DiagnosticServerCapabilities::RegistrationOptions(cap) => {
                cap.diagnostic_options.identifier.clone()
            }
        };

        let params = lsp::DocumentDiagnosticParams {
            text_document,
            identifier,
            previous_result_id,
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        };

        Some(self.call::<lsp::request::DocumentDiagnosticRequest>(params))
    }

    pub fn text_document_document_highlight(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> Option<impl Future<Output = Result<Option<Vec<lsp::DocumentHighlight>>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support document highlight.
        match capabilities.document_highlight_provider {
            Some(lsp::OneOf::Left(true) | lsp::OneOf::Right(_)) => (),
            _ => return None,
        }

        let params = lsp::DocumentHighlightParams {
            text_document_position_params: lsp::TextDocumentPositionParams {
                text_document,
                position,
            },
            work_done_progress_params: lsp::WorkDoneProgressParams { work_done_token },
            partial_result_params: lsp::PartialResultParams {
                partial_result_token: None,
            },
        };

        Some(self.call::<lsp::request::DocumentHighlightRequest>(params))
    }

    fn goto_request<
        T: lsp::request::Request<
            Params = lsp::GotoDefinitionParams,
            Result = Option<lsp::GotoDefinitionResponse>,
        >,
    >(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> impl Future<Output = Result<T::Result>> {
        let params = lsp::GotoDefinitionParams {
            text_document_position_params: lsp::TextDocumentPositionParams {
                text_document,
                position,
            },
            work_done_progress_params: lsp::WorkDoneProgressParams { work_done_token },
            partial_result_params: lsp::PartialResultParams {
                partial_result_token: None,
            },
        };

        self.call::<T>(params)
    }

    pub fn goto_definition(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> Option<impl Future<Output = Result<Option<lsp::GotoDefinitionResponse>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support goto-definition.
        match capabilities.definition_provider {
            Some(lsp::OneOf::Left(true) | lsp::OneOf::Right(_)) => (),
            _ => return None,
        }

        Some(self.goto_request::<lsp::request::GotoDefinition>(
            text_document,
            position,
            work_done_token,
        ))
    }

    pub fn goto_declaration(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> Option<impl Future<Output = Result<Option<lsp::GotoDefinitionResponse>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support goto-declaration.
        match capabilities.declaration_provider {
            Some(
                lsp::DeclarationCapability::Simple(true)
                | lsp::DeclarationCapability::RegistrationOptions(_)
                | lsp::DeclarationCapability::Options(_),
            ) => (),
            _ => return None,
        }

        Some(self.goto_request::<lsp::request::GotoDeclaration>(
            text_document,
            position,
            work_done_token,
        ))
    }

    pub fn goto_type_definition(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> Option<impl Future<Output = Result<Option<lsp::GotoDefinitionResponse>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support goto-type-definition.
        match capabilities.type_definition_provider {
            Some(
                lsp::TypeDefinitionProviderCapability::Simple(true)
                | lsp::TypeDefinitionProviderCapability::Options(_),
            ) => (),
            _ => return None,
        }

        Some(self.goto_request::<lsp::request::GotoTypeDefinition>(
            text_document,
            position,
            work_done_token,
        ))
    }

    pub fn goto_implementation(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> Option<impl Future<Output = Result<Option<lsp::GotoDefinitionResponse>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support goto-definition.
        match capabilities.implementation_provider {
            Some(
                lsp::ImplementationProviderCapability::Simple(true)
                | lsp::ImplementationProviderCapability::Options(_),
            ) => (),
            _ => return None,
        }

        Some(self.goto_request::<lsp::request::GotoImplementation>(
            text_document,
            position,
            work_done_token,
        ))
    }

    pub fn goto_reference(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
        include_declaration: bool,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> Option<impl Future<Output = Result<Option<Vec<lsp::Location>>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support goto-reference.
        match capabilities.references_provider {
            Some(lsp::OneOf::Left(true) | lsp::OneOf::Right(_)) => (),
            _ => return None,
        }

        let params = lsp::ReferenceParams {
            text_document_position: lsp::TextDocumentPositionParams {
                text_document,
                position,
            },
            context: lsp::ReferenceContext {
                include_declaration,
            },
            work_done_progress_params: lsp::WorkDoneProgressParams { work_done_token },
            partial_result_params: lsp::PartialResultParams {
                partial_result_token: None,
            },
        };

        Some(self.call::<lsp::request::References>(params))
    }

    pub fn document_symbols(
        &self,
        text_document: lsp::TextDocumentIdentifier,
    ) -> Option<impl Future<Output = Result<Option<lsp::DocumentSymbolResponse>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support document symbols.
        match capabilities.document_symbol_provider {
            Some(lsp::OneOf::Left(true) | lsp::OneOf::Right(_)) => (),
            _ => return None,
        }

        let params = lsp::DocumentSymbolParams {
            text_document,
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        };

        Some(self.call::<lsp::request::DocumentSymbolRequest>(params))
    }

    pub fn prepare_rename(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
    ) -> Option<impl Future<Output = Result<Option<lsp::PrepareRenameResponse>>>> {
        let capabilities = self.capabilities.get().unwrap();

        match capabilities.rename_provider {
            Some(lsp::OneOf::Right(lsp::RenameOptions {
                prepare_provider: Some(true),
                ..
            })) => (),
            _ => return None,
        }

        let params = lsp::TextDocumentPositionParams {
            text_document,
            position,
        };

        Some(self.call::<lsp::request::PrepareRenameRequest>(params))
    }

    // empty string to get all symbols
    pub fn workspace_symbols(
        &self,
        query: String,
    ) -> Option<impl Future<Output = Result<Option<lsp::WorkspaceSymbolResponse>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support workspace symbols.
        match capabilities.workspace_symbol_provider {
            Some(lsp::OneOf::Left(true) | lsp::OneOf::Right(_)) => (),
            _ => return None,
        }

        let params = lsp::WorkspaceSymbolParams {
            query,
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        };

        Some(self.call::<lsp::request::WorkspaceSymbolRequest>(params))
    }

    pub fn code_actions(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        range: lsp::Range,
        context: lsp::CodeActionContext,
    ) -> Option<impl Future<Output = Result<Option<Vec<lsp::CodeActionOrCommand>>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the server does not support code actions.
        match capabilities.code_action_provider {
            Some(
                lsp::CodeActionProviderCapability::Simple(true)
                | lsp::CodeActionProviderCapability::Options(_),
            ) => (),
            _ => return None,
        }

        let params = lsp::CodeActionParams {
            text_document,
            range,
            context,
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        };

        Some(self.call::<lsp::request::CodeActionRequest>(params))
    }

    pub fn rename_symbol(
        &self,
        text_document: lsp::TextDocumentIdentifier,
        position: lsp::Position,
        new_name: String,
    ) -> Option<impl Future<Output = Result<Option<lsp::WorkspaceEdit>>>> {
        if !self.supports_feature(LanguageServerFeature::RenameSymbol) {
            return None;
        }

        let params = lsp::RenameParams {
            text_document_position: lsp::TextDocumentPositionParams {
                text_document,
                position,
            },
            new_name,
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: None,
            },
        };

        Some(self.call::<lsp::request::Rename>(params))
    }

    pub fn command(
        &self,
        command: lsp::Command,
    ) -> Option<impl Future<Output = Result<Option<Value>>>> {
        let capabilities = self.capabilities.get().unwrap();

        // Return early if the language server does not support executing commands.
        capabilities.execute_command_provider.as_ref()?;

        let params = lsp::ExecuteCommandParams {
            command: command.command,
            arguments: command.arguments.unwrap_or_default(),
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: None,
            },
        };

        Some(self.call::<lsp::request::ExecuteCommand>(params))
    }

    pub fn did_change_watched_files(&self, changes: Vec<lsp::FileEvent>) {
        self.notify::<lsp::notification::DidChangeWatchedFiles>(lsp::DidChangeWatchedFilesParams {
            changes,
        })
    }
}
