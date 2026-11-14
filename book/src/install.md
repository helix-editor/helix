# Installing Helix

The typical way to install Helix is via [your operating system's package manager](./package-managers.md).

Note that:

- To get the latest nightly version of Helix, you need to
  [build from source](./building-from-source.md).

- To take full advantage of Helix, install the language servers for your
  preferred programming languages. See the
  [wiki](https://github.com/helix-editor/helix/wiki/Language-Server-Configurations)
  for instructions.

## Pre-built binaries

Download pre-built binaries from the [GitHub Releases page](https://github.com/helix-editor/helix/releases).
The tarball contents include an `hx` binary and a `runtime` directory.
To set up Helix:

1. Add the `hx` binary to your system's `$PATH` to allow it to be used from the command line.
2. Copy the `runtime` directory to a location that `hx` searches for runtime files. A typical location on Linux/macOS is `~/.config/helix/runtime`.

To see the runtime directories that `hx` searches, run `hx --health`. If necessary, you can override the default runtime location by setting the `HELIX_RUNTIME` environment variable.

## Desktop entry and shell completions

The source tree includes an XDG desktop entry and completion scripts for Bash, Fish, Zsh, Elvish, and Nushell under [`contrib/`](https://github.com/helix-editor/helix/tree/master/contrib). See [Configure the desktop shortcut](./building-from-source.md#configure-the-desktop-shortcut) and [Shell completions](./building-from-source.md#shell-completions) for installation instructions. Packaged installs usually ship these files already.
