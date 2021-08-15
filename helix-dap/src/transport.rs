use crate::{Error, Result};
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

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct Event {
    // seq is omitted as unused and is not sent by some implementations
    pub event: String,
    pub body: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
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
        server_stdout: Box<dyn AsyncBufRead + Unpin + Send>,
        server_stdin: Box<dyn AsyncWrite + Unpin + Send>,
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
        reader: &mut Box<dyn AsyncBufRead + Unpin + Send>,
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
        server_stdin: &mut Box<dyn AsyncWrite + Unpin + Send>,
        req: Request,
    ) -> Result<()> {
        let json = serde_json::to_string(&Payload::Request(req.clone()))?;
        if let Some(back) = req.back_ch {
            self.pending_requests.lock().await.insert(req.seq, back);
        }
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
        match res.success {
            true => {
                info!("<- DAP success in response to {}", res.request_seq);

                Ok(res)
            }
            false => {
                error!(
                    "<- DAP error {:?} ({:?}) for command #{} {}",
                    res.message, res.body, res.request_seq, res.command
                );

                Err(Error::Other(anyhow::format_err!("{:?}", res.body)))
            }
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
            Payload::Event(Event { ref event, .. }) => {
                info!("<- DAP event {}", event);
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
