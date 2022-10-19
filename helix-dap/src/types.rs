use serde::{Deserialize, Serialize};
use serde_json::Value;
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

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnDescriptor {
    pub attribute_name: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<usize>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionBreakpointsFilter {
    pub filter: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_condition: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_description: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebuggerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_configuration_done_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_function_breakpoints: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_conditional_breakpoints: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_hit_conditional_breakpoints: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_evaluate_for_hovers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_step_back: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_set_variable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_restart_frame: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_goto_targets_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_step_in_targets_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_completions_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_modules_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_restart_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_exception_options: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_value_formatting_options: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_exception_info_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_terminate_debuggee: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_suspend_debuggee: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_delayed_stack_trace_loading: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_loaded_sources_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_log_points: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_terminate_threads_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_set_expression: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_terminate_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_data_breakpoints: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_read_memory_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_write_memory_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_disassemble_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_cancel_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_breakpoint_locations_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_clipboard_context: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_stepping_granularity: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_instruction_breakpoints: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_exception_filter_options: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception_breakpoint_filters: Option<Vec<ExceptionBreakpointsFilter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_trigger_characters: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_module_columns: Option<Vec<ColumnDescriptor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_checksum_algorithms: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checksum {
    pub algorithm: String,
    pub checksum: String,
}

#[derive(Debug, Default, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<Source>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksums: Option<Vec<Checksum>>,
}

#[derive(Debug, Default, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakpoint {
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit_condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_message: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Breakpoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<usize>,
    pub verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrameFormat {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_types: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_names: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_values: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_all: Option<bool>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
    pub id: usize,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    pub line: usize,
    pub column: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_restart: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_pointer_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: ThreadId,
    pub name: String,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<String>,
    pub variables_reference: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub named_variables: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_variables: Option<usize>,
    pub expensive: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<usize>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueFormat {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hex: Option<bool>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablePresentationHint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    pub value: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<VariablePresentationHint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluate_name: Option<String>,
    pub variables_reference: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub named_variables: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_variables: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_reference: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Module {
    pub id: String, // TODO: || number
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_optimized: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_user_code: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_time_stamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_range: Option<String>,
}

pub mod requests {
    use super::*;
    #[derive(Debug, Default, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct InitializeArguments {
        #[serde(rename = "clientID", skip_serializing_if = "Option::is_none")]
        pub client_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub client_name: Option<String>,
        #[serde(rename = "adapterID")]
        pub adapter_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub locale: Option<String>,
        #[serde(rename = "linesStartAt1", skip_serializing_if = "Option::is_none")]
        pub lines_start_at_one: Option<bool>,
        #[serde(rename = "columnsStartAt1", skip_serializing_if = "Option::is_none")]
        pub columns_start_at_one: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub path_format: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub supports_variable_type: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub supports_variable_paging: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub supports_run_in_terminal_request: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub supports_memory_references: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub supports_progress_reporting: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
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
        type Result = Value;
        const COMMAND: &'static str = "launch";
    }

    #[derive(Debug)]
    pub enum Attach {}

    impl Request for Attach {
        type Arguments = Value;
        type Result = Value;
        const COMMAND: &'static str = "attach";
    }

    #[derive(Debug)]
    pub enum Disconnect {}

    impl Request for Disconnect {
        type Arguments = ();
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

    #[derive(Debug, Default, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SetBreakpointsArguments {
        pub source: Source,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub breakpoints: Option<Vec<SourceBreakpoint>>,
        // lines is deprecated
        #[serde(skip_serializing_if = "Option::is_none")]
        pub source_modified: Option<bool>,
    }

    #[derive(Debug, Default, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SetBreakpointsResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub breakpoints: Option<Vec<Breakpoint>>,
    }

    #[derive(Debug)]
    pub enum SetBreakpoints {}

    impl Request for SetBreakpoints {
        type Arguments = SetBreakpointsArguments;
        type Result = SetBreakpointsResponse;
        const COMMAND: &'static str = "setBreakpoints";
    }

    #[derive(Debug, Default, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ContinueArguments {
        pub thread_id: ThreadId,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ContinueResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub all_threads_continued: Option<bool>,
    }

    #[derive(Debug)]
    pub enum Continue {}

    impl Request for Continue {
        type Arguments = ContinueArguments;
        type Result = ContinueResponse;
        const COMMAND: &'static str = "continue";
    }

    #[derive(Debug, Default, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StackTraceArguments {
        pub thread_id: ThreadId,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub start_frame: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub levels: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub format: Option<StackFrameFormat>,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StackTraceResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
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

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
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

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ScopesArguments {
        pub frame_id: usize,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
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

    #[derive(Debug, Default, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VariablesArguments {
        pub variables_reference: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub filter: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub start: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub count: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub format: Option<ValueFormat>,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
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

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StepInArguments {
        pub thread_id: ThreadId,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub target_id: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub granularity: Option<String>,
    }

    #[derive(Debug)]
    pub enum StepIn {}

    impl Request for StepIn {
        type Arguments = StepInArguments;
        type Result = ();
        const COMMAND: &'static str = "stepIn";
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StepOutArguments {
        pub thread_id: ThreadId,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub granularity: Option<String>,
    }

    #[derive(Debug)]
    pub enum StepOut {}

    impl Request for StepOut {
        type Arguments = StepOutArguments;
        type Result = ();
        const COMMAND: &'static str = "stepOut";
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NextArguments {
        pub thread_id: ThreadId,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub granularity: Option<String>,
    }

    #[derive(Debug)]
    pub enum Next {}

    impl Request for Next {
        type Arguments = NextArguments;
        type Result = ();
        const COMMAND: &'static str = "next";
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
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

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EvaluateArguments {
        pub expression: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub frame_id: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub context: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub format: Option<ValueFormat>,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EvaluateResponse {
        pub result: String,
        #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
        pub ty: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub presentation_hint: Option<VariablePresentationHint>,
        pub variables_reference: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub named_variables: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub indexed_variables: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memory_reference: Option<String>,
    }

    #[derive(Debug)]
    pub enum Evaluate {}

    impl Request for Evaluate {
        type Arguments = EvaluateArguments;
        type Result = EvaluateResponse;
        const COMMAND: &'static str = "evaluate";
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SetExceptionBreakpointsArguments {
        pub filters: Vec<String>,
        // pub filterOptions: Option<Vec<ExceptionFilterOptions>>, // needs capability
        // pub exceptionOptions: Option<Vec<ExceptionOptions>>, // needs capability
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SetExceptionBreakpointsResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
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

    #[derive(Debug, Default, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RunInTerminalResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub process_id: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub shell_process_id: Option<u32>,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RunInTerminalArguments {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub kind: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        pub cwd: String,
        pub args: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
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

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Stopped {
        pub reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub thread_id: Option<ThreadId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub preserve_focus_hint: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub all_threads_stopped: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub hit_breakpoint_ids: Option<Vec<usize>>,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Continued {
        pub thread_id: ThreadId,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub all_threads_continued: Option<bool>,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Exited {
        pub exit_code: usize,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Terminated {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub restart: Option<Value>,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Thread {
        pub reason: String,
        pub thread_id: ThreadId,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Output {
        pub output: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub category: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub group: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub line: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub column: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub variables_reference: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub source: Option<Source>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub data: Option<Value>,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Breakpoint {
        pub reason: String,
        pub breakpoint: super::Breakpoint,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Module {
        pub reason: String,
        pub module: super::Module,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LoadedSource {
        pub reason: String,
        pub source: super::Source,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Process {
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub system_process_id: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub is_local_process: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub start_method: Option<String>, // TODO: use enum
        #[serde(skip_serializing_if = "Option::is_none")]
        pub pointer_size: Option<usize>,
    }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Capabilities {
        pub capabilities: super::DebuggerCapabilities,
    }

    // #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    // #[serde(rename_all = "camelCase")]
    // pub struct Invalidated {
    // pub areas: Vec<InvalidatedArea>,
    // pub thread_id: Option<ThreadId>,
    // pub stack_frame_id: Option<usize>,
    // }

    #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Memory {
        pub memory_reference: String,
        pub offset: usize,
        pub count: usize,
    }
}
