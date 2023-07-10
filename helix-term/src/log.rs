use anyhow::anyhow;
use log::LevelFilter;

pub fn setup_logging<T: Into<fern::Output>>(
    output: T,
    verbosity: Option<u64>,
) -> anyhow::Result<()> {
    fern::Dispatch::new()
        .level(get_log_level(verbosity)?)
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {} [{}] {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(output)
        .apply()
        .map_err(|error| anyhow!(error))
}

fn get_log_level(verbosity: Option<u64>) -> anyhow::Result<LevelFilter> {
    if let Ok(env_log_level) = std::env::var("HELIX_LOG_LEVEL") {
        return env_log_level
            .parse::<LevelFilter>()
            .map_err(|error| anyhow!(error));
    }

    if let Some(verbosity) = verbosity {
        let log_level = match verbosity {
            0 => LevelFilter::Warn,
            1 => LevelFilter::Info,
            2 => LevelFilter::Debug,
            _3_or_more => LevelFilter::Trace,
        };

        Ok(log_level)
    } else {
        Ok(LevelFilter::Info)
    }
}
