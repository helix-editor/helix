use super::{align_view, Align, Context, Editor};
use crate::{
    commands,
    compositor::Compositor,
    job::Callback,
    ui::{FilePicker, Prompt, PromptEvent},
};
use dap::StackFrame;
use helix_core::Selection;
use helix_dap::{self as dap, Client};
use helix_lsp::block_on;

use serde_json::{to_value, Value};
use tokio_stream::wrappers::UnboundedReceiverStream;

use std::collections::HashMap;

// general utils:
pub fn dap_pos_to_pos(doc: &helix_core::Rope, line: usize, column: usize) -> Option<usize> {
    // 1-indexing to 0 indexing
    let line = doc.try_line_to_char(line - 1).ok()?;
    let pos = line + column;
    // TODO: this is probably utf-16 offsets
    Some(pos)
}

pub fn resume_application(debugger: &mut Client) {
    if let Some(thread_id) = debugger.thread_id {
        debugger
            .thread_states
            .insert(thread_id, "running".to_string());
        debugger.stack_frames.remove(&thread_id);
    }
    debugger.active_frame = None;
    debugger.thread_id = None;
}

pub async fn select_thread_id(editor: &mut Editor, thread_id: isize, force: bool) {
    let debugger = match &mut editor.debugger {
        Some(debugger) => debugger,
        None => return,
    };

    if !force && debugger.thread_id.is_some() {
        return;
    }

    debugger.thread_id = Some(thread_id);

    // fetch stack trace
    // TODO: handle requesting more total frames
    let (frames, _) = match debugger.stack_trace(thread_id).await {
        Ok(frames) => frames,
        Err(_) => return,
    };
    debugger.stack_frames.insert(thread_id, frames);
    debugger.active_frame = Some(0); // TODO: check how to determine this

    let frame = debugger.stack_frames[&thread_id].get(0).cloned();
    if let Some(frame) = &frame {
        jump_to_stack_frame(editor, frame);
    }
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

    editor
        .open(path, helix_view::editor::Action::Replace)
        .unwrap(); // TODO: there should be no unwrapping!

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

fn thread_picker(cx: &mut Context, callback_fn: impl Fn(&mut Editor, &dap::Thread) + 'static) {
    let debugger = match &mut cx.editor.debugger {
        Some(debugger) => debugger,
        None => return,
    };

    let threads = match block_on(debugger.threads()) {
        Ok(threads) => threads,
        Err(e) => {
            cx.editor
                .set_error(format!("Failed to retrieve threads: {:?}", e));
            return;
        }
    };

    if threads.len() == 1 {
        callback_fn(cx.editor, &threads[0]);
        return;
    }

    let thread_states = debugger.thread_states.clone();
    let frames = debugger.stack_frames.clone();
    let picker = FilePicker::new(
        threads,
        move |thread| {
            format!(
                "{} ({})",
                thread.name,
                thread_states
                    .get(&thread.id)
                    .unwrap_or(&"unknown".to_owned())
            )
            .into()
        },
        move |editor, thread, _action| callback_fn(editor, thread),
        move |_editor, thread| {
            if let Some(frame) = frames.get(&thread.id).and_then(|bt| bt.get(0)) {
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
            } else {
                None
            }
        },
    );
    cx.push_layer(Box::new(picker))
}

// -- DAP

pub fn dap_start_impl(
    editor: &mut Editor,
    name: Option<&str>,
    socket: Option<std::net::SocketAddr>,
    params: Option<Vec<&str>>,
) {
    let (_, doc) = current!(editor);

    let path = match doc.path() {
        Some(path) => path.to_path_buf(),
        None => {
            editor.set_error("Can't start debug: document has no path".to_string());
            return;
        }
    };

    let config = editor
        .syn_loader
        .language_config_for_file_name(&path)
        .and_then(|x| x.debugger.clone());
    let config = match config {
        Some(c) => c,
        None => {
            editor.set_error(
                "Can't start debug: no debug adapter available for language".to_string(),
            );
            return;
        }
    };

    let result = match socket {
        Some(socket) => block_on(Client::tcp(socket, 0)),
        None => block_on(Client::process(
            config.transport.clone(),
            config.command.clone(),
            config.args.clone(),
            config.port_arg.clone(),
            0,
        )),
    };

    let (mut debugger, events) = match result {
        Ok(r) => r,
        Err(e) => {
            editor.set_error(format!("Failed to start debug session: {:?}", e));
            return;
        }
    };

    let request = debugger.initialize(config.name.clone());
    if let Err(e) = block_on(request) {
        editor.set_error(format!("Failed to initialize debug adapter: {:?}", e));
        return;
    }

    let start_config = match name {
        Some(name) => config.templates.iter().find(|t| t.name == name),
        None => config.templates.get(0),
    };
    let start_config = match start_config {
        Some(c) => c,
        None => {
            editor.set_error("Can't start debug: no debug config with given name".to_string());
            return;
        }
    };

    let template = start_config.args.clone();
    let mut args: HashMap<String, Value> = HashMap::new();

    if let Some(params) = params {
        for (k, t) in template {
            let mut value = t;
            for (i, x) in params.iter().enumerate() {
                // For param #0 replace {0} in args
                value = value.replace(format!("{{{}}}", i).as_str(), x);
            }

            if let Ok(integer) = value.parse::<usize>() {
                args.insert(k, Value::Number(serde_json::Number::from(integer)));
            } else {
                args.insert(k, Value::String(value));
            }
        }
    }

    let args = to_value(args).unwrap();

    let result = match &start_config.request[..] {
        "launch" => block_on(debugger.launch(args)),
        "attach" => block_on(debugger.attach(args)),
        _ => {
            editor.set_error("Unsupported request".to_string());
            return;
        }
    };
    if let Err(e) = result {
        editor.set_error(format!("Failed {} target: {:?}", start_config.request, e));
        return;
    }

    // TODO: either await "initialized" or buffer commands until event is received
    editor.debugger = Some(debugger);
    let stream = UnboundedReceiverStream::new(events);
    editor.debugger_events.push(stream);
}

pub fn dap_launch(cx: &mut Context) {
    if cx.editor.debugger.is_some() {
        cx.editor
            .set_error("Can't start debug: debugger is running".to_string());
        return;
    }

    let (_, doc) = current!(cx.editor);
    let path = match doc.path() {
        Some(path) => path.to_path_buf(),
        None => {
            cx.editor
                .set_error("Can't start debug: document has no path".to_string());
            return;
        }
    };

    let config = cx
        .editor
        .syn_loader
        .language_config_for_file_name(&path)
        .and_then(|x| x.debugger.clone());
    let config = match config {
        Some(c) => c,
        None => {
            cx.editor.set_error(
                "Can't start debug: no debug adapter available for language".to_string(),
            );
            return;
        }
    };

    cx.editor.debug_config_picker = Some(config.templates.iter().map(|t| t.name.clone()).collect());
    cx.editor.debug_config_completions = Some(
        config
            .templates
            .iter()
            .map(|t| t.completion.clone())
            .collect(),
    );
}

pub fn dap_toggle_breakpoint(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let pos = doc.selection(view.id).primary().cursor(text);

    let breakpoint = helix_dap::SourceBreakpoint {
        line: text.char_to_line(pos) + 1, // convert from 0-indexing to 1-indexing (TODO: could set debugger to 0-indexing on init)
        ..Default::default()
    };

    let path = match doc.path() {
        Some(path) => path.to_path_buf(),
        None => {
            cx.editor
                .set_error("Can't set breakpoint: document has no path".to_string());
            return;
        }
    };

    // TODO: need to map breakpoints over edits and update them?
    // we shouldn't really allow editing while debug is running though

    let breakpoints = cx.editor.breakpoints.entry(path.clone()).or_default();
    if let Some(pos) = breakpoints.iter().position(|b| b.line == breakpoint.line) {
        breakpoints.remove(pos);
    } else {
        breakpoints.push(breakpoint);
    }

    let breakpoints = breakpoints.clone();

    let debugger = match &mut cx.editor.debugger {
        Some(debugger) => debugger,
        None => return,
    };
    let request = debugger.set_breakpoints(path, breakpoints);
    if let Err(e) = block_on(request) {
        cx.editor
            .set_error(format!("Failed to set breakpoints: {:?}", e));
    }
}

pub fn dap_continue(cx: &mut Context) {
    let debugger = match &mut cx.editor.debugger {
        Some(debugger) => debugger,
        None => return,
    };

    if let Some(thread_id) = debugger.thread_id {
        let request = debugger.continue_thread(thread_id);
        if let Err(e) = block_on(request) {
            cx.editor.set_error(format!("Failed to continue: {:?}", e));
            return;
        }
        resume_application(debugger);
    } else {
        cx.editor
            .set_error("Currently active thread is not stopped. Switch the thread.".into());
    }
}

pub fn dap_pause(cx: &mut Context) {
    thread_picker(cx, |editor, thread| {
        let debugger = match &mut editor.debugger {
            Some(debugger) => debugger,
            None => return,
        };
        let request = debugger.pause(thread.id);
        // NOTE: we don't need to set active thread id here because DAP will emit a "stopped" event
        if let Err(e) = block_on(request) {
            editor.set_error(format!("Failed to pause: {:?}", e));
        }
    })
}

pub fn dap_step_in(cx: &mut Context) {
    let debugger = match &mut cx.editor.debugger {
        Some(debugger) => debugger,
        None => return,
    };

    if let Some(thread_id) = debugger.thread_id {
        let request = debugger.step_in(thread_id);
        if let Err(e) = block_on(request) {
            cx.editor.set_error(format!("Failed to continue: {:?}", e));
            return;
        }
        resume_application(debugger);
    } else {
        cx.editor
            .set_error("Currently active thread is not stopped. Switch the thread.".into());
    }
}

pub fn dap_step_out(cx: &mut Context) {
    let debugger = match &mut cx.editor.debugger {
        Some(debugger) => debugger,
        None => return,
    };

    if let Some(thread_id) = debugger.thread_id {
        let request = debugger.step_out(thread_id);
        if let Err(e) = block_on(request) {
            cx.editor.set_error(format!("Failed to continue: {:?}", e));
            return;
        }
        resume_application(debugger);
    } else {
        cx.editor
            .set_error("Currently active thread is not stopped. Switch the thread.".into());
    }
}

pub fn dap_next(cx: &mut Context) {
    let debugger = match &mut cx.editor.debugger {
        Some(debugger) => debugger,
        None => return,
    };

    if let Some(thread_id) = debugger.thread_id {
        let request = debugger.next(thread_id);
        if let Err(e) = block_on(request) {
            cx.editor.set_error(format!("Failed to continue: {:?}", e));
            return;
        }
        resume_application(debugger);
    } else {
        cx.editor
            .set_error("Currently active thread is not stopped. Switch the thread.".into());
    }
}

pub fn dap_variables(cx: &mut Context) {
    let debugger = match &mut cx.editor.debugger {
        Some(debugger) => debugger,
        None => return,
    };

    if debugger.thread_id.is_none() {
        cx.editor
            .set_status("Cannot access variables while target is running".to_owned());
        return;
    }
    let (frame, thread_id) = match (debugger.active_frame, debugger.thread_id) {
        (Some(frame), Some(thread_id)) => (frame, thread_id),
        _ => {
            cx.editor
                .set_status("Cannot find current stack frame to access variables".to_owned());
            return;
        }
    };

    let frame_id = debugger.stack_frames[&thread_id][frame].id;
    let scopes = match block_on(debugger.scopes(frame_id)) {
        Ok(s) => s,
        Err(e) => {
            cx.editor
                .set_error(format!("Failed to get scopes: {:?}", e));
            return;
        }
    };
    let mut variables = Vec::new();

    for scope in scopes.iter() {
        let response = block_on(debugger.variables(scope.variables_reference));

        if let Ok(vars) = response {
            variables.reserve(vars.len());
            for var in vars {
                let prefix = match var.data_type {
                    Some(data_type) => format!("{} ", data_type),
                    None => "".to_owned(),
                };
                variables.push(format!("{}{} = {}\n", prefix, var.name, var.value));
            }
        }
    }

    if !variables.is_empty() {
        cx.editor.variables = Some(variables);
        cx.editor.variables_page = 0;
    }
}

pub fn dap_terminate(cx: &mut Context) {
    let debugger = match &mut cx.editor.debugger {
        Some(debugger) => debugger,
        None => return,
    };

    let request = debugger.disconnect();
    if let Err(e) = block_on(request) {
        cx.editor
            .set_error(format!("Failed to disconnect: {:?}", e));
        return;
    }
    cx.editor.debugger = None;
}

pub fn dap_edit_condition(cx: &mut Context) {
    if let Some((pos, mut bp)) = commands::cmd::get_breakpoint_at_current_line(cx.editor) {
        let callback = Box::pin(async move {
            let call: Callback =
                Box::new(move |_editor: &mut Editor, compositor: &mut Compositor| {
                    let condition = bp.condition.clone();
                    let prompt = Prompt::new(
                        "condition: ".into(),
                        None,
                        |_input: &str| Vec::new(),
                        move |cx: &mut crate::compositor::Context,
                              input: &str,
                              event: PromptEvent| {
                            if event != PromptEvent::Validate {
                                return;
                            }

                            let (_, doc) = current!(cx.editor);
                            let path = match doc.path() {
                                Some(path) => path.to_path_buf(),
                                None => {
                                    cx.editor.set_status(
                                        "Can't edit breakpoint: document has no path".to_owned(),
                                    );
                                    return;
                                }
                            };

                            let breakpoints =
                                cx.editor.breakpoints.entry(path.clone()).or_default();
                            breakpoints.remove(pos);
                            bp.condition = match input {
                                "" => None,
                                input => Some(input.to_owned()),
                            };
                            breakpoints.push(bp.clone());

                            if let Some(debugger) = &mut cx.editor.debugger {
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
                                let request = debugger.set_breakpoints(path, breakpoints.clone());
                                if let Err(e) = block_on(request) {
                                    cx.editor
                                        .set_status(format!("Failed to set breakpoints: {:?}", e))
                                }
                            }
                        },
                        condition,
                    );
                    compositor.push(Box::new(prompt));
                });
            Ok(call)
        });
        cx.jobs.callback(callback);
    }
}

pub fn dap_edit_log(cx: &mut Context) {
    if let Some((pos, mut bp)) = commands::cmd::get_breakpoint_at_current_line(cx.editor) {
        let callback = Box::pin(async move {
            let call: Callback =
                Box::new(move |_editor: &mut Editor, compositor: &mut Compositor| {
                    let log_message = bp.log_message.clone();
                    let prompt = Prompt::new(
                        "log message: ".into(),
                        None,
                        |_input: &str| Vec::new(),
                        move |cx: &mut crate::compositor::Context,
                              input: &str,
                              event: PromptEvent| {
                            if event != PromptEvent::Validate {
                                return;
                            }

                            let (_, doc) = current!(cx.editor);
                            let path = match doc.path() {
                                Some(path) => path.to_path_buf(),
                                None => {
                                    cx.editor.set_status(
                                        "Can't edit breakpoint: document has no path".to_owned(),
                                    );
                                    return;
                                }
                            };

                            let breakpoints =
                                cx.editor.breakpoints.entry(path.clone()).or_default();
                            breakpoints.remove(pos);
                            bp.log_message = match input {
                                "" => None,
                                input => Some(input.to_owned()),
                            };
                            breakpoints.push(bp.clone());

                            if let Some(debugger) = &mut cx.editor.debugger {
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
                                let request = debugger.set_breakpoints(path, breakpoints.clone());
                                if let Err(e) = block_on(request) {
                                    cx.editor
                                        .set_status(format!("Failed to set breakpoints: {:?}", e))
                                }
                            }
                        },
                        log_message,
                    );
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
    let debugger = match &mut cx.editor.debugger {
        Some(debugger) => debugger,
        None => return,
    };

    let thread_id = match debugger.thread_id {
        Some(thread_id) => thread_id,
        None => {
            cx.editor
                .set_error("No thread is currently active".to_owned());
            return;
        }
    };

    let frames = debugger.stack_frames[&thread_id].clone();

    let picker = FilePicker::new(
        frames,
        |frame| frame.name.clone().into(), // TODO: include thread_states in the label
        move |editor, frame, _action| {
            let debugger = match &mut editor.debugger {
                Some(debugger) => debugger,
                None => return,
            };
            // TODO: this should be simpler to find
            let pos = debugger.stack_frames[&thread_id]
                .iter()
                .position(|f| f.id == frame.id);
            debugger.active_frame = pos;

            let frame = debugger.stack_frames[&thread_id]
                .get(pos.unwrap_or(0))
                .cloned();
            if let Some(frame) = &frame {
                jump_to_stack_frame(editor, frame);
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
