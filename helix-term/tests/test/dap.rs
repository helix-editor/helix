use super::*;

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener},
};

use helix_core::syntax::config::DebugAdapterConfig;
use helix_dap::ThreadId;
use helix_view::handlers::dap::select_thread_id;
use serde_json::{json, Value};

/// Starts a debug adapter on a local port that answers every request except
/// `stackTrace`, which it fails.
fn spawn_adapter_without_stack_traces() -> anyhow::Result<SocketAddr> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;

    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut writer = stream.try_clone().unwrap();
        let mut reader = BufReader::new(stream);

        loop {
            let mut content_length = 0;
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).unwrap() == 0 {
                    return;
                }
                let header = header.trim();
                if header.is_empty() {
                    break;
                }
                if let Some(value) = header.strip_prefix("Content-Length: ") {
                    content_length = value.parse().unwrap();
                }
            }

            let mut content = vec![0; content_length];
            reader.read_exact(&mut content).unwrap();
            let request: Value = serde_json::from_slice(&content).unwrap();
            let command = request["command"].as_str().unwrap();

            let response = json!({
                "seq": 0,
                "type": "response",
                "request_seq": request["seq"],
                "command": command,
                "success": command != "stackTrace",
                "body": {},
            })
            .to_string();
            write!(
                writer,
                "Content-Length: {}\r\n\r\n{}",
                response.len(),
                response
            )
            .unwrap();
        }
    });

    Ok(addr)
}

#[tokio::test(flavor = "multi_thread")]
async fn select_thread_without_stack_trace() -> anyhow::Result<()> {
    let addr = spawn_adapter_without_stack_traces()?;
    let mut app = helpers::AppBuilder::new().build()?;
    let editor = &mut app.editor;

    let config = DebugAdapterConfig {
        name: "test".into(),
        transport: "tcp".into(),
        command: String::new(),
        args: Vec::new(),
        port_arg: None,
        templates: Vec::new(),
        quirks: Default::default(),
    };
    let id = editor.debug_adapters.start_client(Some(addr), &config)?;
    editor.debug_adapters.set_active_client(id);

    let thread_id: ThreadId = serde_json::from_value(json!(1))?;
    select_thread_id(editor, thread_id, true).await;

    let debugger = editor.debug_adapters.get_active_client().unwrap();
    assert_eq!(debugger.thread_id, Some(thread_id));
    assert!(!debugger.stack_frames.contains_key(&thread_id));

    Ok(())
}
