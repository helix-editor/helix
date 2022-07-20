use anyhow::{Context, Error, Result};
use crossterm::event::EventStream;
use helix_term::application::Application;
use helix_term::args::Args;
use helix_term::config::Config;
use std::path::PathBuf;

fn setup_logging(logpath: PathBuf, verbosity: u64) -> Result<()> {
    let mut base_config = fern::Dispatch::new();

    base_config = match verbosity {
        0 => base_config.level(log::LevelFilter::Warn),
        1 => base_config.level(log::LevelFilter::Info),
        2 => base_config.level(log::LevelFilter::Debug),
        _3_or_more => base_config.level(log::LevelFilter::Trace),
    };

    // Separate file config so we can include year, month and day in file logs
    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {} [{}] {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(fern::log_file(logpath)?);

    base_config.chain(file_config).apply()?;

    Ok(())
}

fn main() -> Result<()> {
    let exit_code = main_impl()?;
    std::process::exit(exit_code);
}

#[tokio::main]
async fn main_impl() -> Result<i32> {
    let logpath = helix_loader::log_file();
    let parent = logpath.parent().unwrap();
    if !parent.exists() {
        std::fs::create_dir_all(parent).ok();
    }

    let help = format!(
        "\
{} {}
{}
{}

USAGE:
    hx [FLAGS] [files]...

ARGS:
    <files>...    Sets the input file to use, position can also be specified via file[:row[:col]]

FLAGS:
    -h, --help                     Prints help information
    --tutor                        Loads the tutorial
    --health [LANG]                Checks for potential errors in editor setup
                                   If given, checks for config errors in language LANG
    -g, --grammar {{fetch|build}}    Fetches or builds tree-sitter grammars listed in languages.toml
    -v                             Increases logging verbosity each use for up to 3 times
                                   (default file: {})
    -V, --version                  Prints version information
    --vsplit                       Splits all given files vertically into different windows
    --hsplit                       Splits all given files horizontally into different windows
",
        env!("CARGO_PKG_NAME"),
        env!("VERSION_AND_GIT_HASH"),
        env!("CARGO_PKG_AUTHORS"),
        env!("CARGO_PKG_DESCRIPTION"),
        logpath.display(),
    );

    let args = Args::parse_args().context("could not parse arguments")?;

    // Help has a higher priority and should be handled separately.
    if args.display_help {
        print!("{}", help);
        std::process::exit(0);
    }

    if args.display_version {
        println!("helix {}", env!("VERSION_AND_GIT_HASH"));
        std::process::exit(0);
    }

    if args.health {
        if let Err(err) = helix_term::health::print_health(args.health_arg) {
            // Piping to for example `head -10` requires special handling:
            // https://stackoverflow.com/a/65760807/7115678
            if err.kind() != std::io::ErrorKind::BrokenPipe {
                return Err(err.into());
            }
        }

        std::process::exit(0);
    }

    if args.fetch_grammars {
        helix_loader::grammar::fetch_grammars()?;
        return Ok(0);
    }

    if args.build_grammars {
        helix_loader::grammar::build_grammars(None)?;
        return Ok(0);
    }

    setup_logging(logpath, args.verbosity).context("failed to initialize logging")?;

    let config_dir = helix_loader::config_dir();
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).ok();
    }

    let config = match std::fs::read_to_string(config_dir.join("config.toml")) {
        Ok(config) => toml::from_str(&config)
            .map(helix_term::keymap::merge_keys)
            .unwrap_or_else(|err| {
                eprintln!("Bad config: {}", err);
                eprintln!("Press <ENTER> to continue with default config");
                use std::io::Read;
                // This waits for an enter press.
                let _ = std::io::stdin().read(&mut []);
                Config::default()
            }),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(err) => return Err(Error::new(err)),
    };

    // TODO: use the thread local executor to spawn the application task separately from the work pool
    let mut app = Application::new(args, config).context("unable to create new application")?;

    let exit_code = app.run(&mut EventStream::new()).await?;

    Ok(exit_code)
}
