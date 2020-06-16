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

fn main() -> Result<(), Error> {
    let args: Args = argh::from_env();
    println!("{:?}", args.files);

    let mut editor = editor::Editor::new(args)?;
    editor.run();

    Ok(())
}
