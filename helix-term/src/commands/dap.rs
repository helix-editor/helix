use super::{align_view, Align, Context, Editor};
use crate::{
    compositor::{self, Compositor},
    job::{Callback, Jobs},
    ui::{self, overlay::overlayed, FilePicker, Picker, Popup, Prompt, PromptEvent, Text},
};
use helix_core::{
    syntax::{DebugArgumentValue, DebugConfigCompletion},
    Selection,
};
use helix_dap::{self as dap, Client, ThreadId};
use helix_lsp::block_on;
use helix_view::editor::Breakpoint;

use serde_json::{to_value, Value};
use tokio_stream::wrappers::UnboundedReceiverStream;

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;

use anyhow::{anyhow, bail};

#[macro_export]
macro_rules! debugger {
    ($editor:expr) => {{
        match &mut $editor.debugger {
            Some(debugger) => debugger,
            None => return,
        }
    }};
}

// general utils:
pub fn dap_pos_to_pos(doc: &helix_core::Rope, line: usize, column: usize) -> Option<usize> {
    // 1-indexing to 0 indexing
    let line = doc.try_line_to_char(line - 1).ok()?;
    let pos = line + column.saturating_sub(1);
    // TODO: this is probably utf-16 offsets
    Some(pos)
}

pub async fn select_thread_id(editor: &mut Editor, thread_id: ThreadId, force: bool) {
    let debugger = debugger!(editor);

    if !force && debugger.thread_id.is_some() {
        return;
    }

    debugger.thread_id = Some(thread_id);
    fetch_stack_trace(debugger, thread_id).await;

    let frame = debugger.stack_frames[&thread_id].get(0).cloned();
    if let Some(frame) = &frame {
        jump_to_stack_frame(editor, frame);
    }
}

pub async fn fetch_stack_trace(debugger: &mut Client, thread_id: ThreadId) {
    let (frames, _) = match debugger.stack_trace(thread_id).await {
        Ok(frames) => frames,
        Err(_) => return,
    };
    debugger.stack_frames.insert(thread_id, frames);
    debugger.active_frame = Some(0);
}

pub fn jump_to_stack_frame(editor: &mut Editor, frame: &helix_dap::StackFrame) {
    let path = if let Some(helix_dap::Source {
        path: Some(ref path),
        ..
    }) = frame.source
    {
        path.clone()
    } else {
        return;
    };

    if let Err(e) = editor.open(path, helix_view::editor::Action::Replace) {
        editor.set_error(format!("Unable to jump to stack frame: {}", e));
        return;
    }

    let (view, doc) = current!(editor);

    let text_end = doc.text().len_chars().saturating_sub(1);
    let start = dap_pos_to_pos(doc.text(), frame.line, frame.column).unwrap_or(0);
    let end = frame
        .end_line
        .and_then(|end_line| dap_pos_to_pos(doc.text(), end_line, frame.end_column.unwrap_or(0)))
        .unwrap_or(start);

    let selection = Selection::single(start.min(text_end), end.min(text_end));
    doc.set_selection(view.id, selection);
    align_view(doc, view, Align::Center);
}

fn thread_picker(
    cx: &mut Context,
    callback_fn: impl Fn(&mut Editor, &dap::Thread) + Send + 'static,
) {
    let debugger = debugger!(cx.editor);

    let future = debugger.threads();
    dap_callback(
        cx.jobs,
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: dap::requests::ThreadsResponse| {
            let threads = response.threads;
            if threads.len() == 1 {
                callback_fn(editor, &threads[0]);
                return;
            }
            let debugger = debugger!(editor);

            let thread_states = debugger.thread_states.clone();
            let picker = FilePicker::new(
                threads,
                move |_, thread| {
                    format!(
                        "{} ({})",
                        thread.name,
                        thread_states
                            .get(&thread.id)
                            .map(|state| state.as_str())
                            .unwrap_or("unknown")
                    )
                    .into()
                },
                move |cx, thread, _action| callback_fn(cx.editor, thread),
                move |editor, thread| {
                    let frames = editor.debugger.as_ref()?.stack_frames.get(&thread.id)?;
                    let frame = frames.get(0)?;
                    let path = frame.source.as_ref()?.path.clone()?;
                    let pos = Some((
                        frame.line.saturating_sub(1),
                        frame.end_line.unwrap_or(frame.line).saturating_sub(1),
                    ));
                    Some((path, pos))
                },
            );
            compositor.push(Box::new(picker));
        },
    );
}

