use anyhow::{Context, Error, Result};
use helix_term::application::Application;
use helix_term::args::Args;
use helix_term::config::Config;
use helix_term::keymap::merge_keys;
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
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(logpath)?;
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
        .chain(file);

    base_config.chain(file_config).apply()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cache_dir = helix_core::cache_dir();
    if !cache_dir.exists() {
        std::fs::create_dir_all(&cache_dir).ok();
    }

    let logpath = cache_dir.join("helix.log");
    let help = format!(
        "\
{} {}
{}
{}

USAGE:
    hx [FLAGS] [files]...

ARGS:
    <files>...    Sets the input file to use

FLAGS:
    -h, --help       Prints help information
    --tutor          Loads the tutorial
    -v               Increases logging verbosity each use for up to 3 times
                     (default file: {})
    -V, --version    Prints version information
",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
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
        println!("helix {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    let conf_dir = helix_core::config_dir();
    if !conf_dir.exists() {
        std::fs::create_dir_all(&conf_dir).ok();
    }

    let config = match std::fs::read_to_string(conf_dir.join("config.toml")) {
        Ok(config) => merge_keys(toml::from_str(&config)?),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(err) => return Err(Error::new(err)),
    };

    setup_logging(logpath, args.verbosity).context("failed to initialize logging")?;

    // TODO: use the thread local executor to spawn the application task separately from the work pool
    let mut app = Application::new(args, config).context("unable to create new application")?;
    app.run().await.unwrap();

    Ok(())
}
