mod help;

use anyhow::{Context, Result};
use crossterm::event::EventStream;
use helix_core::syntax::LanguageConfigurations;
use helix_loader::VERSION_AND_GIT_HASH;
use helix_term::application::Application;
use helix_term::args::Args;
use helix_term::config::Config;
use helix_view::Theme;
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

    let mut config = check_config_load(Config::merged(), None, "");
    if config.editor.load_local_config {
        // NOTE: deserializes user config once again
        config = check_config_load(Config::merged_local_config(), Some(config), "");
    }
    let language_configurations =
        check_config_load(LanguageConfigurations::merged(), None, "language");

    let true_color_support = {
        config.editor.true_color || {
            if cfg!(windows) {
                true
            } else {
                std::env::var("COLORTERM")
                    .map(|v| matches!(v.as_str(), "truecolor" | "24bit"))
                    .unwrap_or(false)
            }
        }
    };

    Theme::set_true_color_support(true_color_support);
    let theme: Theme = match config.theme.as_deref() {
        Some(theme_name) => check_config_load(Theme::new(theme_name), None, "theme"),
        None => Theme::default(),
    };

    // TODO: use the thread local executor to spawn the application task separately from the work pool
    let mut app = Application::new(args, config, theme, language_configurations)
        .context("unable to create new application")?;
    app.run(&mut EventStream::new()).await
}

fn setup_logging(_logpath: Option<PathBuf>, verbosity: u64) -> Result<()> {
    let log_level = match verbosity {
        0 => match std::env::var("HELIX_LOG_LEVEL") {
            Ok(str) => str.parse()?,
            Err(_) => log::LevelFilter::Warn,
        },
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _3_or_more => log::LevelFilter::Trace,
    };

    #[cfg(feature = "integration")]
    let logger: fern::Output = std::io::stdout().into();

    #[cfg(not(feature = "integration"))]
    let logger: fern::Output = {
        helix_loader::setup_log_file(_logpath);
        fern::log_file(helix_loader::log_file())?.into()
    };

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {} [{}] {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log_level)
        .chain(logger)
        .apply()
        .map_err(|err| anyhow::anyhow!(err))
}

fn check_config_load<T: Default>(
    user_load_result: Result<T>,
    alt_to_default: Option<T>,
    cfg_type: &str,
) -> T {
    user_load_result.unwrap_or_else(|err| {
        eprintln!("Bad {} config: {}", cfg_type, err);
        eprintln!("Press <ENTER> to continue with default {} config", cfg_type);
        let _wait_for_enter = std::io::Read::read(&mut std::io::stdin(), &mut []);
        match alt_to_default {
            Some(alt) => alt,
            None => T::default(),
        }
    })
}
