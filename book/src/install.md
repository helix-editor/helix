# Installation

We provide pre-built binaries on the [GitHub Releases page](https://github.com/helix-editor/helix/releases).

[![Packaging status](https://repology.org/badge/vertical-allrepos/helix.svg)](https://repology.org/project/helix/versions)

## OSX

A Homebrew tap is available:

```
brew tap helix-editor/helix
brew install helix
```

## Linux

### NixOS

A [flake](https://nixos.wiki/wiki/Flakes) containing the package is available in
the project root. The flake can also be used to spin up a reproducible development
shell for working on Helix.

### Arch Linux

Binary packages are available on AUR:
- [helix-bin](https://aur.archlinux.org/packages/helix-bin/) contains the pre-built release
- [helix-git](https://aur.archlinux.org/packages/helix-git/) builds the master branch

## Build from source

```
git clone --recurse-submodules --shallow-submodules -j8 https://github.com/helix-editor/helix
cd helix
cargo install --path helix-term
```

This will install the `hx` binary to `$HOME/.cargo/bin`.

Helix also needs it's runtime files so make sure to copy/symlink the `runtime/` directory into the
config directory (for example `~/.config/helix/runtime` on Linux/macOS). This location can be overriden
via the `HELIX_RUNTIME` environment variable.
