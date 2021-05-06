use std::collections::HashMap;
use std::io;

use log::{error, info};

use crate::Error;

type Result<T> = core::result::Result<T, Error>;

use jsonrpc_core as jsonrpc;
use serde_json::Value;

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

use serde::{Deserialize, Serialize};
/// A type representing all possible values sent from the server to the client.
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
enum Message {
    /// A regular JSON-RPC request output (single response).
    Output(jsonrpc::Output),
    /// A JSON-RPC request or notification.
    Call(jsonrpc::Call),
}

pub struct Transport {
    incoming: UnboundedSender<jsonrpc::Call>,
    outgoing: UnboundedReceiver<Payload>,

    pending_requests: HashMap<jsonrpc::Id, Sender<Result<Value>>>,
    headers: HashMap<String, String>,

    writer: BufWriter<ChildStdin>,
    reader: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
}

impl Transport {
    pub fn start(
        reader: BufReader<ChildStdout>,
        writer: BufWriter<ChildStdin>,
        stderr: BufReader<ChildStderr>,
    ) -> (UnboundedReceiver<jsonrpc::Call>, UnboundedSender<Payload>) {
        let (incoming, rx) = unbounded_channel();
        let (tx, outgoing) = unbounded_channel();

        let transport = Self {
            reader,
            writer,
            stderr,
            incoming,
            outgoing,
            pending_requests: HashMap::default(),
            headers: HashMap::default(),
        };

        tokio::spawn(transport.duplex());

        (rx, tx)
    }

    async fn recv(
        reader: &mut (impl AsyncBufRead + Unpin + Send),
        headers: &mut HashMap<String, String>,
    ) -> core::result::Result<Message, std::io::Error> {
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
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to parse header",
                ));
            }
            headers.insert(parts[0].to_string(), parts[1].to_string());
        }

        // find content-length
        let content_length = headers.get("Content-Length").unwrap().parse().unwrap();

        let mut content = vec![0; content_length];
        reader.read_exact(&mut content).await?;
        let msg = String::from_utf8(content).unwrap();

        // read data

        info!("<- {}", msg);

        // try parsing as output (server response) or call (server request)
        let output: serde_json::Result<Message> = serde_json::from_str(&msg);

        Ok(output?)
    }

    async fn err(
        err: &mut (impl AsyncBufRead + Unpin + Send),
    ) -> core::result::Result<(), std::io::Error> {
        let mut line = String::new();
        err.read_line(&mut line).await?;
        error!("err <- {}", line);

        Ok(())
    }

    pub async fn send_payload(&mut self, payload: Payload) -> io::Result<()> {
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
            Payload::Response(error) => {
                let json = serde_json::to_string(&error)?;
                self.send(json).await
            }
        }
    }

    pub async fn send(&mut self, request: String) -> io::Result<()> {
        info!("-> {}", request);

        // send the headers
        self.writer
            .write_all(format!("Content-Length: {}\r\n\r\n", request.len()).as_bytes())
            .await?;

        // send the body
        self.writer.write_all(request.as_bytes()).await?;

        self.writer.flush().await?;

        Ok(())
    }

    async fn recv_msg(&mut self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::Output(output) => self.recv_response(output).await?,
            Message::Call(call) => {
                self.incoming.send(call).unwrap();
                // let notification = Notification::parse(&method, params);
            }
        };
        Ok(())
    }

    async fn recv_response(&mut self, output: jsonrpc::Output) -> io::Result<()> {
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

    pub async fn duplex(mut self) {
        loop {
            tokio::select! {
                // client -> server
                msg = self.outgoing.recv() => {
                    if msg.is_none() {
                        break;
                    }
                    let msg = msg.unwrap();

                    self.send_payload(msg).await.unwrap();
                }
                // server <- client
                msg = Self::recv(&mut self.reader, &mut self.headers) => {
                    if msg.is_err() {
                        error!("err: <- {:?}", msg);
                        break;
                    }
                    let msg = msg.unwrap();

                    self.recv_msg(msg).await.unwrap();
                }
                _msg = Self::err(&mut self.stderr) => {}
            }
        }
    }
}
