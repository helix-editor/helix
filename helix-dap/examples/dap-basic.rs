use helix_dap::{Client, Result, SourceBreakpoint};

#[tokio::main]
pub async fn main() -> Result<()> {
    let mut client = Client::start("nc", vec!["127.0.0.1", "7777"], 0)?;

    println!("init: {:?}", client.initialize().await);
    println!("caps: {:?}", client.capabilities());
    println!(
        "launch: {:?}",
        client.launch("/tmp/godebug/main".to_owned()).await
    );

    println!(
        "breakpoints: {:?}",
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

    let mut _in = String::new();
    std::io::stdin()
        .read_line(&mut _in)
        .expect("Failed to read line");

    println!("disconnect: {:?}", client.disconnect().await);

    Ok(())
}
