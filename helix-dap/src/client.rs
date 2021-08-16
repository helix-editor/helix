use crate::{
    transport::{Event, Payload, Request, Response, Transport},
    types::*,
    Result,
};
pub use log::{error, info};
use serde::Serialize;
use serde_json::{from_value, to_value, Value};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    process::Stdio,
};
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    io::{AsyncBufRead, AsyncWrite, BufReader, BufWriter},
    join,
    net::TcpStream,
    process::{Child, Command},
    sync::{
        mpsc::{channel, Receiver, Sender, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
    time,
};

#[derive(Debug)]
pub struct Client {
    id: usize,
    _process: Option<Child>,
    server_tx: UnboundedSender<Request>,
    request_counter: AtomicU64,
    capabilities: Option<DebuggerCapabilities>,
    awaited_events: Arc<Mutex<HashMap<String, Sender<Event>>>>,
}

impl Client {
    pub fn streams(
        rx: Box<dyn AsyncBufRead + Unpin + Send>,
        tx: Box<dyn AsyncWrite + Unpin + Send>,
        id: usize,
        process: Option<Child>,
    ) -> Result<Self> {
        let (server_rx, server_tx) = Transport::start(rx, tx, id);

        let client = Self {
            id,
            _process: process,
            server_tx,
            request_counter: AtomicU64::new(0),
            capabilities: None,
            awaited_events: Arc::new(Mutex::new(HashMap::default())),
        };

        tokio::spawn(Self::recv(Arc::clone(&client.awaited_events), server_rx));

        Ok(client)
    }

    pub async fn tcp(addr: std::net::SocketAddr, id: usize) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (rx, tx) = stream.into_split();
        Self::streams(Box::new(BufReader::new(rx)), Box::new(tx), id, None)
    }

    pub fn stdio(cmd: &str, args: Vec<&str>, id: usize) -> Result<Self> {
        let process = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // make sure the process is reaped on drop
            .kill_on_drop(true)
            .spawn();

        let mut process = process?;

        // TODO: do we need bufreader/writer here? or do we use async wrappers on unblock?
        let writer = BufWriter::new(process.stdin.take().expect("Failed to open stdin"));
        let reader = BufReader::new(process.stdout.take().expect("Failed to open stdout"));

        Self::streams(
            Box::new(BufReader::new(reader)),
            Box::new(writer),
            id,
            Some(process),
        )
    }

    async fn get_port() -> Option<u16> {
        Some(
            tokio::net::TcpListener::bind(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                0,
            ))
            .await
            .ok()?
            .local_addr()
            .ok()?
            .port(),
        )
    }

    pub async fn tcp_process(
        cmd: &str,
        args: Vec<&str>,
        port_format: &str,
        id: usize,
    ) -> Result<Self> {
        let port = Self::get_port().await.unwrap();

        let process = Command::new(cmd)
            .args(args)
            .args(port_format.replace("{}", &port.to_string()).split(' '))
            // make sure the process is reaped on drop
            .kill_on_drop(true)
            .spawn()?;

        // Wait for adapter to become ready for connection
        time::sleep(time::Duration::from_millis(500)).await;

        let stream = TcpStream::connect(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port,
        ))
        .await?;

        let (rx, tx) = stream.into_split();
        Self::streams(
            Box::new(BufReader::new(rx)),
            Box::new(tx),
            id,
            Some(process),
        )
    }

    async fn recv(
        awaited_events: Arc<Mutex<HashMap<String, Sender<Event>>>>,
        mut server_rx: UnboundedReceiver<Payload>,
    ) {
        while let Some(msg) = server_rx.recv().await {
            match msg {
                Payload::Event(ev) => {
                    let name = ev.event.clone();
                    let hashmap = awaited_events.lock().await;
                    let tx = hashmap.get(&name);

                    match tx {
                        Some(tx) => match tx.send(ev).await {
                            Ok(_) => (),
                            Err(_) => error!(
                                "Tried sending event into a closed channel (name={:?})",
                                name
                            ),
                        },
                        None => {
                            info!("unhandled event");
                            // client_tx.send(Payload::Event(ev)).expect("Failed to send");
                        }
                    }
                }
                Payload::Response(_) => unreachable!(),
                Payload::Request(_) => todo!(),
            }
        }
    }

    pub async fn listen_for_event(&self, name: String) -> Receiver<Event> {
        let (rx, tx) = channel(1);
        self.awaited_events.lock().await.insert(name.clone(), rx);
        tx
    }

    pub fn id(&self) -> usize {
        self.id
    }

    fn next_request_id(&self) -> u64 {
        self.request_counter.fetch_add(1, Ordering::Relaxed)
    }

    async fn request(&self, command: String, arguments: Option<Value>) -> Result<Response> {
        let (callback_rx, mut callback_tx) = channel(1);

        let req = Request {
            back_ch: Some(callback_rx),
            seq: self.next_request_id(),
            command,
            arguments,
        };

        self.server_tx
            .send(req)
            .expect("Failed to send request to debugger");

        Ok(callback_tx.recv().await.unwrap()?)
    }

    pub fn capabilities(&self) -> &DebuggerCapabilities {
        self.capabilities
            .as_ref()
            .expect("language server not yet initialized!")
    }

    pub async fn initialize(&mut self, adapter_id: String) -> Result<()> {
        let args = requests::InitializeArguments {
            client_id: Some("hx".to_owned()),
            client_name: Some("helix".to_owned()),
            adapter_id,
            locale: Some("en-us".to_owned()),
            lines_start_at_one: Some(true),
            columns_start_at_one: Some(true),
            path_format: Some("path".to_owned()),
            supports_variable_type: Some(false),
            supports_variable_paging: Some(false),
            supports_run_in_terminal_request: Some(false),
            supports_memory_references: Some(false),
            supports_progress_reporting: Some(false),
            supports_invalidated_event: Some(false),
        };

        let response = self
            .request("initialize".to_owned(), to_value(args).ok())
            .await?;
        self.capabilities = from_value(response.body.unwrap()).ok();

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        self.request("disconnect".to_owned(), None).await?;
        Ok(())
    }

    pub async fn launch(&mut self, args: impl Serialize) -> Result<()> {
        let mut initialized = self.listen_for_event("initialized".to_owned()).await;

        let res = self.request("launch".to_owned(), to_value(args).ok());
        let ev = initialized.recv();
        join!(res, ev).0?;

        Ok(())
    }

    pub async fn attach(&mut self, args: impl Serialize) -> Result<()> {
        let mut initialized = self.listen_for_event("initialized".to_owned()).await;

        let res = self.request("attach".to_owned(), to_value(args).ok());
        let ev = initialized.recv();
        join!(res, ev).0?;

        Ok(())
    }

    pub async fn set_breakpoints(
        &mut self,
        file: String,
        breakpoints: Vec<SourceBreakpoint>,
    ) -> Result<Option<Vec<Breakpoint>>> {
        let args = requests::SetBreakpointsArguments {
            source: Source {
                path: Some(file),
                name: None,
                source_reference: None,
                presentation_hint: None,
                origin: None,
                sources: None,
                adapter_data: None,
                checksums: None,
            },
            breakpoints: Some(breakpoints),
            source_modified: Some(false),
        };

        let response = self
            .request("setBreakpoints".to_owned(), to_value(args).ok())
            .await?;
        let body: Option<requests::SetBreakpointsResponse> = from_value(response.body.unwrap()).ok();

        Ok(body.map(|b| b.breakpoints).unwrap())
    }

    pub async fn configuration_done(&mut self) -> Result<()> {
        self.request("configurationDone".to_owned(), None).await?;
        Ok(())
    }

    pub async fn continue_thread(&mut self, thread_id: usize) -> Result<Option<bool>> {
        let args = requests::ContinueArguments { thread_id };

        let response = self
            .request("continue".to_owned(), to_value(args).ok())
            .await?;

        let body: Option<requests::ContinueResponse> = from_value(response.body.unwrap()).ok();

        Ok(body.map(|b| b.all_threads_continued).unwrap())
    }

    pub async fn stack_trace(
        &mut self,
        thread_id: usize,
    ) -> Result<(Vec<StackFrame>, Option<usize>)> {
        let args = requests::StackTraceArguments {
            thread_id,
            start_frame: None,
            levels: None,
            format: None,
        };

        let response = self
            .request("stackTrace".to_owned(), to_value(args).ok())
            .await?;

        let body: requests::StackTraceResponse = from_value(response.body.unwrap()).unwrap();

        Ok((body.stack_frames, body.total_frames))
    }

    pub async fn threads(&mut self) -> Result<Vec<Thread>> {
        let response = self.request("threads".to_owned(), None).await?;

        let body: requests::ThreadsResponse = from_value(response.body.unwrap()).unwrap();

        Ok(body.threads)
    }

    pub async fn scopes(&mut self, frame_id: usize) -> Result<Vec<Scope>> {
        let args = requests::ScopesArguments { frame_id };

        let response = self
            .request("scopes".to_owned(), to_value(args).ok())
            .await?;

        let body: requests::ScopesResponse = from_value(response.body.unwrap()).unwrap();

        Ok(body.scopes)
    }

    pub async fn variables(&mut self, variables_reference: usize) -> Result<Vec<Variable>> {
        let args = requests::VariablesArguments {
            variables_reference,
            filter: None,
            start: None,
            count: None,
            format: None,
        };

        let response = self
            .request("variables".to_owned(), to_value(args).ok())
            .await?;

        let body: requests::VariablesResponse = from_value(response.body.unwrap()).unwrap();

        Ok(body.variables)
    }
}
