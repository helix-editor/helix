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
shell for working on Helix with `nix develop`.

Flake outputs are cached for each push to master using
[Cachix](https://www.cachix.org/). The flake is configured to
automatically make use of this cache assuming the user accepts
the new settings on first use.

If you are using a version of Nix without flakes enabled you can
[install Cachix cli](https://docs.cachix.org/installation); `cachix use helix` will
configure Nix to use cached outputs when possible.

### Arch Linux

Releases are available in the `community` repository.

A [helix-git](https://aur.archlinux.org/packages/helix-git/) package is also available on the AUR, which builds the master branch.

### Fedora Linux

You can install the COPR package for Helix via

```
sudo dnf copr enable varlad/helix
sudo dnf install helix
```

### Void Linux

```
sudo xbps-install helix
```

## Build from source

```
git clone https://github.com/helix-editor/helix
cd helix
cargo install --path helix-term
```

This will install the `hx` binary to `$HOME/.cargo/bin`.

Helix also needs it's runtime files so make sure to copy/symlink the `runtime/` directory into the
config directory (for example `~/.config/helix/runtime` on Linux/macOS). This location can be overridden
via the `HELIX_RUNTIME` environment variable.

| OS                | command   |
|-------------------|-----------|
|windows(cmd.exe)   |`xcopy runtime %AppData%/helix/runtime`     |
|windows(powershell)|`xcopy runtime $Env:AppData\helix\runtime`  |
|linux/macos        |`ln -s $PWD/runtime ~/.config/helix/runtime`|

## Finishing up the installation 

To make sure everything is set up as expected you should finally run the helix healthcheck via 
```
hx --health
```
For more information on the information displayed in the healthcheck results refer to [Healthcheck](https://github.com/helix-editor/helix/wiki/Healthcheck).


### Building tree-sitter grammars

Tree-sitter grammars must be fetched and compiled if not pre-packaged.
Fetch grammars with `hx --grammar fetch` (requires `git`) and compile them
with `hx --grammar build` (requires a C++ compiler).

### Installing language servers

Language servers can optionally be installed if you want their features (auto-complete, diagnostics etc.).
Follow the [instructions on the wiki page](https://github.com/helix-editor/helix/wiki/How-to-install-the-default-language-servers) to add your language servers of choice.
