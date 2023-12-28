use crate::{
    requests::DisconnectArguments,
    transport::{Payload, Request, Response, Transport},
    types::*,
    Error, Result, ThreadId,
};
use helix_core::syntax::DebuggerQuirks;

use serde_json::Value;

use anyhow::anyhow;
use std::{
    collections::HashMap,
    future::Future,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    process::Stdio,
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::{
    io::{AsyncBufRead, AsyncWrite, BufReader, BufWriter},
    net::TcpStream,
    process::{Child, Command},
    sync::mpsc::{channel, unbounded_channel, UnboundedReceiver, UnboundedSender},
    time,
};

#[derive(Debug)]
pub struct Client {
    id: usize,
    _process: Option<Child>,
    server_tx: UnboundedSender<Payload>,
    request_counter: AtomicU64,
    connection_type: Option<ConnectionType>,
    starting_request_args: Option<Value>,
    pub caps: Option<DebuggerCapabilities>,
    // thread_id -> frames
    pub stack_frames: HashMap<ThreadId, Vec<StackFrame>>,
    pub thread_states: ThreadStates,
    pub thread_id: Option<ThreadId>,
    /// Currently active frame for the current thread.
    pub active_frame: Option<usize>,
    pub quirks: DebuggerQuirks,
}

#[derive(Clone, Copy, Debug)]
pub enum ConnectionType {
    Launch,
    Attach,
}

impl Client {
    // Spawn a process and communicate with it by either TCP or stdio
    pub async fn process(
        transport: &str,
        command: &str,
        args: Vec<&str>,
        port_arg: Option<&str>,
        id: usize,
    ) -> Result<(Self, UnboundedReceiver<Payload>)> {
        if command.is_empty() {
            return Result::Err(Error::Other(anyhow!("Command not provided")));
        }
        match (transport, port_arg) {
            ("tcp", Some(port_arg)) => Self::tcp_process(command, args, port_arg, id).await,
            ("stdio", _) => Self::stdio(command, args, id),
            _ => Result::Err(Error::Other(anyhow!("Incorrect transport {}", transport))),
        }
    }

    pub fn streams(
        rx: Box<dyn AsyncBufRead + Unpin + Send>,
        tx: Box<dyn AsyncWrite + Unpin + Send>,
        err: Option<Box<dyn AsyncBufRead + Unpin + Send>>,
        id: usize,
        process: Option<Child>,
    ) -> Result<(Self, UnboundedReceiver<Payload>)> {
        let (server_rx, server_tx) = Transport::start(rx, tx, err, id);
        let (client_tx, client_rx) = unbounded_channel();

        let client = Self {
            id,
            _process: process,
            server_tx,
            request_counter: AtomicU64::new(0),
            caps: None,
            connection_type: None,
            starting_request_args: None,
            stack_frames: HashMap::new(),
            thread_states: HashMap::new(),
            thread_id: None,
            active_frame: None,
            quirks: DebuggerQuirks::default(),
        };

        tokio::spawn(Self::recv(server_rx, client_tx));

        Ok((client, client_rx))
    }

    pub async fn tcp(
        addr: std::net::SocketAddr,
        id: usize,
    ) -> Result<(Self, UnboundedReceiver<Payload>)> {
        let stream = TcpStream::connect(addr).await?;
        let (rx, tx) = stream.into_split();
        Self::streams(Box::new(BufReader::new(rx)), Box::new(tx), None, id, None)
    }

    pub fn stdio(
        cmd: &str,
        args: Vec<&str>,
        id: usize,
    ) -> Result<(Self, UnboundedReceiver<Payload>)> {
        // Resolve path to the binary
        let cmd = which::which(cmd).map_err(|err| anyhow::anyhow!(err))?;

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
        let errors = process.stderr.take().map(BufReader::new);

        Self::streams(
            Box::new(BufReader::new(reader)),
            Box::new(writer),
            // errors.map(|errors| Box::new(BufReader::new(errors))),
            match errors {
                Some(errors) => Some(Box::new(BufReader::new(errors))),
                None => None,
            },
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

    pub fn starting_request_args(&self) -> &Option<Value> {
        &self.starting_request_args
    }

    pub async fn tcp_process(
        cmd: &str,
        args: Vec<&str>,
        port_format: &str,
        id: usize,
    ) -> Result<(Self, UnboundedReceiver<Payload>)> {
        let port = Self::get_port().await.unwrap();

        let process = Command::new(cmd)
            .args(args)
            .args(port_format.replace("{}", &port.to_string()).split(' '))
            // silence messages
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            // Do not kill debug adapter when leaving, it should exit automatically
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
            None,
            id,
            Some(process),
        )
    }

    async fn recv(mut server_rx: UnboundedReceiver<Payload>, client_tx: UnboundedSender<Payload>) {
        while let Some(msg) = server_rx.recv().await {
            match msg {
                Payload::Event(ev) => {
                    client_tx.send(Payload::Event(ev)).expect("Failed to send");
                }
                Payload::Response(_) => unreachable!(),
                Payload::Request(req) => {
                    client_tx
                        .send(Payload::Request(req))
                        .expect("Failed to send");
                }
            }
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn connection_type(&self) -> Option<ConnectionType> {
        self.connection_type
    }

    fn next_request_id(&self) -> u64 {
        self.request_counter.fetch_add(1, Ordering::Relaxed)
    }

    // Internal, called by specific DAP commands when resuming
    pub fn resume_application(&mut self) {
        if let Some(thread_id) = self.thread_id {
            self.thread_states.insert(thread_id, "running".to_string());
            self.stack_frames.remove(&thread_id);
        }
        self.active_frame = None;
        self.thread_id = None;
    }

    /// Execute a RPC request on the debugger.
    pub fn call<R: crate::types::Request>(
        &self,
        arguments: R::Arguments,
    ) -> impl Future<Output = Result<Value>>
    where
        R::Arguments: serde::Serialize,
    {
        let server_tx = self.server_tx.clone();
        let id = self.next_request_id();

        async move {
            use std::time::Duration;
            use tokio::time::timeout;

            let arguments = Some(serde_json::to_value(arguments)?);

            let (callback_tx, mut callback_rx) = channel(1);

            let req = Request {
                back_ch: Some(callback_tx),
                seq: id,
                command: R::COMMAND.to_string(),
                arguments,
            };

            server_tx
                .send(Payload::Request(req))
                .map_err(|e| Error::Other(e.into()))?;

            // TODO: specifiable timeout, delay other calls until initialize success
            timeout(Duration::from_secs(20), callback_rx.recv())
                .await
                .map_err(|_| Error::Timeout(id))? // return Timeout
                .ok_or(Error::StreamClosed)?
                .map(|response| response.body.unwrap_or_default())
            // TODO: check response.success
        }
    }

    pub async fn request<R: crate::types::Request>(&self, params: R::Arguments) -> Result<R::Result>
    where
        R::Arguments: serde::Serialize,
        R::Result: core::fmt::Debug, // TODO: temporary
    {
        // a future that resolves into the response
        let json = self.call::<R>(params).await?;
        let response = serde_json::from_value(json)?;
        Ok(response)
    }

    pub fn reply(
        &self,
        request_seq: u64,
        command: &str,
        result: core::result::Result<Value, Error>,
    ) -> impl Future<Output = Result<()>> {
        let server_tx = self.server_tx.clone();
        let command = command.to_string();

        async move {
            let response = match result {
                Ok(result) => Response {
                    request_seq,
                    command,
                    success: true,
                    message: None,
                    body: Some(result),
                },
                Err(error) => Response {
                    request_seq,
                    command,
                    success: false,
                    message: Some(error.to_string()),
                    body: None,
                },
            };

            server_tx
                .send(Payload::Response(response))
                .map_err(|e| Error::Other(e.into()))?;

            Ok(())
        }
    }

    pub fn capabilities(&self) -> &DebuggerCapabilities {
        self.caps.as_ref().expect("debugger not yet initialized!")
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
            supports_variable_type: Some(true),
            supports_variable_paging: Some(false),
            supports_run_in_terminal_request: Some(true),
            supports_memory_references: Some(false),
            supports_progress_reporting: Some(false),
            supports_invalidated_event: Some(false),
        };

        let response = self.request::<requests::Initialize>(args).await?;
        self.caps = Some(response);

        Ok(())
    }

    pub fn disconnect(
        &mut self,
        args: Option<DisconnectArguments>,
    ) -> impl Future<Output = Result<Value>> {
        self.connection_type = None;
        self.call::<requests::Disconnect>(args)
    }

    pub fn launch(&mut self, args: serde_json::Value) -> impl Future<Output = Result<Value>> {
        self.connection_type = Some(ConnectionType::Launch);
        self.starting_request_args = Some(args.clone());
        self.call::<requests::Launch>(args)
    }

    pub fn attach(&mut self, args: serde_json::Value) -> impl Future<Output = Result<Value>> {
        self.connection_type = Some(ConnectionType::Attach);
        self.starting_request_args = Some(args.clone());
        self.call::<requests::Attach>(args)
    }

    pub fn restart(&self) -> impl Future<Output = Result<Value>> {
        let args = if let Some(args) = &self.starting_request_args {
            args.clone()
        } else {
            Value::Null
        };
        self.call::<requests::Restart>(args)
    }

    pub async fn set_breakpoints(
        &self,
        file: PathBuf,
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

        let response = self.request::<requests::SetBreakpoints>(args).await?;

        Ok(response.breakpoints)
    }

    pub async fn configuration_done(&self) -> Result<()> {
        self.request::<requests::ConfigurationDone>(()).await
    }

    pub fn continue_thread(&self, thread_id: ThreadId) -> impl Future<Output = Result<Value>> {
        let args = requests::ContinueArguments { thread_id };

        self.call::<requests::Continue>(args)
    }

    pub async fn stack_trace(
        &self,
        thread_id: ThreadId,
    ) -> Result<(Vec<StackFrame>, Option<usize>)> {
        let args = requests::StackTraceArguments {
            thread_id,
            start_frame: None,
            levels: None,
            format: None,
        };

        let response = self.request::<requests::StackTrace>(args).await?;
        Ok((response.stack_frames, response.total_frames))
    }

    pub fn threads(&self) -> impl Future<Output = Result<Value>> {
        self.call::<requests::Threads>(())
    }

    pub async fn scopes(&self, frame_id: usize) -> Result<Vec<Scope>> {
        let args = requests::ScopesArguments { frame_id };

        let response = self.request::<requests::Scopes>(args).await?;
        Ok(response.scopes)
    }

    pub async fn variables(&self, variables_reference: usize) -> Result<Vec<Variable>> {
        let args = requests::VariablesArguments {
            variables_reference,
            filter: None,
            start: None,
            count: None,
            format: None,
        };

        let response = self.request::<requests::Variables>(args).await?;
        Ok(response.variables)
    }

    pub fn step_in(&self, thread_id: ThreadId) -> impl Future<Output = Result<Value>> {
        let args = requests::StepInArguments {
            thread_id,
            target_id: None,
            granularity: None,
        };

        self.call::<requests::StepIn>(args)
    }

    pub fn step_out(&self, thread_id: ThreadId) -> impl Future<Output = Result<Value>> {
        let args = requests::StepOutArguments {
            thread_id,
            granularity: None,
        };

        self.call::<requests::StepOut>(args)
    }

    pub fn next(&self, thread_id: ThreadId) -> impl Future<Output = Result<Value>> {
        let args = requests::NextArguments {
            thread_id,
            granularity: None,
        };

        self.call::<requests::Next>(args)
    }

    pub fn pause(&self, thread_id: ThreadId) -> impl Future<Output = Result<Value>> {
        let args = requests::PauseArguments { thread_id };

        self.call::<requests::Pause>(args)
    }

    pub async fn eval(
        &self,
        expression: String,
        frame_id: Option<usize>,
    ) -> Result<requests::EvaluateResponse> {
        let args = requests::EvaluateArguments {
            expression,
            frame_id,
            context: None,
            format: None,
        };

        self.request::<requests::Evaluate>(args).await
    }

    pub fn set_exception_breakpoints(
        &self,
        filters: Vec<String>,
    ) -> impl Future<Output = Result<Value>> {
        let args = requests::SetExceptionBreakpointsArguments { filters };

        self.call::<requests::SetExceptionBreakpoints>(args)
    }

    pub fn current_stack_frame(&self) -> Option<&StackFrame> {
        self.stack_frames
            .get(&self.thread_id?)?
            .get(self.active_frame?)
    }
}
