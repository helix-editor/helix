use crate::{
    transport::{Payload, Request, Transport},
    types::*,
    Error, Result,
};
use anyhow::anyhow;
pub use log::{error, info};
use std::{
    collections::HashMap,
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
    server_tx: UnboundedSender<Request>,
    request_counter: AtomicU64,
    pub caps: Option<DebuggerCapabilities>,
    // thread_id -> frames
    pub stack_frames: HashMap<isize, Vec<StackFrame>>,
    pub thread_states: HashMap<isize, String>,
    pub thread_id: Option<isize>,
    /// Currently active frame for the current thread.
    pub active_frame: Option<usize>,
    pub breakpoints: Vec<Breakpoint>,
}

impl Client {
    // Spawn a process and communicate with it by either TCP or stdio
    pub async fn process(
        transport: String,
        command: String,
        args: Vec<String>,
        port_arg: Option<String>,
        id: usize,
    ) -> Result<(Self, UnboundedReceiver<Payload>)> {
        if transport == "tcp" && port_arg.is_some() {
            Self::tcp_process(
                &command,
                args.iter().map(|s| s.as_str()).collect(),
                &port_arg.unwrap(),
                id,
            )
            .await
        } else if transport == "stdio" {
            Self::stdio(&command, args.iter().map(|s| s.as_str()).collect(), id)
        } else {
            Result::Err(Error::Other(anyhow!("Incorrect transport {}", transport)))
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
        let (client_rx, client_tx) = unbounded_channel();

        let client = Self {
            id,
            _process: process,
            server_tx,
            request_counter: AtomicU64::new(0),
            caps: None,
            //
            stack_frames: HashMap::new(),
            thread_states: HashMap::new(),
            thread_id: None,
            active_frame: None,
            breakpoints: vec![],
        };

        tokio::spawn(Self::recv(server_rx, client_rx));

        Ok((client, client_tx))
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
        let errors = BufReader::new(process.stderr.take().expect("Failed to open stderr"));

        Self::streams(
            Box::new(BufReader::new(reader)),
            Box::new(writer),
            Some(Box::new(BufReader::new(errors))),
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

    fn next_request_id(&self) -> u64 {
        self.request_counter.fetch_add(1, Ordering::Relaxed)
    }

    async fn request<R: crate::types::Request>(
        &self,
        arguments: R::Arguments,
    ) -> Result<R::Result> {
        let (callback_tx, mut callback_rx) = channel(1);

        let arguments = Some(serde_json::to_value(arguments)?);

        let req = Request {
            back_ch: Some(callback_tx),
            seq: self.next_request_id(),
            command: R::COMMAND.to_string(),
            arguments,
        };

        self.server_tx
            .send(req)
            .expect("Failed to send request to debugger");

        let response = callback_rx.recv().await.unwrap()?;
        let response = serde_json::from_value(response.body.unwrap_or_default())?;
        Ok(response)
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
            supports_run_in_terminal_request: Some(false),
            supports_memory_references: Some(false),
            supports_progress_reporting: Some(false),
            supports_invalidated_event: Some(false),
        };

        let response = self.request::<requests::Initialize>(args).await?;
        self.caps = Some(response);

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        self.request::<requests::Disconnect>(()).await
    }

    pub async fn launch(&mut self, args: serde_json::Value) -> Result<()> {
        let response = self.request::<requests::Launch>(args).await?;
        log::error!("launch response {}", response);

        Ok(())
    }

    pub async fn attach(&mut self, args: serde_json::Value) -> Result<()> {
        let response = self.request::<requests::Attach>(args).await?;
        log::error!("attach response {}", response);

        Ok(())
    }

    pub async fn set_breakpoints(
        &mut self,
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

    pub async fn configuration_done(&mut self) -> Result<()> {
        self.request::<requests::ConfigurationDone>(()).await
    }

    pub async fn continue_thread(&mut self, thread_id: isize) -> Result<Option<bool>> {
        let args = requests::ContinueArguments { thread_id };

        let response = self.request::<requests::Continue>(args).await?;
        Ok(response.all_threads_continued)
    }

    pub async fn stack_trace(
        &mut self,
        thread_id: isize,
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

    pub async fn threads(&mut self) -> Result<Vec<Thread>> {
        let response = self.request::<requests::Threads>(()).await?;
        Ok(response.threads)
    }

    pub async fn scopes(&mut self, frame_id: usize) -> Result<Vec<Scope>> {
        let args = requests::ScopesArguments { frame_id };

        let response = self.request::<requests::Scopes>(args).await?;
        Ok(response.scopes)
    }

    pub async fn variables(&mut self, variables_reference: usize) -> Result<Vec<Variable>> {
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

    pub async fn step_in(&mut self, thread_id: isize) -> Result<()> {
        let args = requests::StepInArguments {
            thread_id,
            target_id: None,
            granularity: None,
        };

        self.request::<requests::StepIn>(args).await
    }

    pub async fn step_out(&mut self, thread_id: isize) -> Result<()> {
        let args = requests::StepOutArguments {
            thread_id,
            granularity: None,
        };

        self.request::<requests::StepOut>(args).await
    }

    pub async fn next(&mut self, thread_id: isize) -> Result<()> {
        let args = requests::NextArguments {
            thread_id,
            granularity: None,
        };

        self.request::<requests::Next>(args).await
    }

    pub async fn pause(&mut self, thread_id: isize) -> Result<()> {
        let args = requests::PauseArguments { thread_id };

        self.request::<requests::Pause>(args).await
    }

    pub async fn eval(
        &mut self,
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
}
