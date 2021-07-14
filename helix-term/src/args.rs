use anyhow::{Error, Result};
use helix_core::Position;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Args {
    pub display_help: bool,
    pub display_version: bool,
    pub load_tutor: bool,
    pub verbosity: u64,
    pub files: Vec<(PathBuf, Position)>,
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
                arg => args.files.push(parse_file(arg)),
            }
        }

        // push the remaining args, if any to the files
        for arg in iter {
            args.files.push(parse_file(arg));
        }

        Ok(args)
    }
}

/// Parse arg into [`PathBuf`] and position.
pub(crate) fn parse_file(s: &str) -> (PathBuf, Position) {
    let def = || (PathBuf::from(s), Position::default());
    if Path::new(s).exists() {
        return def();
    }
    split_path_row_col(s)
        .or_else(|| split_path_row(s))
        .unwrap_or_else(def)
}

/// Split file.rs:10:2 into [`PathBuf`], row and col.
///
/// Does not validate if file.rs is a file or directory.
fn split_path_row_col(s: &str) -> Option<(PathBuf, Position)> {
    let mut s = s.rsplitn(3, ':');
    let col: usize = s.next()?.parse().ok()?;
    let row: usize = s.next()?.parse().ok()?;
    let path = s.next()?.into();
    let pos = Position::new(row.saturating_sub(1), col.saturating_sub(1));
    Some((path, pos))
}

/// Split file.rs:10 into [`PathBuf`] and row.
///
/// Does not validate if file.rs is a file or directory.
fn split_path_row(s: &str) -> Option<(PathBuf, Position)> {
    let (row, path) = s.rsplit_once(':')?;
    let row: usize = row.parse().ok()?;
    let path = path.into();
    let pos = Position::new(row.saturating_sub(1), 0);
    Some((path, pos))
}
