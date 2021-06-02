#![allow(unused)]

mod application;
mod commands;
mod compositor;
mod keymap;
mod ui;

use application::Application;

use helix_core::config_dir;

use std::path::PathBuf;

use anyhow::{Context, Result};

fn setup_logging(verbosity: u64) -> Result<()> {
    let mut base_config = fern::Dispatch::new();

    // Let's say we depend on something which whose "info" level messages are too
    // verbose to include in end-user output. If we don't need them,
    // let's not include them.
    // .level_for("overly-verbose-target", log::LevelFilter::Warn)

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
        .chain(fern::log_file(config_dir().join("helix.log"))?);

    base_config.chain(file_config).apply()?;

    Ok(())
}

pub struct Args {
    files: Vec<PathBuf>,
}

fn main() -> Result<()> {
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
    -v               Increases logging verbosity each use for up to 3 times
    -V, --version    Prints version information
",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS"),
        env!("CARGO_PKG_DESCRIPTION"),
    );

    let mut pargs = pico_args::Arguments::from_env();

    // Help has a higher priority and should be handled separately.
    if pargs.contains(["-h", "--help"]) {
        print!("{}", help);
        std::process::exit(0);
    }

    let mut verbosity: u64 = 0;

    while pargs.contains("-v") {
        verbosity += 1;
    }

    let conf_dir = config_dir();

    if !conf_dir.exists() {
        std::fs::create_dir(&conf_dir);
    }

    setup_logging(verbosity).context("failed to initialize logging")?;

    let args = Args {
        files: pargs.finish().into_iter().map(|arg| arg.into()).collect(),
    };

    // initialize language registry
    use helix_core::syntax::{Loader, LOADER};

    // load $HOME/.config/helix/languages.toml, fallback to default config
    let config = std::fs::read(config_dir().join("languages.toml"));
    let toml = config
        .as_deref()
        .unwrap_or(include_bytes!("../../languages.toml"));

    let config = toml::from_slice(toml).context("Could not parse languages.toml")?;
    LOADER.get_or_init(|| Loader::new(config));

    let runtime = tokio::runtime::Runtime::new().context("unable to start tokio runtime")?;

    // TODO: use the thread local executor to spawn the application task separately from the work pool
    let mut app = Application::new(args).context("unable to create new appliction")?;
    runtime.block_on(async move {
        app.run().await;
    });

    Ok(())
}
