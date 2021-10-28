use anyhow::{Error, Result};
use std::path::PathBuf;

#[derive(Default)]
pub struct Args {
    pub display_help: bool,
    pub display_version: bool,
    pub load_tutor: bool,
    pub verbosity: u64,
    pub files: Vec<PathBuf>,
}

impl Args {
    pub fn parse_args() -> Result<Args> {
        let mut args = Args::default();
        let argv: Vec<String> = std::env::args().collect();
        let mut iter = argv.iter();

        iter.next(); // skip the program, we don't care about that

        for arg in &mut iter {
            match arg.as_str() {
                "--" => break, // stop parsing at this point treat the remaining as files
                "--version" => args.display_version = true,
                "--help" => args.display_help = true,
                "--tutor" => args.load_tutor = true,
                arg if arg.starts_with("--") => {
                    return Err(Error::msg(format!(
                        "unexpected double dash argument: {}",
                        arg
                    )))
                }
                arg if arg.starts_with('-') => {
                    let arg = arg.get(1..).unwrap().chars();
                    for chr in arg {
                        match chr {
                            'v' => args.verbosity += 1,
                            'V' => args.display_version = true,
                            'h' => args.display_help = true,
                            _ => return Err(Error::msg(format!("unexpected short arg {}", chr))),
                        }
                    }
                }
                arg => args.files.push(PathBuf::from(arg)),
            }
        }

        // push the remaining args, if any to the files
        for filename in iter {
            args.files.push(PathBuf::from(filename));
        }

        Ok(args)
    }
}
