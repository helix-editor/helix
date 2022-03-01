use crate::{Error, Event, Result};
use anyhow::Context;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt},
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
    pub command: String,
    pub arguments: Option<Value>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct Response {
    // seq is omitted as unused and is not sent by some implementations
    pub request_seq: u64,
    pub success: bool,
    pub command: String,
    pub message: Option<String>,
    pub body: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Payload {
    // type = "event"
    Event(Box<Event>),
    // type = "response"
    Response(Response),
    // type = "request"
    Request(Request),
}

#[derive(Debug)]
pub struct Transport {
    #[allow(unused)]
    id: usize,
    pending_requests: Mutex<HashMap<u64, Sender<Result<Response>>>>,
}

impl Transport {
    pub fn start(
        server_stdout: Box<dyn AsyncBufRead + Unpin + Send>,
        server_stdin: Box<dyn AsyncWrite + Unpin + Send>,
        server_stderr: Option<Box<dyn AsyncBufRead + Unpin + Send>>,
        id: usize,
    ) -> (UnboundedReceiver<Payload>, UnboundedSender<Payload>) {
        let (client_tx, rx) = unbounded_channel();
        let (tx, client_rx) = unbounded_channel();

        let transport = Self {
            id,
            pending_requests: Mutex::new(HashMap::default()),
        };

        let transport = Arc::new(transport);

        tokio::spawn(Self::recv(transport.clone(), server_stdout, client_tx));
        tokio::spawn(Self::send(transport, server_stdin, client_rx));
        if let Some(stderr) = server_stderr {
            tokio::spawn(Self::err(stderr));
        }

        (rx, tx)
    }

    async fn recv_server_message(
        reader: &mut Box<dyn AsyncBufRead + Unpin + Send>,
        buffer: &mut String,
    ) -> Result<Payload> {
        let mut content_length = None;
        loop {
            buffer.truncate(0);
            if reader.read_line(buffer).await? == 0 {
                return Err(Error::StreamClosed);
            };

            if buffer == "\r\n" {
                // look for an empty CRLF line
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

                    // warn!("Failed to parse header: {:?}", header);
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

    async fn recv_server_error(
        err: &mut (impl AsyncBufRead + Unpin + Send),
        buffer: &mut String,
    ) -> Result<()> {
        buffer.truncate(0);
        if err.read_line(buffer).await? == 0 {
            return Err(Error::StreamClosed);
        };
        error!("err <- {}", buffer);

        Ok(())
    }

    async fn send_payload_to_server(
        &self,
        server_stdin: &mut Box<dyn AsyncWrite + Unpin + Send>,
        mut payload: Payload,
    ) -> Result<()> {
        if let Payload::Request(request) = &mut payload {
            if let Some(back) = request.back_ch.take() {
                self.pending_requests.lock().await.insert(request.seq, back);
            }
        }
        let json = serde_json::to_string(&payload)?;
        self.send_string_to_server(server_stdin, json).await
    }

    async fn send_string_to_server(
        &self,
        server_stdin: &mut Box<dyn AsyncWrite + Unpin + Send>,
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

    fn process_response(res: Response) -> Result<Response> {
        if res.success {
            info!("<- DAP success in response to {}", res.request_seq);

            Ok(res)
        } else {
            error!(
                "<- DAP error {:?} ({:?}) for command #{} {}",
                res.message, res.body, res.request_seq, res.command
            );

            Err(Error::Other(anyhow::format_err!("{:?}", res.body)))
        }
    }

    async fn process_server_message(
        &self,
        client_tx: &UnboundedSender<Payload>,
        msg: Payload,
    ) -> Result<()> {
        match msg {
            Payload::Response(res) => {
                let request_seq = res.request_seq;
                let tx = self.pending_requests.lock().await.remove(&request_seq);

                match tx {
                    Some(tx) => match tx.send(Self::process_response(res)).await {
                        Ok(_) => (),
                        Err(_) => error!(
                            "Tried sending response into a closed channel (id={:?}), original request likely timed out",
                            request_seq
                        ),
                    }
                    None => {
                        warn!("Response to nonexistent request #{}", res.request_seq);
                        client_tx.send(Payload::Response(res)).expect("Failed to send");
                    }
                }

                Ok(())
            }
            Payload::Request(Request {
                ref command,
                ref seq,
                ..
            }) => {
                info!("<- DAP request {} #{}", command, seq);
                client_tx.send(msg).expect("Failed to send");
                Ok(())
            }
            Payload::Event(ref event) => {
                info!("<- DAP event {:?}", event);
                client_tx.send(msg).expect("Failed to send");
                Ok(())
            }
        }
    }

    async fn recv(
        transport: Arc<Self>,
        mut server_stdout: Box<dyn AsyncBufRead + Unpin + Send>,
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
        mut server_stdin: Box<dyn AsyncWrite + Unpin + Send>,
        mut client_rx: UnboundedReceiver<Payload>,
    ) {
        while let Some(payload) = client_rx.recv().await {
            transport
                .send_payload_to_server(&mut server_stdin, payload)
                .await
                .unwrap()
        }
    }

    async fn err(mut server_stderr: Box<dyn AsyncBufRead + Unpin + Send>) {
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
}
