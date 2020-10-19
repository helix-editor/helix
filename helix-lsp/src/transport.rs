use std::collections::HashMap;

use crate::{Message, Notification};

use jsonrpc_core as jsonrpc;
use lsp_types as lsp;
use serde_json::Value;

use smol::prelude::*;

use smol::{
    channel::{Receiver, Sender},
    io::{BufReader, BufWriter},
    process::{ChildStderr, ChildStdin, ChildStdout},
    Executor,
};

pub(crate) enum Payload {
    Request {
        chan: Sender<anyhow::Result<Value>>,
        value: jsonrpc::MethodCall,
    },
    Notification(jsonrpc::Notification),
}

pub(crate) struct Transport {
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

    pub async fn recv_msg(&mut self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::Output(output) => self.recv_response(output).await?,
            Message::Notification(jsonrpc::Notification { method, params, .. }) => {
                let notification = Notification::parse(&method, params);

                println!("<- {} {:?}", method, notification);
                // dispatch
            }
            Message::Call(call) => {
                println!("<- {:?}", call);
                // dispatch
            }
            _ => unreachable!(),
        };
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
        use futures_util::{select, FutureExt};
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

                    self.recv_msg(msg).await.unwrap();
                }
            }
        }
    }
}
