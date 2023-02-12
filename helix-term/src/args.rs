use std::{borrow::Cow, iter::Peekable, path::PathBuf};

use anyhow::Result;
use helix_core::{pos_at_coords, Position, Selection};
use helix_view::{tree::Layout, Document};

/// As files are parsed from the CLI they can either contain an explicit position or a request to
/// jump to the end of the file. An explicit position can be declared in either prefix or postfix
/// notation.
///
/// Prefix notation to open a file at line 10 `hx +10 Cargo.toml`.
/// Postfix notation to open a file at line 10 `hx Cargo.toml:10`.
///
/// Both notation can also be used to place there cursor at the last line / Eof.
///
/// `hx +: Cargo.toml`
/// `hx Cargo.toml:`
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PositionRequest {
    /// Set the file cursor to the given position,
    Explicit(Position),
    /// Set the file cursor to the last line when opened
    Eof,
}

impl Default for PositionRequest {
    fn default() -> Self {
        PositionRequest::Explicit(Position::default())
    }
}

impl PositionRequest {
    /// Return a Selection based on this PositionRequest.
    pub(crate) fn selection_for_doc(self, doc: &Document) -> Selection {
        let text = doc.text().slice(..);
        match self {
            PositionRequest::Explicit(pos) => {
                let pos = pos_at_coords(text, pos, true);
                Selection::point(pos)
            }
            PositionRequest::Eof => {
                let line_idx = if text.line(text.len_lines() - 1).len_chars() == 0 {
                    // If the last line is blank, don't jump to it.
                    text.len_lines().saturating_sub(2)
                } else {
                    text.len_lines() - 1
                };
                let pos = text.line_to_char(line_idx);
                Selection::point(pos)
            }
        }
    }
}

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

fn parse_args(argv: &mut Peekable<impl Iterator<Item = String>>) -> Result<Args> {
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
                let file = parse_positional_arg(arg, argv)?;
                args.files.push(file);
            }
        }
    }

    // push the remaining args, if any to the files
    while let Some(arg) = argv.next() {
        let file = parse_positional_arg(arg, argv)?;
        args.files.push(file);
    }

    Ok(args)
}

/// Parse a positional arg. All of which are expected to be file related.
/// If an arg is a prefixed file position, then the next arg is expected to be a file.
/// File paths are not validated, that's left to the consumer.
pub(crate) fn parse_positional_arg(
    arg: String,
    argv: &mut impl Iterator<Item = String>,
) -> Result<(PathBuf, PositionRequest)> {
    let file = if let Some(s) = arg.strip_prefix('+') {
        let prefix_pos = parse_file_position(s);
        let (path, postfix_pos) = match argv.next() {
            Some(file) => parse_file(file),
            None => anyhow::bail!("expected a file after a position"),
        };

        if postfix_pos.is_some() {
            anyhow::bail!("unexpected postfix position after prefix position");
        }

        (path, prefix_pos.unwrap_or_default())
    } else {
        let (path, pos) = parse_file(arg);
        (path, pos.unwrap_or_default())
    };

    Ok(file)
}

pub(crate) fn parse_file_position(s: &str) -> Option<PositionRequest> {
    if s.chars().all(|c| c == ':') {
        return Some(PositionRequest::Eof);
    }

    let mut s = s.splitn(2, ':');
    let row: usize = s.next()?.parse().ok()?;
    let col: usize = s.next().and_then(|x| x.parse().ok()).unwrap_or_default();

    let pos = Position::new(row.saturating_sub(1), col.saturating_sub(1));
    Some(PositionRequest::Explicit(pos))
}

/// Parse arg into [`PathBuf`] and position.
pub(crate) fn parse_file<'a>(s: impl Into<Cow<'a, str>>) -> (PathBuf, Option<PositionRequest>) {
    let s = s.into();
    match s.split_once(':') {
        Some((s, rest)) => (s.into(), parse_file_position(rest)),
        None => (s.into_owned().into(), None),
    }
}

#[cfg(test)]
mod tests {
    use std::{iter::Peekable, path::PathBuf, str::FromStr};

    use assert_matches::assert_matches;

    use helix_core::Position;

    use super::{parse_args, parse_file, parse_file_position, PositionRequest};

    #[test]
    fn should_parse_file() {
        assert_matches!(
            parse_file(""),
            (path, None) if path == PathBuf::from_str("").unwrap()
        );

        assert_matches!(
            parse_file(":"),
            (path, Some(PositionRequest::Eof)) if path == PathBuf::from_str("").unwrap()
        );

        assert_matches!(
            parse_file("file"),
            (path, None) if path == PathBuf::from_str("file").unwrap()
        );

        assert_matches!(
            parse_file("file:"),
            (path, Some(PositionRequest::Eof)) if path == PathBuf::from_str("file").unwrap()
        );

        assert_matches!(
            parse_file("file:10"),
            (path, Some(PositionRequest::Explicit(Position { row: 9, col: 0 })))
                if path == PathBuf::from_str("file").unwrap()
        );

        assert_matches!(
            parse_file("file:10:"),
            (path, Some(PositionRequest::Explicit(Position { row: 9, col: 0 })))
                if path == PathBuf::from_str("file").unwrap()
        );

        assert_matches!(
            parse_file("file:10:20"),
            (path, Some(PositionRequest::Explicit(Position { row: 9, col: 19 })))
                if path == PathBuf::from_str("file").unwrap()
        );
    }

