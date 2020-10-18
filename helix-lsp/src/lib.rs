use std::collections::HashMap;

use jsonrpc_core as jsonrpc;
use lsp_types as lsp;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use smol::channel::{Receiver, Sender};
use smol::io::{BufReader, BufWriter};
use smol::prelude::*;
use smol::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use smol::Executor;

use futures_util::{select, FutureExt};

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

pub struct Client {
    process: Child,
    stderr: BufReader<ChildStderr>,
    outgoing: Sender<Payload>,

    pub request_counter: u64,

    capabilities: Option<lsp::ServerCapabilities>,
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

            request_counter: 0,

            capabilities: None,
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

    pub async fn text_document_did_open(&mut self) -> anyhow::Result<()> {
        self.notify::<lsp::notification::DidOpenTextDocument>(lsp::DidOpenTextDocumentParams {
            text_document: lsp::TextDocumentItem {
                uri: lsp::Url::parse(".")?,
                language_id: "rust".to_string(),
                version: 0,
                text: "".to_string(),
            },
        })
        .await
    }

    pub async fn text_document_did_change(&mut self) -> anyhow::Result<()> {
        unimplemented!()
    }

    // will_save / will_save_wait_until

    pub async fn text_document_did_save(&mut self) -> anyhow::Result<()> {
        unimplemented!()
    }

    pub async fn text_document_did_close(&mut self) -> anyhow::Result<()> {
        unimplemented!()
    }
}

enum Payload {
    Request {
        chan: Sender<anyhow::Result<Value>>,
        value: jsonrpc::MethodCall,
    },
    Notification(jsonrpc::Notification),
}

struct Transport {
    incoming: Sender<Message>,
    outgoing: Receiver<Payload>,

    pending_requests: HashMap<jsonrpc::Id, Sender<anyhow::Result<Value>>>,
    headers: HashMap<String, String>,

    writer: BufWriter<ChildStdin>,
    reader: BufReader<ChildStdout>,
}

impl Transport {
    pub fn start(
        ex: &Executor,
        reader: BufReader<ChildStdout>,
        writer: BufWriter<ChildStdin>,
    ) -> (Receiver<Message>, Sender<Payload>) {
        let (incoming, rx) = smol::channel::unbounded();
        let (tx, outgoing) = smol::channel::unbounded();

        let transport = Self {
            reader,
            writer,
            incoming,
            outgoing,
            pending_requests: Default::default(),
            headers: Default::default(),
        };

        ex.spawn(transport.duplex()).detach();

        (rx, tx)
    }

    async fn recv(
        reader: &mut (impl AsyncBufRead + Unpin),
        headers: &mut HashMap<String, String>,
    ) -> Result<Message, std::io::Error> {
        // read headers
        loop {
            let mut header = String::new();
            // detect pipe closed if 0
            reader.read_line(&mut header).await?;
            let header = header.trim();

            if header.is_empty() {
                break;
            }

            let parts: Vec<&str> = header.split(": ").collect();
            if parts.len() != 2 {
                // return Err(Error::new(ErrorKind::Other, "Failed to parse header"));
                panic!()
            }
            headers.insert(parts[0].to_string(), parts[1].to_string());
        }

        // find content-length
        let content_length = headers.get("Content-Length").unwrap().parse().unwrap();

        let mut content = vec![0; content_length];
        reader.read_exact(&mut content).await?;
        let msg = String::from_utf8(content).unwrap();

        // read data

        // try parsing as output (server response) or call (server request)
        let output: serde_json::Result<Message> = serde_json::from_str(&msg);

        Ok(output?)
    }

    pub async fn send_payload(&mut self, payload: Payload) -> anyhow::Result<()> {
        match payload {
            Payload::Request { chan, value } => {
                self.pending_requests.insert(value.id.clone(), chan);

                let json = serde_json::to_string(&value)?;
                self.send(json).await
            }
            Payload::Notification(value) => {
                let json = serde_json::to_string(&value)?;
                self.send(json).await
            }
        }
    }

    pub async fn send(&mut self, request: String) -> anyhow::Result<()> {
        println!("-> {}", request);

        // send the headers
        self.writer
            .write_all(format!("Content-Length: {}\r\n\r\n", request.len()).as_bytes())
            .await?;

        // send the body
        self.writer.write_all(request.as_bytes()).await?;

        self.writer.flush().await?;

        Ok(())
    }

    pub async fn recv_response(&mut self, output: jsonrpc::Output) -> anyhow::Result<()> {
        match output {
            jsonrpc::Output::Success(jsonrpc::Success { id, result, .. }) => {
                println!("<- {}", result);

                let tx = self
                    .pending_requests
                    .remove(&id)
                    .expect("pending_request with id not found!");
                tx.send(Ok(result)).await?;
            }
            jsonrpc::Output::Failure(_) => panic!("recv fail"),
            msg => unimplemented!("{:?}", msg),
        }
        Ok(())
    }

    pub async fn duplex(mut self) {
        loop {
            select! {
                // client -> server
                msg = self.outgoing.next().fuse() => {
                    if msg.is_none() {
                        break;
                    }
                    let msg = msg.unwrap();

                    self.send_payload(msg).await.unwrap();
                }
                // server <- client
                msg = Self::recv(&mut self.reader, &mut self.headers).fuse() => {
                    if msg.is_err() {
                        break;
                    }
                    let msg = msg.unwrap();

                    match msg {
                        Message::Output(output) => self.recv_response(output).await.unwrap(),
                        Message::Notification(_) => {
                            // dispatch
                        }
                        Message::Call(_) => {
                            // dispatch
                        }
                    };
                }
            }
        }
    }
}
