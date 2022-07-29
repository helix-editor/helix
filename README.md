# Helix

[![Build status](https://github.com/helix-editor/helix/actions/workflows/build.yml/badge.svg)](https://github.com/helix-editor/helix/actions)

![Screenshot](./screenshot.png)

A kakoune / neovim inspired editor, written in Rust.

The editing model is very heavily based on kakoune; during development I found
myself agreeing with most of kakoune's design decisions.

For more information, see the [website](https://helix-editor.com) or
[documentation](https://docs.helix-editor.com/).

All shortcuts/keymaps can be found [in the documentation on the website](https://docs.helix-editor.com/keymap.html).

[Troubleshooting](https://github.com/helix-editor/helix/wiki/Troubleshooting)

# Features

- Vim-like modal editing
- Multiple selections
- Built-in language server support
- Smart, incremental syntax highlighting and code editing via tree-sitter

It's a terminal-based editor first, but I'd like to explore a custom renderer
(similar to emacs) in wgpu or skulpin.

Note: Only certain languages have indentation definitions at the moment. Check
`runtime/queries/<lang>/` for `indents.scm`.

# Installation

Packages are available for various distributions (see [Installation docs](https://docs.helix-editor.com/install.html)).

If you would like to build from source:

```shell
git clone https://github.com/helix-editor/helix
cd helix
cargo install --path helix-term
```

This will install the `hx` binary to `$HOME/.cargo/bin` and build tree-sitter grammars.
If you want to customize your `languages.toml` config,
tree-sitter grammars may be manually fetched and built with `hx --grammar fetch` and `hx --grammar build`.

Helix also needs its runtime files so make sure to copy/symlink the `runtime/` directory into the
config directory (for example `~/.config/helix/runtime` on Linux/macOS, or `%AppData%/helix/runtime` on Windows).

| OS                   | Command                                      |
| -------------------- | -------------------------------------------- |
| Windows (cmd.exe)    | `xcopy runtime %AppData%\helix\runtime`      |
| Windows (PowerShell) | `xcopy runtime $Env:AppData\helix\runtime`   |
| Linux/macOS          | `ln -s $PWD/runtime ~/.config/helix/runtime` |

This location can be overridden via the `HELIX_RUNTIME` environment variable.

Packages already solve this for you by wrapping the `hx` binary with a wrapper
that sets the variable to the install dir.

> NOTE: running via cargo also doesn't require setting explicit `HELIX_RUNTIME` path, it will automatically
> detect the `runtime` directory in the project root.

In order to use LSP features like auto-complete, you will need to
[install the appropriate Language Server](https://github.com/helix-editor/helix/wiki/How-to-install-the-default-language-servers)
for a language.

[![Packaging status](https://repology.org/badge/vertical-allrepos/helix.svg)](https://repology.org/project/helix/versions)

## MacOS

Helix can be installed on MacOS through homebrew via:

```
brew tap helix-editor/helix
brew install helix
```

# Contributing

Contributing guidelines can be found [here](./docs/CONTRIBUTING.md).

# Getting help

Your question might already be answered on the [FAQ](https://github.com/helix-editor/helix/wiki/FAQ).

Discuss the project on the community [Matrix Space](https://matrix.to/#/#helix-community:matrix.org) (make sure to join `#helix-editor:matrix.org` if you're on a client that doesn't support Matrix Spaces yet).
