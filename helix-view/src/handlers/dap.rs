use crate::editor::{Action, Breakpoint};
use crate::{align_view, Align, Editor};
use helix_core::Selection;
use helix_dap::{self as dap, Client, Payload, Request, ThreadId};
use helix_lsp::block_on;
use log::warn;
use std::io::ErrorKind;
use std::path::PathBuf;

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

    if let Err(e) = editor.open(&path, Action::Replace) {
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

pub fn breakpoints_changed(
    debugger: &mut dap::Client,
    path: PathBuf,
    breakpoints: &mut [Breakpoint],
) -> Result<(), anyhow::Error> {
    // TODO: handle capabilities correctly again, by filtering breakpoints when emitting
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

impl Editor {
    pub async fn handle_debugger_message(&mut self, payload: helix_dap::Payload) -> bool {
        use dap::requests::RunInTerminal;
        use helix_dap::{events, Event};

        let debugger = match self.debugger.as_mut() {
            Some(debugger) => debugger,
            None => return false,
        };
        match payload {
            Payload::Event(ev) => match *ev {
                Event::Stopped(events::Stopped {
                    thread_id,
                    description,
                    text,
                    reason,
                    all_threads_stopped,
                    ..
                }) => {
                    let all_threads_stopped = all_threads_stopped.unwrap_or_default();

                    if all_threads_stopped {
                        if let Ok(response) = debugger.request::<dap::requests::Threads>(()).await {
                            for thread in response.threads {
                                fetch_stack_trace(debugger, thread.id).await;
                            }
                            select_thread_id(self, thread_id.unwrap_or_default(), false).await;
                        }
                    } else if let Some(thread_id) = thread_id {
                        debugger.thread_states.insert(thread_id, reason.clone()); // TODO: dap uses "type" || "reason" here

                        // whichever thread stops is made "current" (if no previously selected thread).
                        select_thread_id(self, thread_id, false).await;
                    }

                    let scope = match thread_id {
                        Some(id) => format!("Thread {}", id),
                        None => "Target".to_owned(),
                    };

                    let mut status = format!("{} stopped because of {}", scope, reason);
                    if let Some(desc) = description {
                        status.push_str(&format!(" {}", desc));
                    }
                    if let Some(text) = text {
                        status.push_str(&format!(" {}", text));
                    }
                    if all_threads_stopped {
                        status.push_str(" (all threads stopped)");
                    }

                    self.set_status(status);
                }
                Event::Continued(events::Continued { thread_id, .. }) => {
                    debugger
                        .thread_states
                        .insert(thread_id, "running".to_owned());
                    if debugger.thread_id == Some(thread_id) {
                        debugger.resume_application();
                    }
                }
                Event::Thread(_) => {
                    // TODO: update thread_states, make threads request
                }
                Event::Breakpoint(events::Breakpoint { reason, breakpoint }) => {
                    match &reason[..] {
                        "new" => {
                            if let Some(source) = breakpoint.source {
                                self.breakpoints
                                    .entry(source.path.unwrap()) // TODO: no unwraps
                                    .or_default()
                                    .push(Breakpoint {
                                        id: breakpoint.id,
                                        verified: breakpoint.verified,
                                        message: breakpoint.message,
                                        line: breakpoint.line.unwrap().saturating_sub(1), // TODO: no unwrap
                                        column: breakpoint.column,
                                        ..Default::default()
                                    });
                            }
                        }
                        "changed" => {
                            for breakpoints in self.breakpoints.values_mut() {
                                if let Some(i) =
                                    breakpoints.iter().position(|b| b.id == breakpoint.id)
                                {
                                    breakpoints[i].verified = breakpoint.verified;
                                    breakpoints[i].message = breakpoint.message.clone();
                                    breakpoints[i].line =
                                        breakpoint.line.unwrap().saturating_sub(1); // TODO: no unwrap
                                    breakpoints[i].column = breakpoint.column;
                                }
                            }
                        }
                        "removed" => {
                            for breakpoints in self.breakpoints.values_mut() {
                                if let Some(i) =
                                    breakpoints.iter().position(|b| b.id == breakpoint.id)
                                {
                                    breakpoints.remove(i);
                                }
                            }
                        }
                        reason => {
                            warn!("Unknown breakpoint event: {}", reason);
                        }
                    }
                }
                Event::Output(events::Output {
                    category, output, ..
                }) => {
                    let prefix = match category {
                        Some(category) => {
                            if &category == "telemetry" {
                                return false;
                            }
                            format!("Debug ({}):", category)
                        }
                        None => "Debug:".to_owned(),
                    };

                    log::info!("{}", output);
                    self.set_status(format!("{} {}", prefix, output));
                }
                Event::Initialized => {
                    // send existing breakpoints
                    for (path, breakpoints) in &mut self.breakpoints {
                        // TODO: call futures in parallel, await all
                        let _ = breakpoints_changed(debugger, path.clone(), breakpoints);
                    }
                    // TODO: fetch breakpoints (in case we're attaching)

                    if debugger.configuration_done().await.is_ok() {
                        self.set_status("Debugged application started");
                    }; // TODO: do we need to handle error?
                }
                ev => {
                    log::warn!("Unhandled event {:?}", ev);
                    return false; // return early to skip render
                }
            },
            Payload::Response(_) => unreachable!(),
            Payload::Request(request) => match request.command.as_str() {
                RunInTerminal::COMMAND => {
                    let arguments: dap::requests::RunInTerminalArguments =
                        serde_json::from_value(request.arguments.unwrap_or_default()).unwrap();
                    // TODO: no unwrap

                    let process = if cfg!(windows) {
                        std::process::Command::new("wt")
                            .arg("new-tab")
                            .arg("--title")
                            .arg("DEBUG")
                            .arg("cmd")
                            .arg("/C")
                            .arg(arguments.args.join(" "))
                            .spawn()
                            .unwrap_or_else(|error| match error.kind() {
                                ErrorKind::NotFound => std::process::Command::new("conhost")
                                    .arg("cmd")
                                    .arg("/C")
                                    .arg(arguments.args.join(" "))
                                    .spawn()
                                    .unwrap(),
                                // TODO replace the pretty print {:?} with a regular format {}
                                // when the MSRV is raised to 1.60.0
                                e => panic!("Error to start debug console: {:?}", e),
                            })
                    } else {
                        std::process::Command::new("tmux")
                            .arg("split-window")
                            .arg(arguments.args.join(" "))
                            .spawn()
                            .unwrap()
                    };

                    let _ = debugger
                        .reply(
                            request.seq,
                            dap::requests::RunInTerminal::COMMAND,
                            serde_json::to_value(dap::requests::RunInTerminalResponse {
                                process_id: Some(process.id()),
                                shell_process_id: None,
                            })
                            .map_err(|e| e.into()),
                        )
                        .await;
                }
                _ => log::error!("DAP reverse request not implemented: {:?}", request),
            },
        }
        true
    }
}