fn get_breakpoint_at_current_line(editor: &mut Editor) -> Option<(usize, Breakpoint)> {
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let line = doc.selection(view.id).primary().cursor_line(text);
    let path = doc.path()?;
    editor.breakpoints.get(path).and_then(|breakpoints| {
        let i = breakpoints.iter().position(|b| b.line == line);
        i.map(|i| (i, breakpoints[i].clone()))
    })
}

// -- DAP

fn dap_callback<T, F>(
    jobs: &mut Jobs,
    call: impl Future<Output = helix_dap::Result<serde_json::Value>> + 'static + Send,
    callback: F,
) where
    T: for<'de> serde::Deserialize<'de> + Send + 'static,
    F: FnOnce(&mut Editor, &mut Compositor, T) + Send + 'static,
{
    let callback = Box::pin(async move {
        let json = call.await?;
        let response = serde_json::from_value(json)?;
        let call: Callback = Box::new(move |editor: &mut Editor, compositor: &mut Compositor| {
            callback(editor, compositor, response)
        });
        Ok(call)
    });
    jobs.callback(callback);
}

pub fn dap_start_impl(
    cx: &mut compositor::Context,
    name: Option<&str>,
    socket: Option<std::net::SocketAddr>,
    params: Option<Vec<std::borrow::Cow<str>>>,
) -> Result<(), anyhow::Error> {
    let doc = doc!(cx.editor);

    let config = doc
        .language_config()
        .and_then(|config| config.debugger.as_ref())
        .ok_or_else(|| anyhow!("No debug adapter available for language"))?;

    let result = match socket {
        Some(socket) => block_on(Client::tcp(socket, 0)),
        None => block_on(Client::process(
            &config.transport,
            &config.command,
            config.args.iter().map(|arg| arg.as_str()).collect(),
            config.port_arg.as_deref(),
            0,
        )),
    };

    let (mut debugger, events) = match result {
        Ok(r) => r,
        Err(e) => bail!("Failed to start debug session: {}", e),
    };

    let request = debugger.initialize(config.name.clone());
    if let Err(e) = block_on(request) {
        bail!("Failed to initialize debug adapter: {}", e);
    }

    debugger.quirks = config.quirks.clone();

    // TODO: avoid refetching all of this... pass a config in
    let template = match name {
        Some(name) => config.templates.iter().find(|t| t.name == name),
        None => config.templates.get(0),
    }
    .ok_or_else(|| anyhow!("No debug config with given name"))?;

    let mut args: HashMap<&str, Value> = HashMap::new();

    if let Some(params) = params {
        for (k, t) in &template.args {
            let mut value = t.clone();
            for (i, x) in params.iter().enumerate() {
                let mut param = x.to_string();
                if let Some(DebugConfigCompletion::Advanced(cfg)) = template.completion.get(i) {
                    if matches!(cfg.completion.as_deref(), Some("filename" | "directory")) {
                        param = std::fs::canonicalize(x.as_ref())
                            .ok()
                            .and_then(|pb| pb.into_os_string().into_string().ok())
                            .unwrap_or_else(|| x.to_string());
                    }
                }
                // For param #0 replace {0} in args
                let pattern = format!("{{{}}}", i);
                value = match value {
                    // TODO: just use toml::Value -> json::Value
                    DebugArgumentValue::String(v) => {
                        DebugArgumentValue::String(v.replace(&pattern, &param))
                    }
                    DebugArgumentValue::Array(arr) => DebugArgumentValue::Array(
                        arr.iter().map(|v| v.replace(&pattern, &param)).collect(),
                    ),
                    DebugArgumentValue::Boolean(_) => value,
                };
            }

            match value {
                DebugArgumentValue::String(string) => {
                    if let Ok(integer) = string.parse::<usize>() {
                        args.insert(k, to_value(integer).unwrap());
                    } else {
                        args.insert(k, to_value(string).unwrap());
                    }
                }
                DebugArgumentValue::Array(arr) => {
                    args.insert(k, to_value(arr).unwrap());
                }
                DebugArgumentValue::Boolean(bool) => {
                    args.insert(k, to_value(bool).unwrap());
                }
            }
        }
    }

    let args = to_value(args).unwrap();

    let callback = |_editor: &mut Editor, _compositor: &mut Compositor, _response: Value| {
        // if let Err(e) = result {
        //     editor.set_error(format!("Failed {} target: {}", template.request, e));
        // }
    };

    match &template.request[..] {
        "launch" => {
            let call = debugger.launch(args);
            dap_callback(cx.jobs, call, callback);
        }
        "attach" => {
            let call = debugger.attach(args);
            dap_callback(cx.jobs, call, callback);
        }
        request => bail!("Unsupported request '{}'", request),
    };

    // TODO: either await "initialized" or buffer commands until event is received
    cx.editor.debugger = Some(debugger);
    let stream = UnboundedReceiverStream::new(events);
    cx.editor.debugger_events.push(stream);
    Ok(())
}

