# Completions for Helix: <https://github.com/helix-editor/helix>
#
# NOTE: the `+N` syntax is not supported in Nushell (https://github.com/nushell/nushell/issues/13418)
#       so it has not been specified here and will not be proposed in the autocompletion of Nushell.
#       The help message won't be overriden though, so it will still be present here

def health_categories [] {
    let languages = ^hx --health languages | detect columns | get Language | filter { $in != null }
    let completions = [ "all", "clipboard", "languages" ] | append $languages
    return $completions
}

def grammar_categories [] { ["fetch", "build"] }

# A post-modern text editor.
export extern hx [
    --help(-h),                                 # Prints help information
    --tutor,                                    # Loads the tutorial
    --health: string@health_categories,         # Checks for potential errors in editor setup
    --grammar(-g): string@grammar_categories,   # Fetches or builds tree-sitter grammars listed in `languages.toml`
    --config(-c): glob,                         # Specifies a file to use for configuration
    -v,                                         # Increases logging verbosity each use for up to 3 times
    --log: glob,                                # Specifies a file to use for logging
    --version(-V),                              # Prints version information
    --vsplit,                                   # Splits all given files vertically into different windows
    --hsplit,                                   # Splits all given files horizontally into different windows
    --working-dir(-w): glob,                    # Specify an initial working directory
    ...files: glob,                             # Sets the input file to use, position can also be specified via file[:row[:col]]
]
