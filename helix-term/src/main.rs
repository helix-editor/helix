#![allow(unused)]

mod application;

use application::Application;

use clap::{App, Arg};
use std::path::PathBuf;

use anyhow::Error;

static EX: smol::Executor = smol::Executor::new();

fn main() -> Result<(), Error> {
    let args = clap::app_from_crate!()
        .arg(
            Arg::new("files")
                .about("Sets the input file to use")
                .required(true)
                .multiple(true)
                .index(1),
        )
        .get_matches();

    for _ in 0..num_cpus::get() {
        std::thread::spawn(move || smol::block_on(EX.run(smol::future::pending::<()>())));
    }

    // let mut lsp = helix_lsp::Client::start(&EX, "rust-analyzer", &[]);

    smol::block_on(async {
        // let res = lsp.initialize().await;
        // let state = helix_core::State::load("test.rs".into(), &[]).unwrap();
        // let res = lsp.text_document_did_open(&state).await;
        // loop {}

        Application::new(args, &EX).unwrap().run().await;
    });

    Ok(())
}
