mod position_request;

use anyhow::Result;
use helix_view::tree::Layout;
pub use position_request::PositionRequest;
use std::{iter::Peekable, path::PathBuf};

#[derive(Debug, Default)]
pub struct Args {
    pub display_help: bool,
    pub display_version: bool,
    pub health: bool,
    pub health_arg: Option<String>,
    pub load_tutor: bool,
    pub fetch_grammars: bool,
    pub build_grammars: bool,
    pub split: Option<Layout>,
    pub verbosity: u64,
    pub log_file: Option<PathBuf>,
    pub config_file: Option<PathBuf>,
    pub files: Vec<(PathBuf, PositionRequest)>,
}

impl Args {
    pub fn parse_args() -> Result<Args> {
        let mut argv = std::env::args().peekable();
        parse_args(&mut argv)
    }
}

pub fn parse_args(argv: &mut Peekable<impl Iterator<Item = String>>) -> Result<Args> {
    let mut args = Args::default();
    argv.next(); // skip the program, we don't care about that

    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--" => break, // stop parsing at this point treat the remaining as files
            "--version" => args.display_version = true,
            "--help" => args.display_help = true,
            "--tutor" => args.load_tutor = true,
            "--vsplit" => match args.split {
                Some(_) => anyhow::bail!("can only set a split once of a specific type"),
                None => args.split = Some(Layout::Vertical),
            },
            "--hsplit" => match args.split {
                Some(_) => anyhow::bail!("can only set a split once of a specific type"),
                None => args.split = Some(Layout::Horizontal),
            },
            "--health" => {
                args.health = true;
                args.health_arg = argv.next_if(|opt| !opt.starts_with('-'));
            }
            "-g" | "--grammar" => match argv.next().as_deref() {
                Some("fetch") => args.fetch_grammars = true,
                Some("build") => args.build_grammars = true,
                _ => {
                    anyhow::bail!("--grammar must be followed by either 'fetch' or 'build'")
                }
            },
            "-c" | "--config" => match argv.next().as_deref() {
                Some(path) => args.config_file = Some(path.into()),
                None => anyhow::bail!("--config must specify a path to read"),
            },
            "--log" => match argv.next().as_deref() {
                Some(path) => args.log_file = Some(path.into()),
                None => anyhow::bail!("--log must specify a path to write"),
            },
            arg if arg.starts_with("--") => {
                anyhow::bail!("unexpected double dash argument: {}", arg)
            }
            arg if arg.starts_with('-') => {
                let arg = arg.get(1..).unwrap().chars();
                for chr in arg {
                    match chr {
                        'v' => args.verbosity += 1,
                        'V' => args.display_version = true,
                        'h' => args.display_help = true,
                        _ => anyhow::bail!("unexpected short arg {}", chr),
                    }
                }
            }
            _ => {
                let file = PositionRequest::parse_positional_arg(arg, argv)?;
                args.files.push(file);
            }
        }
    }

    while let Some(arg) = argv.next() {
        let file = PositionRequest::parse_positional_arg(arg, argv)?;
        args.files.push(file);
    }

    Ok(args)
}
