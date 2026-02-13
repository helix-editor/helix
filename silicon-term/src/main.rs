use anyhow::{Context, Result};
use silicon_loader::VERSION_AND_GIT_HASH;
use silicon_term::application::Application;
use silicon_term::args::Args;
use silicon_term::config::Config;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
        .chain(fern::log_file(silicon_loader::log_file())?);

    base_config.chain(file_config).apply()?;

    Ok(())
}

fn main() -> Result<()> {
    let exit_code = main_impl()?;
    std::process::exit(exit_code);
}

#[tokio::main]
async fn main_impl() -> Result<i32> {
    let args = Args::parse_args().context("could not parse arguments")?;

    silicon_loader::initialize_config_file(args.config_file.clone());
    silicon_loader::initialize_log_file(args.log_file.clone());

    // Help has a higher priority and should be handled separately.
    if args.display_help {
        print!(
            "\
{} {}
{}
{}

USAGE:
    si [FLAGS] [files]...

ARGS:
    <files>...    Set the input file to use, position can also be specified via file[:row[:col]]

FLAGS:
    -h, --help                     Print help information
    --tutor                        Load the tutorial
    --health [CATEGORY]            Check for potential errors in editor setup
                                   CATEGORY can be a language or one of 'clipboard', 'languages',
                                   'all-languages' or 'all'. 'languages' is filtered according to
                                   user config, 'all-languages' and 'all' are not. If not specified,
                                   the default is the same as 'all', but with languages filtering.
    -g, --grammar {{fetch|build}}    Fetch or builds tree-sitter grammars listed in languages.toml
    -c, --config <file>            Specify a file to use for configuration
    -v                             Increase logging verbosity each use for up to 3 times
    --log <file>                   Specify a file to use for logging
                                   (default file: {})
    -V, --version                  Print version information
    --vsplit                       Split all given files vertically into different windows
    --hsplit                       Split all given files horizontally into different windows
    -w, --working-dir <path>       Specify an initial working directory
    +[N]                           Open the first given file at line number N, or the last line, if
                                   N is not specified.
",
            env!("CARGO_PKG_NAME"),
            VERSION_AND_GIT_HASH,
            env!("CARGO_PKG_AUTHORS"),
            env!("CARGO_PKG_DESCRIPTION"),
            silicon_loader::default_log_file().display(),
        );
        std::process::exit(0);
    }

    if args.display_version {
        println!("silicon {}", VERSION_AND_GIT_HASH);
        std::process::exit(0);
    }

    if args.health {
        if let Err(err) = silicon_term::health::print_health(args.health_arg) {
            // Piping to for example `head -10` requires special handling:
            // https://stackoverflow.com/a/65760807/7115678
            if err.kind() != std::io::ErrorKind::BrokenPipe {
                return Err(err.into());
            }
        }

        std::process::exit(0);
    }

    if args.fetch_grammars {
        silicon_loader::grammar::fetch_grammars()?;
        return Ok(0);
    }

    if args.build_grammars {
        silicon_loader::grammar::build_grammars(None)?;
        return Ok(0);
    }

    setup_logging(args.verbosity).context("failed to initialize logging")?;

    // NOTE: Set the working directory early so the correct configuration is loaded. Be aware that
    // Application::new() depends on this logic so it must be updated if this changes.
    if let Some(path) = &args.working_directory {
        silicon_stdx::env::set_current_working_dir(path)?;
    } else if let Some((path, _)) = args.files.first().filter(|p| p.0.is_dir()) {
        // If the first file is a directory, it will be the working directory unless -w was specified
        silicon_stdx::env::set_current_working_dir(path)?;
    }

    let config = match silicon_lua::load_config_default() {
        Ok(lua_config) => Config::from_lua(lua_config),
        Err(silicon_lua::LuaConfigError::NotFound) => Config::default(),
        Err(silicon_lua::LuaConfigError::TomlDetected(_)) => {
            log::warn!("TOML config detected. Lua config expected. Run :migrate-config");
            Config::default()
        }
        Err(e) => {
            eprintln!("Config error: {e}");
            eprintln!("Press <ENTER> to continue with default config");
            use std::io::Read;
            let _ = std::io::stdin().read(&mut [0u8]);
            Config::default()
        }
    };

    let lang_loader = silicon_core::config::user_lang_loader_with_overrides(
        config.language_config.clone(),
    )
    .unwrap_or_else(|err| {
        eprintln!("{}", err);
        eprintln!("Press <ENTER> to continue with default language config");
        use std::io::Read;
        // This waits for an enter press.
        let _ = std::io::stdin().read(&mut []);
        silicon_core::config::default_lang_loader()
    });

    // TODO: use the thread local executor to spawn the application task separately from the work pool
    let mut app = Application::new(args, config, lang_loader).context("unable to start Silicon")?;
    let mut events = app.event_stream();

    let exit_code = app.run(&mut events).await?;

    Ok(exit_code)
}
