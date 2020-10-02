#![allow(unused)]

mod editor;

use editor::Editor;

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

    smol::block_on(EX.run(async {
        editor::Editor::new(args).unwrap().run().await;
    }));

    Ok(())
}
