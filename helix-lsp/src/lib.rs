use std::{
    collections::HashMap,
    process::{ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
};

use smol::io::{BufReader, BufWriter};
use smol::prelude::*;
use smol::Unblock;

struct Client {
    // process: Command,
    reader: BufReader<Unblock<ChildStdout>>,
}

impl Client {
    fn start(cmd: &str, args: &[String]) -> Self {
        let mut process = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start language server");
        // TODO: impl drop that kills the process

        // TODO: do we need bufreader/writer here? or do we use async wrappers on unblock?
        let writer = BufWriter::new(Unblock::new(
            process.stdin.take().expect("Failed to open stdin"),
        ));
        let reader = BufReader::new(Unblock::new(
            process.stdout.take().expect("Failed to open stdout"),
        ));
        let stderr = BufReader::new(Unblock::new(
            process.stderr.take().expect("Failed to open stderr"),
        ));

        Client { reader }
    }

    async fn receiver(&mut self) -> Result<(), std::io::Error> {
        let mut headers: HashMap<String, String> = HashMap::default();
        loop {
            // read headers
            loop {
                let mut header = String::new();
                // detect pipe closed if 0
                self.reader.read_line(&mut header).await?;
                let header = header.trim();

                if header.is_empty() {
                    break;
                }

                let parts: Vec<&str> = header.split(": ").collect();
                if parts.len() != 2 {
                    // return Err(Error::new(ErrorKind::Other, "Failed to parse header"));
                    panic!()
                }
                headers.insert(parts[0].to_string(), parts[1].to_string());
            }

            // find content-length

            // read data
            // decode via serde_json decoding into jsonrpc_core Output
            break;
        }

        Ok(())
    }
}
