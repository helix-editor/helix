use crate::Result;
use jsonrpc_core as jsonrpc;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    process::{ChildStderr, ChildStdin, ChildStdout},
    sync::mpsc::{unbounded_channel, Sender, UnboundedReceiver, UnboundedSender},
};

#[derive(Debug)]
pub enum Payload {
    Request {
        chan: Sender<Result<Value>>,
        value: jsonrpc::MethodCall,
    },
    Notification(jsonrpc::Notification),
    Response(jsonrpc::Output),
}

/// A type representing all possible values sent from the server to the client.
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
enum ServerMessage {
    /// A regular JSON-RPC request output (single response).
    Output(jsonrpc::Output),
    /// A JSON-RPC request or notification.
    Call(jsonrpc::Call),
}

#[derive(Debug)]
pub struct Transport {
    id: usize,
    client_tx: UnboundedSender<(usize, jsonrpc::Call)>,
    client_rx: UnboundedReceiver<Payload>,

    pending_requests: HashMap<jsonrpc::Id, Sender<Result<Value>>>,

    server_stdin: BufWriter<ChildStdin>,
    server_stdout: BufReader<ChildStdout>,
    server_stderr: BufReader<ChildStderr>,
}

impl Transport {
    pub fn start(
        server_stdout: BufReader<ChildStdout>,
        server_stdin: BufWriter<ChildStdin>,
        server_stderr: BufReader<ChildStderr>,
        id: usize,
    ) -> (
        UnboundedReceiver<(usize, jsonrpc::Call)>,
        UnboundedSender<Payload>,
    ) {
        let (client_tx, rx) = unbounded_channel();
        let (tx, client_rx) = unbounded_channel();

        let transport = Self {
            id,
            server_stdout,
            server_stdin,
            server_stderr,
            client_tx,
            client_rx,
            pending_requests: HashMap::default(),
        };

        tokio::spawn(transport.duplex());

        (rx, tx)
    }

    async fn recv_server_message(
        reader: &mut (impl AsyncBufRead + Unpin + Send),
        buffer: &mut String,
    ) -> Result<ServerMessage> {
        let mut content_length = None;
        loop {
            buffer.truncate(0);
            reader.read_line(buffer).await?;
            let header = buffer.trim();

            if header.is_empty() && content_length.is_some() {
                break;
            }

            let mut parts = header.split(": ");

            match (parts.next(), parts.next(), parts.next()) {
                (Some("Content-Length"), Some(value), None) => {
                    content_length = Some(value.parse().unwrap());
                }
                (Some(_), Some(_), None) => {}
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Failed to parse header",
                    )
                    .into());
                }
            }
        }

        let content_length = content_length.unwrap();

        //TODO: reuse vector
        let mut content = vec![0; content_length];
        reader.read_exact(&mut content).await?;
        let msg = String::from_utf8(content).unwrap();

        info!("<- {}", msg);

        // try parsing as output (server response) or call (server request)
        let output: serde_json::Result<ServerMessage> = serde_json::from_str(&msg);

        Ok(output?)
    }

    async fn recv_server_error(
        err: &mut (impl AsyncBufRead + Unpin + Send),
        buffer: &mut String,
    ) -> Result<()> {
        buffer.truncate(0);
        err.read_line(buffer).await?;
        error!("err <- {}", buffer);

        Ok(())
    }

    async fn send_payload_to_server(&mut self, payload: Payload) -> Result<()> {
        //TODO: reuse string
        let json = match payload {
            Payload::Request { chan, value } => {
                self.pending_requests.insert(value.id.clone(), chan);
                serde_json::to_string(&value)?
            }
            Payload::Notification(value) => serde_json::to_string(&value)?,
            Payload::Response(error) => serde_json::to_string(&error)?,
        };
        self.send_string_to_server(json).await
    }

    async fn send_string_to_server(&mut self, request: String) -> Result<()> {
        info!("-> {}", request);

        // send the headers
        self.server_stdin
            .write_all(format!("Content-Length: {}\r\n\r\n", request.len()).as_bytes())
            .await?;

        // send the body
        self.server_stdin.write_all(request.as_bytes()).await?;

        self.server_stdin.flush().await?;

        Ok(())
    }

    async fn process_server_message(&mut self, msg: ServerMessage) -> Result<()> {
        match msg {
            ServerMessage::Output(output) => self.process_request_response(output).await?,
            ServerMessage::Call(call) => {
                self.client_tx.send((self.id, call)).unwrap();
                // let notification = Notification::parse(&method, params);
            }
        };
        Ok(())
    }

    async fn process_request_response(&mut self, output: jsonrpc::Output) -> Result<()> {
        let (id, result) = match output {
            jsonrpc::Output::Success(jsonrpc::Success { id, result, .. }) => {
                info!("<- {}", result);
                (id, Ok(result))
            }
            jsonrpc::Output::Failure(jsonrpc::Failure { id, error, .. }) => {
                error!("<- {}", error);
                (id, Err(error.into()))
            }
        };

        let tx = self
            .pending_requests
            .remove(&id)
            .expect("pending_request with id not found!");

        match tx.send(result).await {
            Ok(_) => (),
            Err(_) => error!(
                "Tried sending response into a closed channel (id={:?}), original request likely timed out",
                id
            ),
        };

        Ok(())
    }

    async fn duplex(mut self) {
        let mut recv_buffer = String::new();
        let mut err_buffer = String::new();
        loop {
            tokio::select! {
                // client -> server
                msg = self.client_rx.recv() => {
                    match msg {
                        Some(msg) => {
                            self.send_payload_to_server(msg).await.unwrap()
                        },
                        None => break
                    }
                }
                // server -> client
                msg = Self::recv_server_message(&mut self.server_stdout, &mut recv_buffer) => {
                    match msg {
                        Ok(msg) => {
                            self.process_server_message(msg).await.unwrap();
                        }
                        Err(_) => {
                            error!("err: <- {:?}", msg);
                            break;
                        },
                    }
                }
                _msg = Self::recv_server_error(&mut self.server_stderr, &mut err_buffer) => {}
            }
        }
    }
}
