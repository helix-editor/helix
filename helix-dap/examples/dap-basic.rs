use helix_dap::{Client, Result, SourceBreakpoint};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchArguments {
    mode: String,
    program: String,
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

    let client = Client::tcp("127.0.0.1:7777".parse::<std::net::SocketAddr>().unwrap(), 0).await;
    println!("create: {:?}", client);
    let mut client = client?;

    println!("init: {:?}", client.initialize("go".to_owned()).await);
    println!("caps: {:#?}", client.capabilities());

    let args = LaunchArguments {
        mode: "exec".to_owned(),
        program: "/tmp/godebug/main".to_owned(),
    };

    println!("launch: {:?}", client.launch(args).await);

    println!(
        "breakpoints: {:#?}",
        client
            .set_breakpoints(
                "/tmp/godebug/main.go".to_owned(),
                vec![SourceBreakpoint {
                    line: 8,
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
    println!("stopped: {:?}", client.wait_for_stopped().await);
    println!("threads: {:#?}", client.threads().await);
    let bt = client.stack_trace(1).await.expect("expected stack trace");
    println!("stack trace: {:#?}", bt);
    let scopes = client
        .scopes(bt.0[0].id)
        .await
        .expect("expected scopes for thread");
    println!("scopes: {:#?}", scopes);
    println!(
        "vars: {:#?}",
        client.variables(scopes[1].variables_reference).await
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
