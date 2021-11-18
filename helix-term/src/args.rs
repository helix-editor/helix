use anyhow::{Error, Result};
use std::path::PathBuf;

#[derive(Default)]
pub struct Args {
    pub display_help: bool,
    pub display_version: bool,
    pub load_tutor: bool,
    pub verbosity: u64,
    pub files: Vec<PathBuf>,
    pub positions: Vec<Option<(usize, Option<usize>)>>,
}

impl Args {
    pub fn parse_args() -> Result<Args> {
        let mut args = Args::default();
        let argv: Vec<String> = std::env::args().collect();
        let mut iter = argv.iter();

        iter.next(); // skip the program, we don't care about that

        let mut position: Option<(usize, Option<usize>)> = None;
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
                arg if arg.starts_with('+') => {
                    let mut line_col: Vec<usize> = vec![];
                    for number_str in arg[1..].split(':') {
                        match number_str.parse() {
                            Ok(number) => line_col.push(number),
                            Err(_) => {
                                return Err(Error::msg(format!(
                                    "parsing {} expected number, actual {}",
                                    arg, number_str
                                )));
                            }
                        }
                    }
                    match line_col.len() {
                        1 => {
                            position = Some((line_col[0], None));
                        }
                        2 => {
                            position = Some((line_col[0], Some(line_col[1])));
                        }
                        _ => {
                            return Err(Error::msg(format!(
                                "expected +<line>:<column>, actual {}",
                                arg
                            )))
                        }
                    }
                }
                arg => {
                    args.files.push(PathBuf::from(arg));
                    args.positions.push(position);
                    position = None;
                }
            }
        }

        // push the remaining args, if any to the files
        for filename in iter {
            args.files.push(PathBuf::from(filename));
            args.positions.push(position);
        }

        Ok(args)
    }
}
