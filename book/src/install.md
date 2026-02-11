# Installing Silicon

The typical way to install Silicon is via [your operating system's package manager](./package-managers.md).

Note that:

- To get the latest nightly version of Silicon, you need to
  [build from source](./building-from-source.md).

- To take full advantage of Silicon, install the language servers for your
  preferred programming languages. See the
  [wiki](https://github.com/Rani367/Silicon/wiki/Language-Server-Configurations)
  for instructions.

## Pre-built binaries

Download pre-built binaries from the [GitHub Releases page](https://github.com/Rani367/Silicon/releases).
The tarball contents include an `si` binary and a `runtime` directory.
To set up Silicon:

1. Add the `si` binary to your system's `$PATH` to allow it to be used from the command line.
2. Copy the `runtime` directory to a location that `si` searches for runtime files. A typical location on Linux/macOS is `~/.config/silicon/runtime`.

To see the runtime directories that `si` searches, run `si --health`. If necessary, you can override the default runtime location by setting the `SILICON_RUNTIME` environment variable.
