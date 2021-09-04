use anyhow::{Context, Error, Result};
use helix_term::application::Application;
use helix_term::args::Args;
use helix_term::config::Config;
use helix_term::keymap::merge_keys;
use tracing_subscriber::{filter::LevelFilter, EnvFilter};

fn setup_logging(verbosity: u64) -> Result<impl Drop> {
    use std::fs::OpenOptions;
    use tracing::subscriber;
    use tracing_appender::non_blocking;
    use tracing_subscriber::{layer::SubscriberExt, registry::Registry};

    let cache_dir = helix_core::cache_dir();

    let open_options = {
        let mut o = OpenOptions::new();
        o.append(true).create(true).write(true);
        o
    };

    let mut env_filter = EnvFilter::try_from_env("HELIX_LOG").unwrap_or_default();

    env_filter = match verbosity {
        0 => env_filter.add_directive(LevelFilter::WARN.into()),
        1 => env_filter.add_directive(LevelFilter::INFO.into()),
        2 => env_filter.add_directive(LevelFilter::DEBUG.into()),
        _ => env_filter.add_directive(LevelFilter::TRACE.into()),
    };

    let registry = Registry::default().with(env_filter);

    let (registry, guard) = {
        let log_file = open_options.open(cache_dir.join("helix.log"))?;
        let (non_blocking, guard) = non_blocking(log_file);
        let layer = tracing_subscriber::fmt::layer().with_writer(non_blocking);
        (registry.with(layer), guard)
    };

    #[cfg(tracing_flame)]
    let (registry, guard) = {
        let flame_file = open_options.open(cache_dir.join("tracing.folded"));
        let (non_blocking, guard_) = non_blocking(flame_file);
        let layer = tracing_flame::FlameLayer::with_writer(non_blocking);
        (registry.with(layer), (guard, guard_))
    };

    #[cfg(tracing_tracy)]
    let (registry, guard) = (registry.with(tracing_tracy::TracyLayer::new()), guard);

    subscriber::set_global_default(registry).unwrap();

    tracing_log::LogTracer::builder().init();

    Ok(guard)
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

    let _guard = setup_logging(args.verbosity).context("failed to initialize logging")?;
    tracing::info!("Just a test");

    // TODO: use the thread local executor to spawn the application task separately from the work pool
    let mut app = Application::new(args, config).context("unable to create new application")?;
    app.run().await.unwrap();

    Ok(())
}
