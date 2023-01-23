pub fn help() -> String {
    format!(
        "\
{pkg_name} {version}
{authors}
{description}

USAGE:
    hx [FLAGS] [files]...

ARGS:
    <files>...    Sets the input file to use, position can also be specified via file[:row[:col]]

FLAGS:
    -h, --help                     Prints help information
    --tutor                        Loads the tutorial
    --health [SECTION]             Displays potential errors in editor setup.
                                   Optional SECTION can 'paths', 'clipboard', 'languages' or a
                                   singular language name.
    -g, --grammar {{fetch|build}}    Fetches or builds tree-sitter grammars listed in languages.toml
    -c, --config <file>            Specifies a file to use for configuration
    -v                             Increases logging verbosity each use for up to 3 times
    --log                          Specifies a file to use for logging
                                   (default file: {log_file_path})
    -V, --version                  Prints version information
    --vsplit                       Splits all given files vertically into different windows
    --hsplit                       Splits all given files horizontally into different windows
",
        pkg_name = env!("CARGO_PKG_NAME"),
        version = helix_loader::VERSION_AND_GIT_HASH,
        authors = env!("CARGO_PKG_AUTHORS"),
        description = env!("CARGO_PKG_DESCRIPTION"),
        log_file_path = helix_loader::log_file().display(),
    )
}