pub fn dap_launch(cx: &mut Context) {
    if cx.editor.debugger.is_some() {
        cx.editor.set_error("Debugger is already running");
        return;
    }

    let doc = doc!(cx.editor);

    let config = match doc
        .language_config()
        .and_then(|config| config.debugger.as_ref())
    {
        Some(c) => c,
        None => {
            cx.editor
                .set_error("No debug adapter available for language");
            return;
        }
    };

    let templates = config.templates.clone();

    cx.push_layer(Box::new(overlayed(Picker::new(
        templates,
        |_, template| template.name.as_str().into(),
        |cx, template, _action| {
            let completions = template.completion.clone();
            let name = template.name.clone();
            let callback = Box::pin(async move {
                let call: Callback =
                    Box::new(move |_editor: &mut Editor, compositor: &mut Compositor| {
                        let prompt = debug_parameter_prompt(completions, name, Vec::new());
                        compositor.push(Box::new(prompt));
                    });
                Ok(call)
            });
            cx.jobs.callback(callback);
        },
        |_, _| {},
        |_| {},
    ))));
}

fn debug_parameter_prompt(
    completions: Vec<DebugConfigCompletion>,
    config_name: String,
    mut params: Vec<String>,
) -> Prompt {
    let completion = completions.get(params.len()).unwrap();
    let field_type = if let DebugConfigCompletion::Advanced(cfg) = completion {
        cfg.completion.as_deref().unwrap_or("")
    } else {
        ""
    };
    let name = match completion {
        DebugConfigCompletion::Advanced(cfg) => cfg.name.as_deref().unwrap_or(field_type),
        DebugConfigCompletion::Named(name) => name.as_str(),
    };
    let default_val = match completion {
        DebugConfigCompletion::Advanced(cfg) => cfg.default.as_deref().unwrap_or(""),
        _ => "",
    }
    .to_owned();

    let completer = match field_type {
        "filename" => ui::completers::filename,
        "directory" => ui::completers::directory,
        _ => ui::completers::none,
    };

    Prompt::new(
        format!("{}: ", name).into(),
        None,
        completer,
        move |cx, input: &str, event: PromptEvent| {
            if event != PromptEvent::Validate {
                return;
            }

            let mut value = input.to_owned();
            if value.is_empty() {
                value = default_val.clone();
            }
            params.push(value);

            if params.len() < completions.len() {
                let completions = completions.clone();
                let config_name = config_name.clone();
                let params = params.clone();
                let callback = Box::pin(async move {
                    let call: Callback =
                        Box::new(move |_editor: &mut Editor, compositor: &mut Compositor| {
                            let prompt = debug_parameter_prompt(completions, config_name, params);
                            compositor.push(Box::new(prompt));
                        });
                    Ok(call)
                });
                cx.jobs.callback(callback);
            } else if let Err(err) = dap_start_impl(
                cx,
                Some(&config_name),
                None,
                Some(params.iter().map(|x| x.into()).collect()),
            ) {
                cx.editor.set_error(err.to_string());
            }
        },
    )
}

pub fn dap_toggle_breakpoint(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let path = match doc.path() {
        Some(path) => path.clone(),
        None => {
            cx.editor
                .set_error("Can't set breakpoint: document has no path");
            return;
        }
    };
    let text = doc.text().slice(..);
    let line = doc.selection(view.id).primary().cursor_line(text);
    dap_toggle_breakpoint_impl(cx, path, line);
}

