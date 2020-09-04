#![allow(unused)]
// mod editor;
// mod component;
mod editor;
mod keymap;

use editor::Editor;

use argh::FromArgs;
use std::path::PathBuf;

use anyhow::Error;

#[derive(FromArgs)]
/// A post-modern text editor.
pub struct Args {
    #[argh(positional)]
    files: Vec<PathBuf>,
}

static EX: smol::Executor = smol::Executor::new();

fn main() -> Result<(), Error> {
    let args: Args = argh::from_env();
    println!("{:?}", args.files);

    for _ in 0..num_cpus::get() {
        std::thread::spawn(move || smol::block_on(EX.run(smol::future::pending::<()>())));
    }

    smol::block_on(EX.run(async {
        editor::Editor::new(args).unwrap().run().await;
    }));

    Ok(())
}
