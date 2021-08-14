use crate::{
    transport::{Event, Payload, Request, Response, Transport},
    Result,
};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value, Value};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::{collections::HashMap, process::Stdio};
use tokio::{
    io::{AsyncBufRead, AsyncWrite, BufReader, BufWriter},
    net::TcpStream,
    process::{Child, Command},
    sync::{
        mpsc::{channel, Receiver, Sender, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebuggerCapabilities {
    pub supports_configuration_done_request: Option<bool>,
    pub supports_function_breakpoints: Option<bool>,
    pub supports_conditional_breakpoints: Option<bool>,
    pub supports_exception_info_request: Option<bool>,
    pub support_terminate_debuggee: Option<bool>,
    pub supports_delayed_stack_trace_loading: Option<bool>,
    // TODO: complete this
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitializeArguments {
    client_id: String,
    client_name: String,
    adapter_id: String,
    locale: String,
    #[serde(rename = "linesStartAt1")]
    lines_start_at_one: bool,
    #[serde(rename = "columnsStartAt1")]
    columns_start_at_one: bool,
    path_format: String,
    supports_variable_type: bool,
    supports_variable_paging: bool,
    supports_run_in_terminal_request: bool,
    supports_memory_references: bool,
    supports_progress_reporting: bool,
    supports_invalidated_event: bool,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checksum {
    pub algorithm: String,
    pub checksum: String,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub name: Option<String>,
    pub path: Option<String>,
    pub source_reference: Option<usize>,
    pub presentation_hint: Option<String>,
    pub origin: Option<String>,
    pub sources: Option<Vec<Source>>,
    pub adapter_data: Option<Value>,
    pub checksums: Option<Vec<Checksum>>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakpoint {
    pub line: usize,
    pub column: Option<usize>,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub log_message: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetBreakpointsArguments {
    source: Source,
    breakpoints: Option<Vec<SourceBreakpoint>>,
    // lines is deprecated
    source_modified: Option<bool>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Breakpoint {
    pub id: Option<usize>,
    pub verified: bool,
    pub message: Option<String>,
    pub source: Option<Source>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
    pub instruction_reference: Option<String>,
    pub offset: Option<usize>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetBreakpointsResponseBody {
    breakpoints: Option<Vec<Breakpoint>>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ContinueArguments {
    thread_id: usize,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ContinueResponseBody {
    all_threads_continued: Option<bool>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StackFrameFormat {
    parameters: Option<bool>,
    parameter_types: Option<bool>,
    parameter_names: Option<bool>,
    parameter_values: Option<bool>,
    line: Option<bool>,
    module: Option<bool>,
    include_all: Option<bool>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StackTraceArguments {
    thread_id: usize,
    start_frame: Option<usize>,
    levels: Option<usize>,
    format: Option<StackFrameFormat>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
    pub id: usize,
    pub name: String,
    pub source: Option<Source>,
    pub line: usize,
    pub column: usize,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
    pub can_restart: Option<bool>,
    pub instruction_pointer_reference: Option<String>,
    pub module_id: Option<Value>,
    pub presentation_hint: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StackTraceResponseBody {
    total_frames: Option<usize>,
    stack_frames: Vec<StackFrame>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: usize,
    pub name: String,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThreadsResponseBody {
    threads: Vec<Thread>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScopesArguments {
    frame_id: usize,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub name: String,
    pub presentation_hint: Option<String>,
    pub variables_reference: usize,
    pub named_variables: Option<usize>,
    pub indexed_variables: Option<usize>,
    pub expensive: bool,
    pub source: Option<Source>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScopesResponseBody {
    scopes: Vec<Scope>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ValueFormat {
    hex: Option<bool>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct VariablesArguments {
    variables_reference: usize,
    filter: Option<String>,
    start: Option<usize>,
    count: Option<usize>,
    format: Option<ValueFormat>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablePresentationHint {
    pub kind: Option<String>,
    pub attributes: Option<Vec<String>>,
    pub visibility: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    pub value: String,
    #[serde(rename = "type")]
    pub data_type: Option<String>,
    pub presentation_hint: Option<VariablePresentationHint>,
    pub evaluate_name: Option<String>,
    pub variables_reference: usize,
    pub named_variables: Option<usize>,
    pub indexed_variables: Option<usize>,
    pub memory_reference: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct VariablesResponseBody {
    variables: Vec<Variable>,
}

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

    async fn recv(
        awaited_events: Arc<Mutex<HashMap<String, Sender<Event>>>>,
        mut server_rx: UnboundedReceiver<Payload>,
    ) {
        while let Some(msg) = server_rx.recv().await {
            match msg {
                Payload::Event(ev) => {
                    let name = ev.event.clone();
                    let tx = awaited_events.lock().await.remove(&name);

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
            msg_type: "request".to_owned(),
            command,
            arguments,
        };

        self.server_tx
            .send(req)
            .expect("Failed to send request to debugger");

        callback_tx
            .recv()
            .await
            .expect("Failed to receive response")
    }

    pub fn capabilities(&self) -> &DebuggerCapabilities {
        self.capabilities
            .as_ref()
            .expect("language server not yet initialized!")
    }

    pub async fn initialize(&mut self, adapter_id: String) -> Result<()> {
        let args = InitializeArguments {
            client_id: "hx".to_owned(),
            client_name: "helix".to_owned(),
            adapter_id,
            locale: "en-us".to_owned(),
            lines_start_at_one: true,
            columns_start_at_one: true,
            path_format: "path".to_owned(),
            supports_variable_type: false,
            supports_variable_paging: false,
            supports_run_in_terminal_request: false,
            supports_memory_references: false,
            supports_progress_reporting: true,
            supports_invalidated_event: true,
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

        self.request("launch".to_owned(), to_value(args).ok())
            .await?;

        initialized.recv().await;

        Ok(())
    }

    pub async fn attach(&mut self, args: impl Serialize) -> Result<()> {
        let mut initialized = self.listen_for_event("initialized".to_owned()).await;

        self.request("attach".to_owned(), to_value(args).ok())
            .await?;

        initialized.recv().await;

        Ok(())
    }

    pub async fn set_breakpoints(
        &mut self,
        file: String,
        breakpoints: Vec<SourceBreakpoint>,
    ) -> Result<Option<Vec<Breakpoint>>> {
        let args = SetBreakpointsArguments {
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
        let body: Option<SetBreakpointsResponseBody> = from_value(response.body.unwrap()).ok();

        Ok(body.map(|b| b.breakpoints).unwrap())
    }

    pub async fn configuration_done(&mut self) -> Result<()> {
        self.request("configurationDone".to_owned(), None).await?;
        Ok(())
    }

    pub async fn continue_thread(&mut self, thread_id: usize) -> Result<Option<bool>> {
        let args = ContinueArguments { thread_id };

        let response = self
            .request("continue".to_owned(), to_value(args).ok())
            .await?;

        let body: Option<ContinueResponseBody> = from_value(response.body.unwrap()).ok();

        Ok(body.map(|b| b.all_threads_continued).unwrap())
    }

    pub async fn stack_trace(
        &mut self,
        thread_id: usize,
    ) -> Result<(Vec<StackFrame>, Option<usize>)> {
        let args = StackTraceArguments {
            thread_id,
            start_frame: None,
            levels: None,
            format: None,
        };

        let response = self
            .request("stackTrace".to_owned(), to_value(args).ok())
            .await?;

        let body: StackTraceResponseBody = from_value(response.body.unwrap()).unwrap();

        Ok((body.stack_frames, body.total_frames))
    }

    pub async fn threads(&mut self) -> Result<Vec<Thread>> {
        let response = self.request("threads".to_owned(), None).await?;

        let body: ThreadsResponseBody = from_value(response.body.unwrap()).unwrap();

        Ok(body.threads)
    }

    pub async fn scopes(&mut self, frame_id: usize) -> Result<Vec<Scope>> {
        let args = ScopesArguments { frame_id };

        let response = self
            .request("scopes".to_owned(), to_value(args).ok())
            .await?;

        let body: ScopesResponseBody = from_value(response.body.unwrap()).unwrap();

        Ok(body.scopes)
    }

    pub async fn variables(&mut self, variables_reference: usize) -> Result<Vec<Variable>> {
        let args = VariablesArguments {
            variables_reference,
            filter: None,
            start: None,
            count: None,
            format: None,
        };

        let response = self
            .request("variables".to_owned(), to_value(args).ok())
            .await?;

        let body: VariablesResponseBody = from_value(response.body.unwrap()).unwrap();

        Ok(body.variables)
    }
}