    #[test]
    fn should_parse_file_position() {
        assert_matches!(parse_file_position(":"), Some(PositionRequest::Eof));
        assert_matches!(parse_file_position("::"), Some(PositionRequest::Eof));
        assert_matches!(
            parse_file_position("10"),
            Some(PositionRequest::Explicit(Position { row: 9, col: 0 }))
        );
        assert_matches!(
            parse_file_position("10:"),
            Some(PositionRequest::Explicit(Position { row: 9, col: 0 }))
        );
        assert_matches!(
            parse_file_position("10:20"),
            Some(PositionRequest::Explicit(Position { row: 9, col: 19 }))
        );
        assert_matches!(parse_file_position("x"), None);
        assert_matches!(parse_file_position("x:y"), None);
    }

    #[test]
    fn should_parse_positional_args() {
        let args = parse_args(&mut str_to_argv("hx Cargo.toml")).unwrap();
        assert_eq!(
            args.files[0],
            (
                PathBuf::from_str("Cargo.toml").unwrap(),
                PositionRequest::Explicit(Position::default())
            )
        );

        let args = parse_args(&mut str_to_argv("hx +10 Cargo.toml")).unwrap();
        assert_eq!(
            args.files[0],
            (
                PathBuf::from_str("Cargo.toml").unwrap(),
                PositionRequest::Explicit(Position { row: 9, col: 0 })
            )
        );

        let args = parse_args(&mut str_to_argv("hx +: Cargo.toml")).unwrap();
        assert_eq!(
            args.files[0],
            (
                PathBuf::from_str("Cargo.toml").unwrap(),
                PositionRequest::Eof
            )
        );

        let args = parse_args(&mut str_to_argv("hx Cargo.toml:")).unwrap();
        assert_eq!(
            args.files[0],
            (
                PathBuf::from_str("Cargo.toml").unwrap(),
                PositionRequest::Eof
            )
        );

        parse_args(&mut str_to_argv("hx +10")).unwrap_err();
        parse_args(&mut str_to_argv("hx +10 Cargo.toml +20")).unwrap_err();
        parse_args(&mut str_to_argv("hx +10 Cargo.toml:20")).unwrap_err();
        parse_args(&mut str_to_argv("hx +10 Cargo.toml:")).unwrap_err();

        let args = parse_args(&mut str_to_argv("hx +10 Cargo.toml +20 README")).unwrap();
        assert_eq!(
            args.files[0],
            (
                PathBuf::from_str("Cargo.toml").unwrap(),
                PositionRequest::Explicit(Position { row: 9, col: 0 })
            )
        );
        assert_eq!(
            args.files[1],
            (
                PathBuf::from_str("README").unwrap(),
                PositionRequest::Explicit(Position { row: 19, col: 0 })
            )
        );

        let args = parse_args(&mut str_to_argv("hx -- Cargo.toml")).unwrap();
        assert_eq!(
            args.files[0],
            (
                PathBuf::from_str("Cargo.toml").unwrap(),
                PositionRequest::Explicit(Position::default())
            )
        );

        let args =
            parse_args(&mut str_to_argv("hx --vsplit -- +10 Cargo.toml +20 README")).unwrap();
        assert_eq!(args.split, Some(helix_view::tree::Layout::Vertical));
        assert_eq!(
            args.files[0],
            (
                PathBuf::from_str("Cargo.toml").unwrap(),
                PositionRequest::Explicit(Position { row: 9, col: 0 })
            )
        );
        assert_eq!(
            args.files[1],
            (
                PathBuf::from_str("README").unwrap(),
                PositionRequest::Explicit(Position { row: 19, col: 0 })
            )
        );
    }

    #[test]
    fn should_parse_config() {
        let args = parse_args(&mut str_to_argv("hx --config other/config.toml")).unwrap();
        assert_eq!(
            args.config_file,
            Some(PathBuf::from_str("other/config.toml").unwrap())
        );
    }

    #[test]
    fn should_parse_layout() {
        let args = parse_args(&mut str_to_argv("hx --vsplit Cargo.toml")).unwrap();
        assert_eq!(args.split, Some(helix_view::tree::Layout::Vertical));

        let args = parse_args(&mut str_to_argv("hx --hsplit Cargo.toml")).unwrap();
        assert_eq!(args.split, Some(helix_view::tree::Layout::Horizontal));

        parse_args(&mut str_to_argv("hx --hsplit -vsplit Cargo.toml")).unwrap_err();
    }

    /// Return a peekable Iterator of arguments naively split on whitespace
    fn str_to_argv(s: &'static str) -> Peekable<impl Iterator<Item = String>> {
        s.split_whitespace().map(ToOwned::to_owned).peekable()
    }
}
