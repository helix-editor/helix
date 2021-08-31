use crate::{Error, Result};
use anyhow::Context;
use jsonrpc_core as jsonrpc;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    process::{ChildStderr, ChildStdin, ChildStdout},
    sync::{
        mpsc::{unbounded_channel, Sender, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
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
    pending_requests: Mutex<HashMap<jsonrpc::Id, Sender<Result<Value>>>>,
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
            pending_requests: Mutex::new(HashMap::default()),
        };

        let transport = Arc::new(transport);

        tokio::spawn(Self::recv(transport.clone(), server_stdout, client_tx));
        tokio::spawn(Self::err(transport.clone(), server_stderr));
        tokio::spawn(Self::send(transport, server_stdin, client_rx));

        (rx, tx)
    }

    async fn recv_server_message(
        reader: &mut (impl AsyncBufRead + Unpin + Send),
        buffer: &mut String,
    ) -> Result<ServerMessage> {
        let mut content_length = None;
        loop {
            buffer.truncate(0);
            if reader.read_line(buffer).await? == 0 {
                return Err(Error::StreamClosed);
            };

            // debug!("<- header {:?}", buffer);

            if header.is_empty() {
                break;
            }

            let header = buffer.trim();

            let parts = header.split_once(": ");

            match parts {
                Some(("Content-Length", value)) => {
                    content_length = Some(value.parse().context("invalid content length")?);
                }
                Some((_, _)) => {}
                None => {
                    // Workaround: Some non-conformant language servers will output logging and other garbage
                    // into the same stream as JSON-RPC messages. This can also happen from shell scripts that spawn
                    // the server. Skip such lines and log a warning.
                    warn!("Failed to parse header: {:?}", header);
                }
            }
        }

        let content_length = content_length.context("missing content length")?;

        //TODO: reuse vector
        let mut content = vec![0; content_length];
        reader.read_exact(&mut content).await?;
        let msg = std::str::from_utf8(&content).context("invalid utf8 from server")?;

        info!("<- {}", msg);

        // try parsing as output (server response) or call (server request)
        let output: serde_json::Result<ServerMessage> = serde_json::from_str(msg);

        Ok(output?)
    }

    async fn recv_server_error(
        err: &mut (impl AsyncBufRead + Unpin + Send),
        buffer: &mut String,
    ) -> Result<()> {
        buffer.truncate(0);
        if err.read_line(buffer).await? == 0 {
            return Err(Error::StreamClosed);
        };
        error!("err <- {:?}", buffer);

        Ok(())
    }

    async fn send_payload_to_server(
        &self,
        server_stdin: &mut BufWriter<ChildStdin>,
        payload: Payload,
    ) -> Result<()> {
        //TODO: reuse string
        let json = match payload {
            Payload::Request { chan, value } => {
                self.pending_requests
                    .lock()
                    .await
                    .insert(value.id.clone(), chan);
                serde_json::to_string(&value)?
            }
            Payload::Notification(value) => serde_json::to_string(&value)?,
            Payload::Response(error) => serde_json::to_string(&error)?,
        };
        self.send_string_to_server(server_stdin, json).await
    }

    async fn send_string_to_server(
        &self,
        server_stdin: &mut BufWriter<ChildStdin>,
        request: String,
    ) -> Result<()> {
        info!("-> {}", request);

        // send the headers
        server_stdin
            .write_all(format!("Content-Length: {}\r\n\r\n", request.len()).as_bytes())
            .await?;

        // send the body
        server_stdin.write_all(request.as_bytes()).await?;

        server_stdin.flush().await?;

        Ok(())
    }

    async fn process_server_message(
        &self,
        client_tx: &UnboundedSender<(usize, jsonrpc::Call)>,
        msg: ServerMessage,
    ) -> Result<()> {
        match msg {
            ServerMessage::Output(output) => self.process_request_response(output).await?,
            ServerMessage::Call(call) => {
                client_tx
                    .send((self.id, call))
                    .context("failed to send a message to server")?;
                // let notification = Notification::parse(&method, params);
            }
        };
        Ok(())
    }

    async fn process_request_response(&self, output: jsonrpc::Output) -> Result<()> {
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
            .lock()
            .await
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

    async fn recv(
        transport: Arc<Self>,
        mut server_stdout: BufReader<ChildStdout>,
        client_tx: UnboundedSender<(usize, jsonrpc::Call)>,
    ) {
        let mut recv_buffer = String::new();
        loop {
            match Self::recv_server_message(&mut server_stdout, &mut recv_buffer).await {
                Ok(msg) => {
                    match transport.process_server_message(&client_tx, msg).await {
                        Ok(_) => {}
                        Err(err) => {
                            error!("err: <- {:?}", err);
                            break;
                        }
                    };
                }
                Err(err) => {
                    error!("err: <- {:?}", err);
                    break;
                }
            }
        }
    }

    async fn err(_transport: Arc<Self>, mut server_stderr: BufReader<ChildStderr>) {
        let mut recv_buffer = String::new();
        loop {
            match Self::recv_server_error(&mut server_stderr, &mut recv_buffer).await {
                Ok(_) => {}
                Err(err) => {
                    error!("err: <- {:?}", err);
                    break;
                }
            }
        }
    }

    async fn send(
        transport: Arc<Self>,
        mut server_stdin: BufWriter<ChildStdin>,
        mut client_rx: UnboundedReceiver<Payload>,
    ) {
        while let Some(msg) = client_rx.recv().await {
            match transport
                .send_payload_to_server(&mut server_stdin, msg)
                .await
            {
                Ok(_) => {}
                Err(err) => {
                    error!("err: <- {:?}", err);
                }
            }
        }
    }
}
