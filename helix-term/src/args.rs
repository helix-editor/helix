use anyhow::{anyhow, Result};
use clap::builder::PossibleValue;
use helix_core::Position;
use helix_loader::VERSION_AND_GIT_HASH;
use helix_view::tree::Layout;
use std::{convert::Infallible, path::PathBuf, str::FromStr};

#[derive(clap::Parser)]
#[command(author, version = VERSION_AND_GIT_HASH, about, long_about = None)]
pub struct Args {
    #[arg(
        long,
        default_missing_value = "all",
        value_names = ["CATEGORY"],
        help = "Checks for potential errors in editor setup. CATEGORY can be a language or one of 'clipboard', 'languages' or 'all'. 'all' is the default if not specified."
    )]
    pub health: Option<HealthCheckCategory>,
    #[arg(long, help = "Loads the tutorial")]
    pub tutor: bool,
    #[arg(
        value_enum,
        long = "grammar",
        value_names = ["ACTION"],
        help = "Fetches or builds tree-sitter grammars listed in languages.toml"
    )]
    pub grammar_action: Option<GrammarsAction>,
    #[arg(
        value_enum,
        long,
        value_names = ["LAYOUT"],
        help = "Splits all given files vertically or horizontally into different windows"
    )]
    pub split: Option<Split>,
    #[arg(short, long, default_value = "0", help = "Sets logging verbosity")]
    pub verbosity: u64,
    #[arg(long = "log", help = "Specifies a file to use for logging")]
    pub log_file: Option<PathBuf>,
    #[arg(
        long = "config",
        short = 'c',
        help = "Specifies a file to use for configuration"
    )]
    pub config_file: Option<PathBuf>,
    #[arg(
        long = "working-dir",
        short,
        help = "Specify an initial working directory"
    )]
    pub working_directory: Option<PathBuf>,
    #[arg(
        help = "Sets the input file to use, position can also be specified via file[:row[:col]]"
    )]
    pub files: Vec<FileWithPosition>,
}

#[derive(Clone)]
pub enum HealthCheckCategory {
    All,
    Specified(String),
}

impl FromStr for HealthCheckCategory {
    type Err = Infallible;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        if arg == "all" {
            return Ok(Self::All);
        }

        Ok(Self::Specified(arg.to_string()))
    }
}

#[derive(clap::ValueEnum, Clone)]
pub enum GrammarsAction {
    Fetch,
    Build,
}

#[derive(Clone, Copy)]
pub struct Split(pub Layout);

impl clap::ValueEnum for Split {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self(Layout::Vertical), Self(Layout::Horizontal)]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        let value = match self.0 {
            Layout::Horizontal => "horizontal",
            Layout::Vertical => "vertical",
        };

        Some(PossibleValue::new(value))
    }
}

#[derive(Clone)]
pub struct FileWithPosition {
    pub path: PathBuf,
    pub position: Position,
}

impl FromStr for FileWithPosition {
    type Err = anyhow::Error;

    fn from_str(arg: &str) -> Result<Self> {
        const FILE_POSITION_DELIMETER: char = ':';

        let mut arg = arg.split(FILE_POSITION_DELIMETER);
        let file = PathBuf::from(arg.next().unwrap());

        let mut next_position_number = || arg.next().map(str::parse).unwrap_or(Ok(0));
        let position = {
            let row = next_position_number().map_err(|_| anyhow!("Invalid row"))?;
            let column = next_position_number().map_err(|_| anyhow!("Invalid column"))?;

            Position::new(row, column)
        };

        Ok(Self {
            path: file,
            position,
        })
    }
}
