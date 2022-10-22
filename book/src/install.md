# Installation

We provide pre-built binaries on the [GitHub Releases page](https://github.com/helix-editor/helix/releases).

[![Packaging status](https://repology.org/badge/vertical-allrepos/helix.svg)](https://repology.org/project/helix/versions)

## OSX

Helix is available in homebrew-core:

```
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

## Windows

Helix can be installed using [Scoop](https://scoop.sh/) or [Chocolatey](https://chocolatey.org/).

**Scoop:**

```
scoop install helix
```

**Chocolatey:**

```
choco install helix
```


## Build from source

```
git clone https://github.com/helix-editor/helix
cd helix
cargo install --path helix-term
```

This will install the `hx` binary to `$HOME/.cargo/bin` and build tree-sitter grammars in `./runtime/grammars`.

Helix also needs its runtime files so make sure to copy/symlink the `runtime/` directory into the
config directory (for example `~/.config/helix/runtime` on Linux/macOS). This location can be overridden
via the `HELIX_RUNTIME` environment variable.

| OS                   | Command                                          |
| -------------------- | ------------------------------------------------ |
| Windows (Cmd)        | `xcopy /e /i runtime %AppData%\helix\runtime`    |
| Windows (PowerShell) | `xcopy /e /i runtime $Env:AppData\helix\runtime` |
| Linux / MacOS        | `ln -s $PWD/runtime ~/.config/helix/runtime`     |

Starting with Windows Vista you can also create symbolic links on Windows. Note that this requires
elevated priviliges - i.e. PowerShell or Cmd must be run as administrator.

**PowerShell:**

```powershell
New-Item -ItemType SymbolicLink -Target "runtime" -Path "$Env:AppData\helix\runtime"
```

**Cmd:**

```cmd
cd %appdata%\helix
mklink /D runtime "<helix-repo>\runtime"
```

The runtime location can be overridden via the `HELIX_RUNTIME` environment variable.

> NOTE: if `HELIX_RUNTIME` is set prior to calling `cargo install --path helix-term`,
> tree-sitter grammars will be built in `$HELIX_RUNTIME/grammars`.

If you plan on keeping the repo locally, an alternative to copying/symlinking
runtime files is to set `HELIX_RUNTIME=/path/to/helix/runtime`
(`HELIX_RUNTIME=$PWD/runtime` if you're in the helix repo directory).

To use Helix in desktop environments that supports [XDG desktop menu](https://specifications.freedesktop.org/menu-spec/menu-spec-latest.html), including Gnome and KDE, copy the provided `.desktop` file to the correct folder:

```bash
cp contrib/Helix.desktop ~/.local/share/applications
```

To use another terminal than the default, you will need to modify the `.desktop` file. For example, to use `kitty`:

```bash
sed -i "s|Exec=hx %F|Exec=kitty hx %F|g" ~/.local/share/applications/Helix.desktop
sed -i "s|Terminal=true|Terminal=false|g" ~/.local/share/applications/Helix.desktop
```

Please note: there is no icon for Helix yet, so the system default will be used.

## Finishing up the installation

To make sure everything is set up as expected you should finally run the helix healthcheck via

```
hx --health
```

For more information on the information displayed in the health check results refer to [Healthcheck](https://github.com/helix-editor/helix/wiki/Healthcheck).

### Building tree-sitter grammars

Tree-sitter grammars must be fetched and compiled if not pre-packaged.
Fetch grammars with `hx --grammar fetch` (requires `git`) and compile them
with `hx --grammar build` (requires a C++ compiler).

### Installing language servers

Language servers can optionally be installed if you want their features (auto-complete, diagnostics etc.).
Follow the [instructions on the wiki page](https://github.com/helix-editor/helix/wiki/How-to-install-the-default-language-servers) to add your language servers of choice.
