use helix_dap::{Client, Result, SourceBreakpoint};

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

    let mut client = Client::start("nc", vec!["127.0.0.1", "7777"], 0)?;

    println!("init: {:?}", client.initialize().await);
    println!("caps: {:#?}", client.capabilities());
    println!(
        "launch: {:?}",
        client.launch("/tmp/godebug/main".to_owned()).await
    );

    println!(
        "breakpoints: {:#?}",
        client
            .set_breakpoints(
                "/tmp/godebug/main.go".to_owned(),
                vec![SourceBreakpoint {
                    line: 6,
                    column: Some(2),
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
    println!("stack trace: {:#?}", client.stack_trace(1).await);

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
