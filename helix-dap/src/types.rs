use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnDescriptor {
    pub attribute_name: String,
    pub label: String,
    pub format: Option<String>,
    #[serde(rename = "type")]
    pub col_type: Option<String>,
    pub width: Option<usize>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionBreakpointsFilter {
    pub filter: String,
    pub label: String,
    pub description: Option<String>,
    pub default: Option<bool>,
    pub supports_condition: Option<bool>,
    pub condition_description: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
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

impl std::ops::Deref for DebuggerCapabilities {
    type Target = Option<bool>;

    fn deref(&self) -> &Self::Target {
        &self.supports_exception_options
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
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
pub struct SetBreakpointsArguments {
    pub source: Source,
    pub breakpoints: Option<Vec<SourceBreakpoint>>,
    // lines is deprecated
    pub source_modified: Option<bool>,
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
pub struct SetBreakpointsResponseBody {
    pub breakpoints: Option<Vec<Breakpoint>>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueArguments {
    pub thread_id: usize,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueResponseBody {
    pub all_threads_continued: Option<bool>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
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

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceArguments {
    pub thread_id: usize,
    pub start_frame: Option<usize>,
    pub levels: Option<usize>,
    pub format: Option<StackFrameFormat>,
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
pub struct StackTraceResponseBody {
    pub total_frames: Option<usize>,
    pub stack_frames: Vec<StackFrame>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: usize,
    pub name: String,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadsResponseBody {
    pub threads: Vec<Thread>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopesArguments {
    pub frame_id: usize,
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
pub struct ScopesResponseBody {
    pub scopes: Vec<Scope>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueFormat {
    pub hex: Option<bool>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablesArguments {
    pub variables_reference: usize,
    pub filter: Option<String>,
    pub start: Option<usize>,
    pub count: Option<usize>,
    pub format: Option<ValueFormat>,
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
pub struct VariablesResponseBody {
    pub variables: Vec<Variable>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEventBody {
    pub output: String,
    pub category: Option<String>,
    pub group: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub variables_reference: Option<usize>,
    pub source: Option<Source>,
    pub data: Option<Value>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoppedEventBody {
    pub reason: String,
    pub description: Option<String>,
    pub thread_id: Option<usize>,
    pub preserve_focus_hint: Option<bool>,
    pub text: Option<String>,
    pub all_threads_stopped: Option<bool>,
    pub hit_breakpoint_ids: Option<Vec<usize>>,
}
