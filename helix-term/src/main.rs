use anyhow::{Context, Error, Result};
use cobs::{DecodeResult, DecoderState};
use crossterm::event::EventStream;
use helix_loader::VERSION_AND_GIT_HASH;
use helix_term::application::Application;
use helix_term::args::Args;
use helix_term::config::{Config, ConfigLoadError};
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Write};
use std::sync::Mutex;

fn setup_logging(verbosity: u64) -> Result<()> {
    let mut base_config = fern::Dispatch::new();

    base_config = match verbosity {
        0 => base_config.level(log::LevelFilter::Warn),
        1 => base_config.level(log::LevelFilter::Info),
        2 => base_config.level(log::LevelFilter::Debug),
        _3_or_more => base_config.level(log::LevelFilter::Trace),
    };

    let log_compressor = Mutex::new((
        fern::log_file(helix_loader::log_file())?,
        zstd::bulk::Compressor::with_dictionary(3, include_bytes!("zstd_dict"))?,
    ));

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
        .chain(fern::Output::call(move |record| {
            let mut compressor = log_compressor.lock().unwrap();

            let record = compressor
                .1
                .compress(format!("{}", record.args()).as_bytes())
                .unwrap();

            let mut record = cobs::encode_vec(&record);
            record.push(0);

            compressor.0.write_all(&record).unwrap();
        }));

    base_config.chain(file_config).apply()?;

    Ok(())
}

fn main() -> Result<()> {
    let exit_code = main_impl()?;
    std::process::exit(exit_code);
}

#[tokio::main]
async fn main_impl() -> Result<i32> {
    let help = format!(
        "\
{} {}
{}
{}

USAGE:
    hx [FLAGS] [files]...

ARGS:
    <files>...    Sets the input file to use, position can also be specified via file[:row[:col]]

FLAGS:
    -h, --help                     Prints help information
    --tutor                        Loads the tutorial
    --health [CATEGORY]            Checks for potential errors in editor setup
                                   CATEGORY can be a language or one of 'clipboard', 'languages'
                                   or 'all'. 'all' is the default if not specified.
    -g, --grammar {{fetch|build}}    Fetches or builds tree-sitter grammars listed in languages.toml
    -c, --config <file>            Specifies a file to use for configuration
    -v                             Increases logging verbosity each use for up to 3 times
    --decode-log                   Decodes the compressed log file from stdin and writes it to stdout
    --log <file>                   Specifies a file to use for logging
                                   (default file: {})
    -V, --version                  Prints version information
    --vsplit                       Splits all given files vertically into different windows
    --hsplit                       Splits all given files horizontally into different windows
    -w, --working-dir <path>       Specify an initial working directory
    +N                             Open the first given file at line number N
",
        env!("CARGO_PKG_NAME"),
        VERSION_AND_GIT_HASH,
        env!("CARGO_PKG_AUTHORS"),
        env!("CARGO_PKG_DESCRIPTION"),
        helix_loader::default_log_file().display(),
    );

    let mut args = Args::parse_args().context("could not parse arguments")?;

    helix_loader::initialize_config_file(args.config_file.clone());
    helix_loader::initialize_log_file(args.log_file.clone());

    // Help has a higher priority and should be handled separately.
    if args.display_help {
        print!("{}", help);
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
        return Ok(0);
    }

    if args.build_grammars {
        helix_loader::grammar::build_grammars(None)?;
        return Ok(0);
    }

    if args.decode_log {
        let mut decompressor =
            zstd::bulk::Decompressor::with_dictionary(include_bytes!("zstd_dict"))?;

        let mut cobs_decoder = DecoderState::Idle;

        let mut reader = BufReader::new(stdin().lock());
        let mut writer = BufWriter::new(stdout().lock());

        let mut out_buf = Vec::new();
        let mut log_buf = [0u8; 10_000];
        loop {
            let mut consumed = 0;
            let buf = reader.fill_buf()?;

            if buf.is_empty() {
                break;
            }

            for &byte in buf {
                consumed += 1;
                match cobs_decoder.feed(byte) {
                    Ok(DecodeResult::DataComplete) => {
                        let decompressed =
                            decompressor.decompress_to_buffer(&out_buf, &mut log_buf)?;

                        writer.write_all(&log_buf[..decompressed])?;
                        writer.write_all(b"\n")?;
                        out_buf.clear();
                    }
                    Ok(DecodeResult::DataContinue(b)) => out_buf.push(b),
                    Ok(DecodeResult::NoData) => {}
                    Err(()) => out_buf.clear(),
                }
            }
            writer.flush()?;
            reader.consume(consumed);
        }
        return Ok(0);
    }

    setup_logging(args.verbosity).context("failed to initialize logging")?;

    // Before setting the working directory, resolve all the paths in args.files
    for (path, _) in args.files.iter_mut() {
        *path = helix_stdx::path::canonicalize(&path);
    }

    // NOTE: Set the working directory early so the correct configuration is loaded. Be aware that
    // Application::new() depends on this logic so it must be updated if this changes.
    if let Some(path) = &args.working_directory {
        helix_stdx::env::set_current_working_dir(path)?;
    } else if let Some((path, _)) = args.files.first().filter(|p| p.0.is_dir()) {
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

    let syn_loader_conf = helix_core::config::user_syntax_loader().unwrap_or_else(|err| {
        eprintln!("Bad language config: {}", err);
        eprintln!("Press <ENTER> to continue with default language config");
        use std::io::Read;
        // This waits for an enter press.
        let _ = std::io::stdin().read(&mut []);
        helix_core::config::default_syntax_loader()
    });

    // TODO: use the thread local executor to spawn the application task separately from the work pool
    let mut app = Application::new(args, config, syn_loader_conf)
        .context("unable to create new application")?;

    let exit_code = app.run(&mut EventStream::new()).await?;

    Ok(exit_code)
}
