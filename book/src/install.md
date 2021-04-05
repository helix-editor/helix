# Installation

TODO: Prebuilt binaries on GitHub Releases page

## OSX

TODO: brew tap

```
$ brew tap helix-editor/helix
$ brew install helix
```

## Linux

### NixOS

A [flake](https://nixos.wiki/wiki/Flakes) containing the package is available in
the project root. The flake can also be used to spin up a reproducible development
shell for working on Helix.

### Arch Linux

TODO: AUR

## Build from source

```
$ git clone --depth 1 --recurse-submodules -j8 https://github.com/helix-editor/helix
$ cd helix
$ cargo install --path helix-term
```

This will install the `hx` binary to `$HOME/.cargo/bin`.
