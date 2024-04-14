use anyhow::{Context, Error, Result};
use clap::Parser;
use crossterm::event::EventStream;
use helix_term::application::Application;
use helix_term::args::{Args, FileWithPosition, GrammarsAction};
use helix_term::config::{Config, ConfigLoadError};

fn setup_logging(verbosity: u64) -> Result<()> {
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
        .chain(fern::log_file(helix_loader::log_file())?);

    base_config.chain(file_config).apply()?;

    Ok(())
}

fn main() -> Result<()> {
    let exit_code = main_impl()?;
    std::process::exit(exit_code);
}

#[tokio::main]
async fn main_impl() -> Result<i32> {
    let mut args = Args::parse();

    helix_loader::initialize_config_file(args.config_file.clone());
    helix_loader::initialize_log_file(args.log_file.clone());

    if let Some(health) = args.health {
        if let Err(err) = helix_term::health::print_health(health) {
            // Piping to for example `head -10` requires special handling:
            // https://stackoverflow.com/a/65760807/7115678
            if err.kind() != std::io::ErrorKind::BrokenPipe {
                return Err(err.into());
            }
        }

        std::process::exit(0);
    }

    if let Some(grammar_action) = args.grammar_action {
        match grammar_action {
            GrammarsAction::Fetch => helix_loader::grammar::fetch_grammars()?,
            GrammarsAction::Build => helix_loader::grammar::build_grammars(None)?,
        }

        return Ok(0);
    }
    setup_logging(args.verbosity).context("failed to initialize logging")?;

    // Before setting the working directory, resolve all the paths in args.files
    for FileWithPosition { path, .. } in args.files.iter_mut() {
        *path = helix_stdx::path::canonicalize(&path);
    }

    // NOTE: Set the working directory early so the correct configuration is loaded. Be aware that
    // Application::new() depends on this logic so it must be updated if this changes.
    if let Some(path) = &args.working_directory {
        helix_stdx::env::set_current_working_dir(path)?;
    } else if let Some(FileWithPosition { path, .. }) =
        args.files.first().filter(|f| f.path.is_dir())
    {
        // If the first file is a directory, it will be the working directory unless -w was specified
        helix_stdx::env::set_current_working_dir(path)?;
    }

    let config = match Config::load_default() {
        Ok(config) => config,
        Err(ConfigLoadError::Error(err)) if err.kind() == std::io::ErrorKind::NotFound => {
            Config::default()
        }
        Err(ConfigLoadError::Error(err)) => return Err(Error::new(err)),
        Err(ConfigLoadError::BadConfig(err)) => {
            eprintln!("Bad config: {}", err);
            eprintln!("Press <ENTER> to continue with default config");
            use std::io::Read;
            let _ = std::io::stdin().read(&mut []);
            Config::default()
        }
    };

    let lang_loader = helix_core::config::user_lang_loader().unwrap_or_else(|err| {
        eprintln!("{}", err);
        eprintln!("Press <ENTER> to continue with default language config");
        use std::io::Read;
        // This waits for an enter press.
        let _ = std::io::stdin().read(&mut []);
        helix_core::config::default_lang_loader()
    });

    // TODO: use the thread local executor to spawn the application task separately from the work pool
    let mut app =
        Application::new(args, config, lang_loader).context("unable to create new application")?;

    let exit_code = app.run(&mut EventStream::new()).await?;

    Ok(exit_code)
}
