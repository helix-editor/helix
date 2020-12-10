#![allow(unused)]

mod application;
mod commands;
mod compositor;
mod keymap;
mod prompt;

use application::Application;

use clap::{App, Arg};
use std::path::PathBuf;

use anyhow::Error;

static EX: smol::Executor = smol::Executor::new();

fn setup_logging(verbosity: u64) -> Result<(), fern::InitError> {
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
        .chain(fern::log_file("helix.log")?);

    base_config.chain(file_config).apply()?;

    Ok(())
}

fn main() -> Result<(), Error> {
    let args = clap::app_from_crate!()
        .arg(
            Arg::new("files")
                .about("Sets the input file to use")
                .required(true)
                .multiple(true)
                .index(1),
        )
        .arg(
            Arg::new("verbose")
                .about("Increases logging verbosity each use for up to 3 times")
                .short('v')
                .takes_value(false)
                .multiple_occurrences(true),
        )
        .get_matches();

    let verbosity: u64 = args.occurrences_of("verbose");

    setup_logging(verbosity).expect("failed to initialize logging.");

    for _ in 0..num_cpus::get() {
        std::thread::spawn(move || smol::block_on(EX.run(smol::future::pending::<()>())));
    }

    let mut app = Application::new(args, &EX).unwrap();

    // we use the thread local executor to spawn the application task separately from the work pool
    smol::block_on(app.run());

    Ok(())
}