pub fn breakpoints_changed(
    debugger: &mut dap::Client,
    path: PathBuf,
    breakpoints: &mut [Breakpoint],
) -> Result<(), anyhow::Error> {
    // TODO: handle capabilities correctly again, by filterin breakpoints when emitting
    // if breakpoint.condition.is_some()
    //     && !debugger
    //         .caps
    //         .as_ref()
    //         .unwrap()
    //         .supports_conditional_breakpoints
    //         .unwrap_or_default()
    // {
    //     bail!(
    //         "Can't edit breakpoint: debugger does not support conditional breakpoints"
    //     )
    // }
    // if breakpoint.log_message.is_some()
    //     && !debugger
    //         .caps
    //         .as_ref()
    //         .unwrap()
    //         .supports_log_points
    //         .unwrap_or_default()
    // {
    //     bail!("Can't edit breakpoint: debugger does not support logpoints")
    // }
    let source_breakpoints = breakpoints
        .iter()
        .map(|breakpoint| helix_dap::SourceBreakpoint {
            line: breakpoint.line + 1, // convert from 0-indexing to 1-indexing (TODO: could set debugger to 0-indexing on init)
            ..Default::default()
        })
        .collect::<Vec<_>>();

    let request = debugger.set_breakpoints(path, source_breakpoints);
    match block_on(request) {
        Ok(Some(dap_breakpoints)) => {
            for (breakpoint, dap_breakpoint) in breakpoints.iter_mut().zip(dap_breakpoints) {
                breakpoint.id = dap_breakpoint.id;
                breakpoint.verified = dap_breakpoint.verified;
                breakpoint.message = dap_breakpoint.message;
                // TODO: handle breakpoint.message
                // TODO: verify source matches
                breakpoint.line = dap_breakpoint.line.unwrap_or(0).saturating_sub(1); // convert to 0-indexing
                                                                                      // TODO: no unwrap
                breakpoint.column = dap_breakpoint.column;
                // TODO: verify end_linef/col instruction reference, offset
            }
        }
        Err(e) => anyhow::bail!("Failed to set breakpoints: {}", e),
        _ => {}
    };
    Ok(())
}

pub fn dap_toggle_breakpoint_impl(cx: &mut Context, path: PathBuf, line: usize) {
    // TODO: need to map breakpoints over edits and update them?
    // we shouldn't really allow editing while debug is running though

    let breakpoints = cx.editor.breakpoints.entry(path.clone()).or_default();
    // TODO: always keep breakpoints sorted and use binary search to determine insertion point
    if let Some(pos) = breakpoints
        .iter()
        .position(|breakpoint| breakpoint.line == line)
    {
        breakpoints.remove(pos);
    } else {
        breakpoints.push(Breakpoint {
            line,
            ..Default::default()
        });
    }

    let debugger = debugger!(cx.editor);

    if let Err(e) = breakpoints_changed(debugger, path, breakpoints) {
        cx.editor
            .set_error(format!("Failed to set breakpoints: {}", e));
    }
}

pub fn dap_continue(cx: &mut Context) {
    let debugger = debugger!(cx.editor);

    if let Some(thread_id) = debugger.thread_id {
        let request = debugger.continue_thread(thread_id);

        dap_callback(
            cx.jobs,
            request,
            |editor, _compositor, _response: dap::requests::ContinueResponse| {
                debugger!(editor).resume_application();
            },
        );
    } else {
        cx.editor
            .set_error("Currently active thread is not stopped. Switch the thread.");
    }
}

pub fn dap_pause(cx: &mut Context) {
    thread_picker(cx, |editor, thread| {
        let debugger = debugger!(editor);
        let request = debugger.pause(thread.id);
        // NOTE: we don't need to set active thread id here because DAP will emit a "stopped" event
        if let Err(e) = block_on(request) {
            editor.set_error(format!("Failed to pause: {}", e));
        }
    })
}

pub fn dap_step_in(cx: &mut Context) {
    let debugger = debugger!(cx.editor);

    if let Some(thread_id) = debugger.thread_id {
        let request = debugger.step_in(thread_id);

        dap_callback(cx.jobs, request, |editor, _compositor, _response: ()| {
            debugger!(editor).resume_application();
        });
    } else {
        cx.editor
            .set_error("Currently active thread is not stopped. Switch the thread.");
    }
}

