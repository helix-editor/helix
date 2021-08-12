use crate::{
    transport::{Event, Payload, Request, Response, Transport},
    Result,
};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value, Value};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::{
    io::{BufReader, BufWriter},
    process::{Child, Command},
    sync::mpsc::{channel, UnboundedReceiver, UnboundedSender},
};

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebuggerCapabilities {
    supports_configuration_done_request: bool,
    supports_function_breakpoints: bool,
    supports_conditional_breakpoints: bool,
    supports_exception_info_request: bool,
    support_terminate_debuggee: bool,
    supports_delayed_stack_trace_loading: bool,
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

// TODO: split out
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchArguments {
    mode: String,
    program: String,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    path: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakpoint {
    pub line: usize,
    pub column: Option<usize>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetBreakpointsArguments {
    source: Source,
    breakpoints: Option<Vec<SourceBreakpoint>>,
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
    id: usize,
    name: String,
    source: Option<Source>,
    line: usize,
    column: usize,
    end_line: Option<usize>,
    end_column: Option<usize>,
    can_restart: Option<bool>,
    instruction_pointer_reference: Option<String>,
    // module_id
    presentation_hint: Option<String>,
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
    id: usize,
    name: String,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThreadsResponseBody {
    threads: Vec<Thread>,
}

#[derive(Debug)]
pub struct Client {
    id: usize,
    _process: Child,
    server_tx: UnboundedSender<Request>,
    server_rx: UnboundedReceiver<Payload>,
    request_counter: AtomicU64,
    capabilities: Option<DebuggerCapabilities>,
}

impl Client {
    pub fn start(cmd: &str, args: Vec<&str>, id: usize) -> Result<Self> {
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

        let (server_rx, server_tx) = Transport::start(reader, writer, id);

        let client = Self {
            id,
            _process: process,
            server_tx,
            server_rx,
            request_counter: AtomicU64::new(0),
            capabilities: None,
        };

        // TODO: async client.initialize()
        // maybe use an arc<atomic> flag

        Ok(client)
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

    pub async fn initialize(&mut self) -> Result<()> {
        let args = InitializeArguments {
            client_id: "hx".to_owned(),
            client_name: "helix".to_owned(),
            adapter_id: "go".to_owned(),
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

    pub async fn launch(&mut self, executable: String) -> Result<()> {
        let args = LaunchArguments {
            mode: "exec".to_owned(),
            program: executable,
        };

        self.request("launch".to_owned(), to_value(args).ok())
            .await?;

        match self
            .server_rx
            .recv()
            .await
            .expect("Expected initialized event")
        {
            Payload::Event(Event { event, .. }) => {
                if event == *"initialized" {
                    Ok(())
                } else {
                    unreachable!()
                }
            }
            _ => unreachable!(),
        }
    }

    pub async fn set_breakpoints(
        &mut self,
        file: String,
        breakpoints: Vec<SourceBreakpoint>,
    ) -> Result<Option<Vec<Breakpoint>>> {
        let args = SetBreakpointsArguments {
            source: Source { path: Some(file) },
            breakpoints: Some(breakpoints),
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

    pub async fn wait_for_stopped(&mut self) -> Result<()> {
        match self.server_rx.recv().await.expect("Expected stopped event") {
            Payload::Event(Event { event, .. }) => {
                if event == *"stopped" {
                    Ok(())
                } else {
                    unreachable!()
                }
            }
            _ => unreachable!(),
        }
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
}
