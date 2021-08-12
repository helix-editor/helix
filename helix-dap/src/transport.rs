use crate::{Error, Result};
use anyhow::Context;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    process::{ChildStdin, ChildStdout},
    sync::{
        mpsc::{unbounded_channel, Sender, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Request {
    #[serde(skip)]
    pub back_ch: Option<Sender<Result<Response>>>,
    pub seq: u64,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub command: String,
    pub arguments: Option<Value>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct Response {
    pub seq: u64,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub request_seq: u64,
    pub success: bool,
    pub command: String,
    pub message: Option<String>,
    pub body: Option<Value>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct Event {
    pub seq: u64,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub event: String,
    pub body: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Payload {
    // type = "event"
    Event(Event),
    // type = "response"
    Response(Response),
    // type = "request"
    Request(Request),
}

#[derive(Debug)]
pub struct Transport {
    id: usize,
    pending_requests: Mutex<HashMap<u64, Sender<Result<Response>>>>,
}

impl Transport {
    pub fn start(
        server_stdout: BufReader<ChildStdout>,
        server_stdin: BufWriter<ChildStdin>,
        id: usize,
    ) -> (UnboundedReceiver<Payload>, UnboundedSender<Request>) {
        let (client_tx, rx) = unbounded_channel();
        let (tx, client_rx) = unbounded_channel();

        let transport = Self {
            id,
            pending_requests: Mutex::new(HashMap::default()),
        };

        let transport = Arc::new(transport);

        tokio::spawn(Self::recv(transport.clone(), server_stdout, client_tx));
        tokio::spawn(Self::send(transport, server_stdin, client_rx));

        (rx, tx)
    }

    async fn recv_server_message(
        reader: &mut (impl AsyncBufRead + Unpin + Send),
        buffer: &mut String,
    ) -> Result<Payload> {
        let mut content_length = None;
        loop {
            buffer.truncate(0);
            reader.read_line(buffer).await?;
            let header = buffer.trim();

            if header.is_empty() {
                break;
            }

            let mut parts = header.split(": ");

            match (parts.next(), parts.next(), parts.next()) {
                (Some("Content-Length"), Some(value), None) => {
                    content_length = Some(value.parse().context("invalid content length")?);
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

        let content_length = content_length.context("missing content length")?;

        //TODO: reuse vector
        let mut content = vec![0; content_length];
        reader.read_exact(&mut content).await?;
        let msg = std::str::from_utf8(&content).context("invalid utf8 from server")?;

        info!("<- DAP {}", msg);

        // try parsing as output (server response) or call (server request)
        let output: serde_json::Result<Payload> = serde_json::from_str(msg);

        Ok(output?)
    }

    async fn send_payload_to_server(
        &self,
        server_stdin: &mut BufWriter<ChildStdin>,
        req: Request,
    ) -> Result<()> {
        let json = serde_json::to_string(&req)?;
        if let Some(back) = req.back_ch {
            self.pending_requests.lock().await.insert(req.seq, back);
        }
        self.send_string_to_server(server_stdin, json).await
    }

    async fn send_string_to_server(
        &self,
        server_stdin: &mut BufWriter<ChildStdin>,
        request: String,
    ) -> Result<()> {
        info!("-> DAP {}", request);

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
        client_tx: &UnboundedSender<Payload>,
        msg: Payload,
    ) -> Result<()> {
        let (id, result) = match msg {
            Payload::Response(Response {
                success: true,
                seq,
                request_seq,
                ..
            }) => {
                info!("<- DAP success ({}, in response to {})", seq, request_seq);
                if let Payload::Response(val) = msg {
                    (request_seq, Ok(val))
                } else {
                    unreachable!();
                }
            }
            Payload::Response(Response {
                success: false,
                message,
                body,
                request_seq,
                command,
                ..
            }) => {
                error!(
                    "<- DAP error {:?} ({:?}) for command #{} {}",
                    message, body, request_seq, command
                );
                (
                    request_seq,
                    Err(Error::Other(anyhow::format_err!("{:?}", body))),
                )
            }
            Payload::Request(Request {
                ref command,
                ref seq,
                ..
            }) => {
                info!("<- DAP request {} #{}", command, seq);
                client_tx.send(msg).expect("Failed to send");
                return Ok(());
            }
            Payload::Event(Event {
                ref event, ref seq, ..
            }) => {
                info!("<- DAP event {} #{}", event, seq);
                client_tx.send(msg).expect("Failed to send");
                return Ok(());
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
        client_tx: UnboundedSender<Payload>,
    ) {
        let mut recv_buffer = String::new();
        loop {
            match Self::recv_server_message(&mut server_stdout, &mut recv_buffer).await {
                Ok(msg) => {
                    transport
                        .process_server_message(&client_tx, msg)
                        .await
                        .unwrap();
                }
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
        mut client_rx: UnboundedReceiver<Request>,
    ) {
        while let Some(req) = client_rx.recv().await {
            transport
                .send_payload_to_server(&mut server_stdin, req)
                .await
                .unwrap()
        }
    }
}