pub fn dap_step_out(cx: &mut Context) {
    let debugger = debugger!(cx.editor);

    if let Some(thread_id) = debugger.thread_id {
        let request = debugger.step_out(thread_id);
        dap_callback(cx.jobs, request, |editor, _compositor, _response: ()| {
            debugger!(editor).resume_application();
        });
    } else {
        cx.editor
            .set_error("Currently active thread is not stopped. Switch the thread.");
    }
}

pub fn dap_next(cx: &mut Context) {
    let debugger = debugger!(cx.editor);

    if let Some(thread_id) = debugger.thread_id {
        let request = debugger.next(thread_id);
        dap_callback(cx.jobs, request, |editor, _compositor, _response: ()| {
            debugger!(editor).resume_application();
        });
    } else {
        cx.editor
            .set_error("Currently active thread is not stopped. Switch the thread.");
    }
}

pub fn dap_variables(cx: &mut Context) {
    let debugger = debugger!(cx.editor);

    if debugger.thread_id.is_none() {
        cx.editor
            .set_status("Cannot access variables while target is running");
        return;
    }
    let (frame, thread_id) = match (debugger.active_frame, debugger.thread_id) {
        (Some(frame), Some(thread_id)) => (frame, thread_id),
        _ => {
            cx.editor
                .set_status("Cannot find current stack frame to access variables");
            return;
        }
    };

    let frame_id = debugger.stack_frames[&thread_id][frame].id;
    let scopes = match block_on(debugger.scopes(frame_id)) {
        Ok(s) => s,
        Err(e) => {
            cx.editor.set_error(format!("Failed to get scopes: {}", e));
            return;
        }
    };

    // TODO: allow expanding variables into sub-fields
    let mut variables = Vec::new();

    let theme = &cx.editor.theme;
    let scope_style = theme.get("ui.linenr.selected");
    let type_style = theme.get("ui.text");
    let text_style = theme.get("ui.text.focus");

    for scope in scopes.iter() {
        // use helix_view::graphics::Style;
        use tui::text::{Span, Spans};
        let response = block_on(debugger.variables(scope.variables_reference));

        variables.push(Spans::from(Span::styled(
            format!("▸ {}", scope.name),
            scope_style,
        )));

        if let Ok(vars) = response {
            variables.reserve(vars.len());
            for var in vars {
                let mut spans = Vec::with_capacity(5);

                spans.push(Span::styled(var.name.to_owned(), text_style));
                if let Some(ty) = var.ty {
                    spans.push(Span::raw(": "));
                    spans.push(Span::styled(ty.to_owned(), type_style));
                }
                spans.push(Span::raw(" = "));
                spans.push(Span::styled(var.value.to_owned(), text_style));
                variables.push(Spans::from(spans));
            }
        }
    }

    let contents = Text::from(tui::text::Text::from(variables));
    let popup = Popup::new("dap-variables", contents);
    cx.push_layer(Box::new(popup));
}

pub fn dap_terminate(cx: &mut Context) {
    let debugger = debugger!(cx.editor);

    let request = debugger.disconnect();
    dap_callback(cx.jobs, request, |editor, _compositor, _response: ()| {
        // editor.set_error(format!("Failed to disconnect: {}", e));
        editor.debugger = None;
    });
}

pub fn dap_enable_exceptions(cx: &mut Context) {
    let debugger = debugger!(cx.editor);

    let filters = match &debugger.capabilities().exception_breakpoint_filters {
        Some(filters) => filters.iter().map(|f| f.filter.clone()).collect(),
        None => return,
    };

    let request = debugger.set_exception_breakpoints(filters);

    dap_callback(
        cx.jobs,
        request,
        |_editor, _compositor, _response: dap::requests::SetExceptionBreakpointsResponse| {
            // editor.set_error(format!("Failed to set up exception breakpoints: {}", e));
        },
    )
}

pub fn dap_disable_exceptions(cx: &mut Context) {
    let debugger = debugger!(cx.editor);

    let request = debugger.set_exception_breakpoints(Vec::new());

    dap_callback(
        cx.jobs,
        request,
        |_editor, _compositor, _response: dap::requests::SetExceptionBreakpointsResponse| {
            // editor.set_error(format!("Failed to set up exception breakpoints: {}", e));
        },
    )
}

