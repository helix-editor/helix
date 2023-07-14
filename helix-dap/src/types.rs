use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize,
)]
pub struct ThreadId(isize);

impl std::fmt::Display for ThreadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub type ThreadStates = HashMap<ThreadId, String>;

pub trait Request {
    type Arguments: serde::de::DeserializeOwned + serde::Serialize;
    type Result: serde::de::DeserializeOwned + serde::Serialize;
    const COMMAND: &'static str;
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnDescriptor {
    pub attribute_name: String,
    pub label: String,
    pub format: Option<String>,
    #[serde(rename = "type")]
    pub ty: Option<String>,
    pub width: Option<usize>,
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionBreakpointsFilter {
    pub filter: String,
    pub label: String,
    pub description: Option<String>,
    pub default: Option<bool>,
    pub supports_condition: Option<bool>,
    pub condition_description: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebuggerCapabilities {
    pub supports_configuration_done_request: Option<bool>,
    pub supports_function_breakpoints: Option<bool>,
    pub supports_conditional_breakpoints: Option<bool>,
    pub supports_hit_conditional_breakpoints: Option<bool>,
    pub supports_evaluate_for_hovers: Option<bool>,
    pub supports_step_back: Option<bool>,
    pub supports_set_variable: Option<bool>,
    pub supports_restart_frame: Option<bool>,
    pub supports_goto_targets_request: Option<bool>,
    pub supports_step_in_targets_request: Option<bool>,
    pub supports_completions_request: Option<bool>,
    pub supports_modules_request: Option<bool>,
    pub supports_restart_request: Option<bool>,
    pub supports_exception_options: Option<bool>,
    pub supports_value_formatting_options: Option<bool>,
    pub supports_exception_info_request: Option<bool>,
    pub support_terminate_debuggee: Option<bool>,
    pub support_suspend_debuggee: Option<bool>,
    pub supports_delayed_stack_trace_loading: Option<bool>,
    pub supports_loaded_sources_request: Option<bool>,
    pub supports_log_points: Option<bool>,
    pub supports_terminate_threads_request: Option<bool>,
    pub supports_set_expression: Option<bool>,
    pub supports_terminate_request: Option<bool>,
    pub supports_data_breakpoints: Option<bool>,
    pub supports_read_memory_request: Option<bool>,
    pub supports_write_memory_request: Option<bool>,
    pub supports_disassemble_request: Option<bool>,
    pub supports_cancel_request: Option<bool>,
    pub supports_breakpoint_locations_request: Option<bool>,
    pub supports_clipboard_context: Option<bool>,
    pub supports_stepping_granularity: Option<bool>,
    pub supports_instruction_breakpoints: Option<bool>,
    pub supports_exception_filter_options: Option<bool>,
    pub exception_breakpoint_filters: Option<Vec<ExceptionBreakpointsFilter>>,
    pub completion_trigger_characters: Option<Vec<String>>,
    pub additional_module_columns: Option<Vec<ColumnDescriptor>>,
    pub supported_checksum_algorithms: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checksum {
    pub algorithm: String,
    pub checksum: String,
}

#[skip_serializing_none]
#[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub name: Option<String>,
    pub path: Option<PathBuf>,
    pub source_reference: Option<usize>,
    pub presentation_hint: Option<String>,
    pub origin: Option<String>,
    pub sources: Option<Vec<Source>>,
    pub adapter_data: Option<Value>,
    pub checksums: Option<Vec<Checksum>>,
}

#[skip_serializing_none]
#[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakpoint {
    pub line: usize,
    pub column: Option<usize>,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub log_message: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
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

#[skip_serializing_none]
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrameFormat {
    pub parameters: Option<bool>,
    pub parameter_types: Option<bool>,
    pub parameter_names: Option<bool>,
    pub parameter_values: Option<bool>,
    pub line: Option<bool>,
    pub module: Option<bool>,
    pub include_all: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
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

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: ThreadId,
    pub name: String,
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
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

#[skip_serializing_none]
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueFormat {
    pub hex: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablePresentationHint {
    pub kind: Option<String>,
    pub attributes: Option<Vec<String>>,
    pub visibility: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    pub value: String,
    #[serde(rename = "type")]
    pub ty: Option<String>,
    pub presentation_hint: Option<VariablePresentationHint>,
    pub evaluate_name: Option<String>,
    pub variables_reference: usize,
    pub named_variables: Option<usize>,
    pub indexed_variables: Option<usize>,
    pub memory_reference: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Module {
    pub id: String, // TODO: || number
    pub name: String,
    pub path: Option<PathBuf>,
    pub is_optimized: Option<bool>,
    pub is_user_code: Option<bool>,
    pub version: Option<String>,
    pub symbol_status: Option<String>,
    pub symbol_file_path: Option<String>,
    pub date_time_stamp: Option<String>,
    pub address_range: Option<String>,
}

pub mod requests {
    use super::*;
    #[skip_serializing_none]
    #[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct InitializeArguments {
        #[serde(rename = "clientID")]
        pub client_id: Option<String>,
        pub client_name: Option<String>,
        #[serde(rename = "adapterID")]
        pub adapter_id: String,
        pub locale: Option<String>,
        #[serde(rename = "linesStartAt1")]
        pub lines_start_at_one: Option<bool>,
        #[serde(rename = "columnsStartAt1")]
        pub columns_start_at_one: Option<bool>,
        pub path_format: Option<String>,
        pub supports_variable_type: Option<bool>,
        pub supports_variable_paging: Option<bool>,
        pub supports_run_in_terminal_request: Option<bool>,
        pub supports_memory_references: Option<bool>,
        pub supports_progress_reporting: Option<bool>,
        pub supports_invalidated_event: Option<bool>,
    }

    #[derive(Debug)]
    pub enum Initialize {}

    impl Request for Initialize {
        type Arguments = InitializeArguments;
        type Result = DebuggerCapabilities;
        const COMMAND: &'static str = "initialize";
    }

    #[derive(Debug)]
    pub enum Launch {}

    impl Request for Launch {
        type Arguments = Value;
        type Result = ();
        const COMMAND: &'static str = "launch";
    }

    #[derive(Debug)]
    pub enum Attach {}

    impl Request for Attach {
        type Arguments = Value;
        type Result = ();
        const COMMAND: &'static str = "attach";
    }

    #[skip_serializing_none]
    #[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DisconnectArguments {
        pub restart: Option<bool>,
        pub terminate_debuggee: Option<bool>,
        pub suspend_debuggee: Option<bool>,
    }

    #[derive(Debug)]
    pub enum Restart {}

    impl Request for Restart {
        type Arguments = Value;
        type Result = ();
        const COMMAND: &'static str = "restart";
    }

    #[derive(Debug)]
    pub enum Disconnect {}

    impl Request for Disconnect {
        type Arguments = Option<DisconnectArguments>;
        type Result = ();
        const COMMAND: &'static str = "disconnect";
    }

    #[derive(Debug)]
    pub enum ConfigurationDone {}

    impl Request for ConfigurationDone {
        type Arguments = ();
        type Result = ();
        const COMMAND: &'static str = "configurationDone";
    }

    #[skip_serializing_none]
    #[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SetBreakpointsArguments {
        pub source: Source,
        pub breakpoints: Option<Vec<SourceBreakpoint>>,
        // lines is deprecated
        pub source_modified: Option<bool>,
    }

    #[skip_serializing_none]
    #[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SetBreakpointsResponse {
        pub breakpoints: Option<Vec<Breakpoint>>,
    }

    #[derive(Debug)]
    pub enum SetBreakpoints {}

    impl Request for SetBreakpoints {
        type Arguments = SetBreakpointsArguments;
        type Result = SetBreakpointsResponse;
        const COMMAND: &'static str = "setBreakpoints";
    }

    #[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ContinueArguments {
        pub thread_id: ThreadId,
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ContinueResponse {
        pub all_threads_continued: Option<bool>,
    }

    #[derive(Debug)]
    pub enum Continue {}

    impl Request for Continue {
        type Arguments = ContinueArguments;
        type Result = ContinueResponse;
        const COMMAND: &'static str = "continue";
    }

    #[skip_serializing_none]
    #[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StackTraceArguments {
        pub thread_id: ThreadId,
        pub start_frame: Option<usize>,
        pub levels: Option<usize>,
        pub format: Option<StackFrameFormat>,
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StackTraceResponse {
        pub total_frames: Option<usize>,
        pub stack_frames: Vec<StackFrame>,
    }

    #[derive(Debug)]
    pub enum StackTrace {}

    impl Request for StackTrace {
        type Arguments = StackTraceArguments;
        type Result = StackTraceResponse;
        const COMMAND: &'static str = "stackTrace";
    }

    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ThreadsResponse {
        pub threads: Vec<Thread>,
    }

    #[derive(Debug)]
    pub enum Threads {}

    impl Request for Threads {
        type Arguments = ();
        type Result = ThreadsResponse;
        const COMMAND: &'static str = "threads";
    }

    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ScopesArguments {
        pub frame_id: usize,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ScopesResponse {
        pub scopes: Vec<Scope>,
    }

    #[derive(Debug)]
    pub enum Scopes {}

    impl Request for Scopes {
        type Arguments = ScopesArguments;
        type Result = ScopesResponse;
        const COMMAND: &'static str = "scopes";
    }

    #[skip_serializing_none]
    #[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VariablesArguments {
        pub variables_reference: usize,
        pub filter: Option<String>,
        pub start: Option<usize>,
        pub count: Option<usize>,
        pub format: Option<ValueFormat>,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VariablesResponse {
        pub variables: Vec<Variable>,
    }

    #[derive(Debug)]
    pub enum Variables {}

    impl Request for Variables {
        type Arguments = VariablesArguments;
        type Result = VariablesResponse;
        const COMMAND: &'static str = "variables";
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StepInArguments {
        pub thread_id: ThreadId,
        pub target_id: Option<usize>,
        pub granularity: Option<String>,
    }

    #[derive(Debug)]
    pub enum StepIn {}

    impl Request for StepIn {
        type Arguments = StepInArguments;
        type Result = ();
        const COMMAND: &'static str = "stepIn";
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StepOutArguments {
        pub thread_id: ThreadId,
        pub granularity: Option<String>,
    }

    #[derive(Debug)]
    pub enum StepOut {}

    impl Request for StepOut {
        type Arguments = StepOutArguments;
        type Result = ();
        const COMMAND: &'static str = "stepOut";
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NextArguments {
        pub thread_id: ThreadId,
        pub granularity: Option<String>,
    }

    #[derive(Debug)]
    pub enum Next {}

    impl Request for Next {
        type Arguments = NextArguments;
        type Result = ();
        const COMMAND: &'static str = "next";
    }

    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PauseArguments {
        pub thread_id: ThreadId,
    }

    #[derive(Debug)]
    pub enum Pause {}

    impl Request for Pause {
        type Arguments = PauseArguments;
        type Result = ();
        const COMMAND: &'static str = "pause";
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EvaluateArguments {
        pub expression: String,
        pub frame_id: Option<usize>,
        pub context: Option<String>,
        pub format: Option<ValueFormat>,
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EvaluateResponse {
        pub result: String,
        #[serde(rename = "type")]
        pub ty: Option<String>,
        pub presentation_hint: Option<VariablePresentationHint>,
        pub variables_reference: usize,
        pub named_variables: Option<usize>,
        pub indexed_variables: Option<usize>,
        pub memory_reference: Option<String>,
    }

    #[derive(Debug)]
    pub enum Evaluate {}

    impl Request for Evaluate {
        type Arguments = EvaluateArguments;
        type Result = EvaluateResponse;
        const COMMAND: &'static str = "evaluate";
    }

    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SetExceptionBreakpointsArguments {
        pub filters: Vec<String>,
        // pub filterOptions: Option<Vec<ExceptionFilterOptions>>, // needs capability
        // pub exceptionOptions: Option<Vec<ExceptionOptions>>, // needs capability
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SetExceptionBreakpointsResponse {
        pub breakpoints: Option<Vec<Breakpoint>>,
    }

    #[derive(Debug)]
    pub enum SetExceptionBreakpoints {}

    impl Request for SetExceptionBreakpoints {
        type Arguments = SetExceptionBreakpointsArguments;
        type Result = SetExceptionBreakpointsResponse;
        const COMMAND: &'static str = "setExceptionBreakpoints";
    }

    // Reverse Requests

    #[skip_serializing_none]
    #[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RunInTerminalResponse {
        pub process_id: Option<u32>,
        pub shell_process_id: Option<u32>,
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RunInTerminalArguments {
        pub kind: Option<String>,
        pub title: Option<String>,
        pub cwd: String,
        pub args: Vec<String>,
        pub env: Option<HashMap<String, Option<String>>>,
    }

    #[derive(Debug)]
    pub enum RunInTerminal {}

    impl Request for RunInTerminal {
        type Arguments = RunInTerminalArguments;
        type Result = RunInTerminalResponse;
        const COMMAND: &'static str = "runInTerminal";
    }
}

// Events

pub mod events {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[serde(tag = "event", content = "body")]
    // seq is omitted as unused and is not sent by some implementations
    pub enum Event {
        Initialized(Option<DebuggerCapabilities>),
        Stopped(Stopped),
        Continued(Continued),
        Exited(Exited),
        Terminated(Option<Terminated>),
        Thread(Thread),
        Output(Output),
        Breakpoint(Breakpoint),
        Module(Module),
        LoadedSource(LoadedSource),
        Process(Process),
        Capabilities(Capabilities),
        // ProgressStart(),
        // ProgressUpdate(),
        // ProgressEnd(),
        // Invalidated(),
        Memory(Memory),
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Stopped {
        pub reason: String,
        pub description: Option<String>,
        pub thread_id: Option<ThreadId>,
        pub preserve_focus_hint: Option<bool>,
        pub text: Option<String>,
        pub all_threads_stopped: Option<bool>,
        pub hit_breakpoint_ids: Option<Vec<usize>>,
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Continued {
        pub thread_id: ThreadId,
        pub all_threads_continued: Option<bool>,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Exited {
        pub exit_code: usize,
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Terminated {
        pub restart: Option<Value>,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Thread {
        pub reason: String,
        pub thread_id: ThreadId,
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Output {
        pub output: String,
        pub category: Option<String>,
        pub group: Option<String>,
        pub line: Option<usize>,
        pub column: Option<usize>,
        pub variables_reference: Option<usize>,
        pub source: Option<Source>,
        pub data: Option<Value>,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Breakpoint {
        pub reason: String,
        pub breakpoint: super::Breakpoint,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Module {
        pub reason: String,
        pub module: super::Module,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LoadedSource {
        pub reason: String,
        pub source: super::Source,
    }

    #[skip_serializing_none]
    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Process {
        pub name: String,
        pub system_process_id: Option<usize>,
        pub is_local_process: Option<bool>,
        pub start_method: Option<String>, // TODO: use enum
        pub pointer_size: Option<usize>,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Capabilities {
        pub capabilities: super::DebuggerCapabilities,
    }

    // #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    // #[serde(rename_all = "camelCase")]
    // pub struct Invalidated {
    // pub areas: Vec<InvalidatedArea>,
    // pub thread_id: Option<ThreadId>,
    // pub stack_frame_id: Option<usize>,
    // }

    #[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Memory {
        pub memory_reference: String,
        pub offset: usize,
        pub count: usize,
    }
}
