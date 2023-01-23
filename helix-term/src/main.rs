mod help;
use anyhow::{Context, Result};
use crossterm::event::EventStream;
use helix_core::syntax::LanguageConfigurations;
use helix_loader::VERSION_AND_GIT_HASH;
use helix_term::application::Application;
use helix_term::args::Args;
use helix_term::config::Config;
use std::path::PathBuf;

fn main() -> Result<()> {
    let exit_code = main_impl()?;
    std::process::exit(exit_code);
}

#[tokio::main]
async fn main_impl() -> Result<i32> {
    let args = Args::parse_args().context("failed to parse arguments")?;
    setup_logging(args.log_file.clone(), args.verbosity).context("failed to initialize logging")?;

    if args.display_help {
        print!("{}", help::help());
        return Ok(0);
    }
    if args.display_version {
        println!("helix {}", VERSION_AND_GIT_HASH);
        return Ok(0);
    }
    if args.health {
        if let Err(err) = helix_term::health::print_health(args.health_arg) {
            // Piping to for example `head -10` requires special handling:
            // https://stackoverflow.com/a/65760807/7115678
            if err.kind() != std::io::ErrorKind::BrokenPipe {
                return Err(err.into());
            }
        }
        return Ok(0);
    }
    if args.fetch_grammars {
        helix_loader::grammar::fetch_grammars()?;
        return Ok(0);
    }
    if args.build_grammars {
        helix_loader::grammar::build_grammars(None)?;
        return Ok(0);
    }

    helix_loader::setup_config_file(args.config_file.clone());
    let mut config = Config::merged().unwrap_or_else(|err| {
        eprintln!("Bad config: {}", err);
        eprintln!("Press <ENTER> to continue with default config");
        let _wait_for_enter = std::io::Read::read(&mut std::io::stdin(), &mut[]);
        Config::default()
    });
    if config.editor.load_local_config {
        // NOTE: deserializes user config once again
        config = Config::merged_local_config().unwrap_or_else(|err| {
            eprintln!("Bad local config: {}", err);
            eprintln!("Press <ENTER> to continue with default and user config");
            let _wait_for_enter = std::io::Read::read(&mut std::io::stdin(), &mut[]);
            config
        });
    }

    let language_configurations = LanguageConfigurations::merged().unwrap_or_else(|err| {
        eprintln!("Bad language config: {}", err);
        eprintln!("Press <ENTER> to continue with default language config");
        let _wait_for_enter = std::io::Read::read(&mut std::io::stdin(), &mut[]);
        LanguageConfigurations::default()
    });

    // TODO: use the thread local executor to spawn the application task separately from the work pool
    let mut app = Application::new(args, config, language_configurations)
        .context("unable to create new application")?;
    let exit_code = app.run(&mut EventStream::new()).await?;
    Ok(exit_code)
}

fn setup_logging(logpath: Option<PathBuf>, verbosity: u64) -> Result<()> {
    helix_loader::setup_log_file(logpath); 

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