// TODO: both edit condition and edit log need to be stable: we might get new breakpoints from the debugger which can change offsets
pub fn dap_edit_condition(cx: &mut Context) {
    if let Some((pos, breakpoint)) = get_breakpoint_at_current_line(cx.editor) {
        let path = match doc!(cx.editor).path() {
            Some(path) => path.clone(),
            None => return,
        };
        let callback = Box::pin(async move {
            let call: Callback =
                Box::new(move |_editor: &mut Editor, compositor: &mut Compositor| {
                    let mut prompt = Prompt::new(
                        "condition:".into(),
                        None,
                        ui::completers::none,
                        move |cx, input: &str, event: PromptEvent| {
                            if event != PromptEvent::Validate {
                                return;
                            }

                            let breakpoints = &mut cx.editor.breakpoints.get_mut(&path).unwrap();
                            breakpoints[pos].condition = match input {
                                "" => None,
                                input => Some(input.to_owned()),
                            };

                            let debugger = debugger!(cx.editor);

                            if let Err(e) = breakpoints_changed(debugger, path.clone(), breakpoints)
                            {
                                cx.editor
                                    .set_error(format!("Failed to set breakpoints: {}", e));
                            }
                        },
                    );
                    if let Some(condition) = breakpoint.condition {
                        prompt.insert_str(&condition)
                    }
                    compositor.push(Box::new(prompt));
                });
            Ok(call)
        });
        cx.jobs.callback(callback);
    }
}

pub fn dap_edit_log(cx: &mut Context) {
    if let Some((pos, breakpoint)) = get_breakpoint_at_current_line(cx.editor) {
        let path = match doc!(cx.editor).path() {
            Some(path) => path.clone(),
            None => return,
        };
        let callback = Box::pin(async move {
            let call: Callback =
                Box::new(move |_editor: &mut Editor, compositor: &mut Compositor| {
                    let mut prompt = Prompt::new(
                        "log-message:".into(),
                        None,
                        ui::completers::none,
                        move |cx, input: &str, event: PromptEvent| {
                            if event != PromptEvent::Validate {
                                return;
                            }

                            let breakpoints = &mut cx.editor.breakpoints.get_mut(&path).unwrap();
                            breakpoints[pos].log_message = match input {
                                "" => None,
                                input => Some(input.to_owned()),
                            };

                            let debugger = debugger!(cx.editor);
                            if let Err(e) = breakpoints_changed(debugger, path.clone(), breakpoints)
                            {
                                cx.editor
                                    .set_error(format!("Failed to set breakpoints: {}", e));
                            }
                        },
                    );
                    if let Some(log_message) = breakpoint.log_message {
                        prompt.insert_str(&log_message);
                    }
                    compositor.push(Box::new(prompt));
                });
            Ok(call)
        });
        cx.jobs.callback(callback);
    }
}

pub fn dap_switch_thread(cx: &mut Context) {
    thread_picker(cx, |editor, thread| {
        block_on(select_thread_id(editor, thread.id, true));
    })
}
pub fn dap_switch_stack_frame(cx: &mut Context) {
    let debugger = debugger!(cx.editor);

    let thread_id = match debugger.thread_id {
        Some(thread_id) => thread_id,
        None => {
            cx.editor.set_error("No thread is currently active");
            return;
        }
    };

    let frames = debugger.stack_frames[&thread_id].clone();

    let picker = FilePicker::new(
        frames,
        |_, frame| frame.name.as_str().into(), // TODO: include thread_states in the label
        move |cx, frame, _action| {
            let debugger = debugger!(cx.editor);
            // TODO: this should be simpler to find
            let pos = debugger.stack_frames[&thread_id]
                .iter()
                .position(|f| f.id == frame.id);
            debugger.active_frame = pos;

            let frame = debugger.stack_frames[&thread_id]
                .get(pos.unwrap_or(0))
                .cloned();
            if let Some(frame) = &frame {
                jump_to_stack_frame(cx.editor, frame);
            }
        },
        move |_editor, frame| {
            frame
                .source
                .as_ref()
                .and_then(|source| source.path.clone())
                .map(|path| {
                    (
                        path,
                        Some((
                            frame.line.saturating_sub(1),
                            frame.end_line.unwrap_or(frame.line).saturating_sub(1),
                        )),
                    )
                })
        },
    );
    cx.push_layer(Box::new(picker))
}
