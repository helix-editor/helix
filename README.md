# Helix


[![Build status](https://github.com/helix-editor/helix/actions/workflows/build.yml/badge.svg)](https://github.com/helix-editor/helix/actions)

![Screenshot](./screenshot.png)

A kakoune / neovim inspired editor, written in Rust.

The editing model is very heavily based on kakoune; during development I found
myself agreeing with most of kakoune's design decisions.

For more information, see the [website](https://helix-editor.com) or
[documentation](https://docs.helix-editor.com/).

All shortcuts/keymaps can be found [in the documentation on the website](https://docs.helix-editor.com/keymap.html)

# Features

- Vim-like modal editing
- Multiple selections
- Built-in language server support
- Smart, incremental syntax highlighting and code editing via tree-sitter

It's a terminal-based editor first, but I'd like to explore a custom renderer
(similar to emacs) in wgpu or skulpin.

# Installation

Note: Only certain languages have indentation definitions at the moment. Check
`runtime/<lang>/` for `indents.toml`.

We provide packaging for various distributions, but here's a quick method to
build from source.

```
git clone --recurse-submodules --shallow-submodules -j8 https://github.com/helix-editor/helix
cd helix
cargo install --path helix-term
```

This will install the `hx` binary to `$HOME/.cargo/bin`.

Now copy the `runtime/` directory somewhere. Helix will by default look for the runtime
inside the config directory or the same directory as executable, but that can be overriden
via the `HELIX_RUNTIME` environment variable.

> NOTE: running via cargo doesn't require setting explicit `HELIX_RUNTIME` path, it will automatically
> detect the `runtime` directory in the project root.

If you want to embed the `runtime/` directory into the Helix binary you can build
it with:

```
cargo install --path helix-term --features "embed_runtime"
```

[![Packaging status](https://repology.org/badge/vertical-allrepos/helix.svg)](https://repology.org/project/helix/versions)

## MacOS
Helix can be installed on MacOS through homebrew via:

```
brew tap helix-editor/helix
brew install helix
```
 
# Contributing

Contributors are very welcome! **No contribution is too small and all contributions are valued.**

Some suggestions to get started:

- You can look at the [good first issue](https://github.com/helix-editor/helix/labels/E-easy) label on the issue tracker.
- Help with packaging on various distributions needed!
- To use print debugging to the `~/.cache/helix/helix.log` file, you must:
  * Print using `log::info!`, `warn!`, or `error!`. (`log::info!("helix!")`)
  * Pass the appropriate verbosity level option for the desired log level. (`hx -v <file>` for info, more `v`s for higher severity inclusive)
- If your preferred language is missing, integrating a tree-sitter grammar for
    it and defining syntax highlight queries for it is straight forward and
    doesn't require much knowledge of the internals.

We provide an [architecture.md](./docs/architecture.md) that should give you
a good overview of the internals.

# Getting help

Discuss the project on the community [Matrix Space](https://matrix.to/#/#helix-community:matrix.org) (make sure to join `#helix-editor:matrix.org` if you're on a client that doesn't support Matrix Spaces yet).
