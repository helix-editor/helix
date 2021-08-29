use super::{Context, Editor};
use crate::ui::Picker;
use helix_dap::Client;
use helix_lsp::block_on;

use serde_json::{to_value, Value};
use tokio_stream::wrappers::UnboundedReceiverStream;

use std::collections::HashMap;

// DAP
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

    if let Some(debugger) = &mut cx.editor.debugger {
        let breakpoints = debugger.breakpoints.entry(path.clone()).or_default();
        if let Some(pos) = breakpoints.iter().position(|b| b.line == breakpoint.line) {
            breakpoints.remove(pos);
        } else {
            breakpoints.push(breakpoint);
        }

        let breakpoints = breakpoints.clone();

        let request = debugger.set_breakpoints(path, breakpoints);
        if let Err(e) = block_on(request) {
            cx.editor
                .set_error(format!("Failed to set breakpoints: {:?}", e));
        }
    }
}

pub fn dap_run(cx: &mut Context) {
    if let Some(debugger) = &mut cx.editor.debugger {
        if debugger.is_running {
            cx.editor
                .set_status("Debuggee is already running".to_owned());
            return;
        }
        let request = debugger.configuration_done();
        if let Err(e) = block_on(request) {
            cx.editor.set_error(format!("Failed to run: {:?}", e));
            return;
        }
        debugger.is_running = true;
    }
}

pub fn dap_continue(cx: &mut Context) {
    if let Some(debugger) = &mut cx.editor.debugger {
        if debugger.is_running {
            cx.editor
                .set_status("Debuggee is already running".to_owned());
            return;
        }

        let request = debugger.continue_thread(debugger.thread_id.unwrap());
        if let Err(e) = block_on(request) {
            cx.editor.set_error(format!("Failed to continue: {:?}", e));
            return;
        }
        debugger.is_running = true;
        debugger.stack_pointer = None;
    }
}

pub fn dap_pause(cx: &mut Context) {
    if let Some(debugger) = &mut cx.editor.debugger {
        if !debugger.is_running {
            cx.editor.set_status("Debuggee is not running".to_owned());
            return;
        }

        // FIXME: correct number here
        let request = debugger.pause(0);
        if let Err(e) = block_on(request) {
            cx.editor.set_error(format!("Failed to pause: {:?}", e));
        }
    }
}

pub fn dap_step_in(cx: &mut Context) {
    if let Some(debugger) = &mut cx.editor.debugger {
        if debugger.is_running {
            cx.editor
                .set_status("Debuggee is already running".to_owned());
            return;
        }

        let request = debugger.step_in(debugger.thread_id.unwrap());
        if let Err(e) = block_on(request) {
            cx.editor.set_error(format!("Failed to step: {:?}", e));
        }
    }
}

pub fn dap_step_out(cx: &mut Context) {
    if let Some(debugger) = &mut cx.editor.debugger {
        if debugger.is_running {
            cx.editor
                .set_status("Debuggee is already running".to_owned());
            return;
        }

        let request = debugger.step_out(debugger.thread_id.unwrap());
        if let Err(e) = block_on(request) {
            cx.editor.set_error(format!("Failed to step: {:?}", e));
        }
    }
}

pub fn dap_next(cx: &mut Context) {
    if let Some(debugger) = &mut cx.editor.debugger {
        if debugger.is_running {
            cx.editor
                .set_status("Debuggee is already running".to_owned());
            return;
        }

        let request = debugger.next(debugger.thread_id.unwrap());
        if let Err(e) = block_on(request) {
            cx.editor.set_error(format!("Failed to step: {:?}", e));
        }
    }
}

pub fn dap_variables(cx: &mut Context) {
    if let Some(debugger) = &mut cx.editor.debugger {
        if debugger.is_running {
            cx.editor
                .set_status("Cannot access variables while target is running".to_owned());
            return;
        }
        if debugger.stack_pointer.is_none() {
            cx.editor
                .set_status("Cannot find current stack pointer to access variables".to_owned());
            return;
        }

        let frame_id = debugger.stack_pointer.clone().unwrap().id;
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
}

pub fn dap_terminate(cx: &mut Context) {
    if let Some(debugger) = &mut cx.editor.debugger {
        let request = debugger.disconnect();
        if let Err(e) = block_on(request) {
            cx.editor
                .set_error(format!("Failed to disconnect: {:?}", e));
            return;
        }
        cx.editor.debugger = None;
    }
}

pub fn dap_switch_thread(cx: &mut Context) {
    if let Some(debugger) = &mut cx.editor.debugger {
        let request = debugger.threads();
        let threads = match block_on(request) {
            Ok(threads) => threads,
            Err(e) => {
                cx.editor
                    .set_error(format!("Failed to retrieve threads: {:?}", e));
                return;
            }
        };

        let picker = Picker::new(
            true,
            threads,
            |thread| thread.name.clone().into(),
            |editor, thread, _action| {
                if let Some(debugger) = &mut editor.debugger {
                    debugger.thread_id = Some(thread.id);
                    // TODO: probably need to refetch stack frames?
                }
            },
        );
        cx.push_layer(Box::new(picker))
    }
}
