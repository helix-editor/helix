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

## Desktop integration

The source tree includes optional desktop menu and shell completion files under
`contrib/`. To make Helix available from an XDG desktop menu, install
`contrib/Helix.desktop` and the icon as described in
[Configure the desktop shortcut](./building-from-source.md#configure-the-desktop-shortcut).

Shell completion scripts are available for Bash, Elvish, Fish, Nushell, and Zsh
in `contrib/completion/`. For common shell layouts:

```sh
mkdir -p ~/.local/share/bash-completion/completions
cp contrib/completion/hx.bash ~/.local/share/bash-completion/completions/hx

mkdir -p ~/.config/fish/completions
cp contrib/completion/hx.fish ~/.config/fish/completions/hx.fish

mkdir -p ~/.zfunc
cp contrib/completion/hx.zsh ~/.zfunc/_hx
```

For Zsh, make sure `~/.zfunc` is included in `fpath` before `compinit` runs.
