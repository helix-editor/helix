use anyhow::{Context, Error, Result};
use helix_loader::VERSION_AND_GIT_HASH;
use helix_term::application::Application;
use helix_term::args::Args;
use helix_term::config::{Config, ConfigLoadError};
use helix_term::help::HELP_MESSAGE;
use helix_term::log::setup_logging;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse_args().context("could not parse arguments")?;

    helix_loader::initialize_config_file(args.config_file.clone());
    helix_loader::initialize_log_file(args.log_file.clone());

    // Help has a higher priority and should be handled separately.
    if args.display_help {
        print!("{}", *HELP_MESSAGE);
        std::process::exit(0);
    }

    if args.display_version {
        println!("helix {}", VERSION_AND_GIT_HASH);
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
        std::process::exit(0);
    }

    if args.build_grammars {
        helix_loader::grammar::build_grammars(None)?;
        std::process::exit(0);
    }

    setup_logging(
        fern::log_file(helix_loader::log_file())?,
        Some(args.verbosity),
    )
    .context("failed to initialize logging")?;

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

    let syn_loader_conf = helix_core::config::user_syntax_loader().unwrap_or_else(|err| {
        eprintln!("Bad language config: {}", err);
        eprintln!("Press <ENTER> to continue with default language config");
        use std::io::Read;
        // This waits for an enter press.
        let _ = std::io::stdin().read(&mut []);
        helix_core::config::default_syntax_loader()
    });

    // TODO: use the thread local executor to spawn the application task separately from the work pool
    let mut app = Application::new(
        tui::backend::CrosstermBackend::new(std::io::stdout(), &config.editor),
        args,
        config,
        syn_loader_conf,
    )
    .context("unable to create new application")?;

    std::process::exit(app.run().await?)
}
