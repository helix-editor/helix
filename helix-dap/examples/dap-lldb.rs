use helix_dap::{events, Client, Event, Payload, Result, SourceBreakpoint};
use serde::{Deserialize, Serialize};
use serde_json::to_value;
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchArguments {
    program: String,
    console: String,
}

async fn dispatch(mut rx: UnboundedReceiver<Payload>) {
    loop {
        match rx.recv().await.unwrap() {
            Payload::Event(Event::Output(events::Output {
                category, output, ..
            })) => {
                println!(
                    "> [{}] {}",
                    category.unwrap_or("unknown".to_owned()),
                    output
                );
            }
            Payload::Event(Event::Stopped(_)) => {
                println!("stopped");
            }
            _ => {}
        };
    }
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let base_config = fern::Dispatch::new().level(log::LevelFilter::Info);

    let stderr_config = fern::Dispatch::new()
        .format(|out, message, record| out.finish(format_args!("[{}] {}", record.level(), message)))
        .chain(std::io::stderr());

    base_config
        .chain(stderr_config)
        .apply()
        .expect("Failed to set up logging");

    let (mut client, events) = Client::tcp_process("lldb-vscode", vec![], "-p {}", 0).await?;
    println!("create: {:?}", client);

    tokio::spawn(dispatch(events));

    println!(
        "init: {:?}",
        client.initialize("lldb-vscode".to_owned()).await
    );
    println!("caps: {:?}", client.capabilities());

    let args = LaunchArguments {
        program: "/tmp/cdebug/main".to_owned(),
        console: "internalConsole".to_owned(),
    };

    println!("launch: {:?}", client.launch(to_value(args)?).await);

    println!(
        "breakpoints: {:#?}",
        client
            .set_breakpoints(
                "/tmp/cdebug/main.c".into(),
                vec![SourceBreakpoint {
                    line: 6,
                    column: Some(2),
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                }]
            )
            .await
    );

    let mut _in = String::new();
    std::io::stdin()
        .read_line(&mut _in)
        .expect("Failed to read line");

    println!("configurationDone: {:?}", client.configuration_done().await);

    let mut _in = String::new();
    std::io::stdin()
        .read_line(&mut _in)
        .expect("Failed to read line");

    let threads = client.threads().await?;
    println!("threads: {:#?}", threads);
    let bt = client
        .stack_trace(threads[0].id)
        .await
        .expect("expected stack trace");
    println!("stack trace: {:#?}", bt);
    let scopes = client
        .scopes(bt.0[0].id)
        .await
        .expect("expected scopes for thread");
    println!("scopes: {:#?}", scopes);
    println!(
        "vars: {:#?}",
        client.variables(scopes[0].variables_reference).await
    );

    let mut _in = String::new();
    std::io::stdin()
        .read_line(&mut _in)
        .expect("Failed to read line");

    println!("continued: {:?}", client.continue_thread(0).await);

    let mut _in = String::new();
    std::io::stdin()
        .read_line(&mut _in)
        .expect("Failed to read line");

    println!("disconnect: {:?}", client.disconnect().await);

    Ok(())
}
