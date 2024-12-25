## Package managers

- [Linux](#linux)
  - [Ubuntu](#ubuntu)
  - [Fedora/RHEL](#fedorarhel)
  - [Arch Linux extra](#arch-linux-extra)
  - [NixOS](#nixos)
  - [Flatpak](#flatpak)
  - [Snap](#snap)
  - [AppImage](#appimage)
- [macOS](#macos)
  - [Homebrew Core](#homebrew-core)
  - [MacPorts](#macports)
- [Windows](#windows)
  - [Winget](#winget)
  - [Scoop](#scoop)
  - [Chocolatey](#chocolatey)
  - [MSYS2](#msys2)

[![Packaging status](https://repology.org/badge/vertical-allrepos/helix-editor.svg)](https://repology.org/project/helix-editor/versions)

## Linux

The following third party repositories are available:

### Ubuntu

Add the `PPA` for Helix:

```sh
sudo add-apt-repository ppa:maveonair/helix-editor
sudo apt update
sudo apt install helix
```

### Fedora/RHEL

```sh
sudo dnf install helix
```

### Arch Linux extra

Releases are available in the `extra` repository:

```sh
sudo pacman -S helix
```

> 💡 When installed from the `extra` repository, run Helix with `helix` instead of `hx`.
>
> For example:
> ```sh
> helix --health
> ```
> to check health

Additionally, a [helix-git](https://aur.archlinux.org/packages/helix-git/) package is available
in the AUR, which builds the master branch.

### NixOS

Helix is available in [nixpkgs](https://github.com/nixos/nixpkgs) through the `helix` attribute,
the unstable channel usually carries the latest release.

Helix is also available as a [flake](https://wiki.nixos.org/wiki/Flakes) in the project
root. Use `nix develop` to spin up a reproducible development shell. Outputs are
cached for each push to master using [Cachix](https://www.cachix.org/). The
flake is configured to automatically make use of this cache assuming the user
accepts the new settings on first use.

If you are using a version of Nix without flakes enabled,
[install Cachix CLI](https://docs.cachix.org/installation) and use
`cachix use helix` to configure Nix to use cached outputs when possible.

### Flatpak

Helix is available on [Flathub](https://flathub.org/en-GB/apps/com.helix_editor.Helix):

```sh
flatpak install flathub com.helix_editor.Helix
flatpak run com.helix_editor.Helix
```

### Snap

Helix is available on [Snapcraft](https://snapcraft.io/helix) and can be installed with:

```sh
snap install --classic helix
```

This will install Helix as both `/snap/bin/helix` and `/snap/bin/hx`, so make sure `/snap/bin` is in your `PATH`.

### AppImage

Install Helix using the Linux [AppImage](https://appimage.org/) format.
Download the official Helix AppImage from the [latest releases](https://github.com/helix-editor/helix/releases/latest) page.

```sh
chmod +x helix-*.AppImage # change permission for executable mode
./helix-*.AppImage # run helix
```

You can optionally [add the `.desktop` file](./building-from-source.md#configure-the-desktop-shortcut). Helix must be installed in `PATH` with the name `hx`. For example:
```sh
mkdir -p "$HOME/.local/bin"
mv helix-*.AppImage "$HOME/.local/bin/hx"
```

and make sure `~/.local/bin` is in your `PATH`.

## macOS

### Homebrew Core

```sh
brew install helix
```

### MacPorts

```sh
sudo port install helix
```

## Windows

Install on Windows using [Winget](https://learn.microsoft.com/en-us/windows/package-manager/winget/), [Scoop](https://scoop.sh/), [Chocolatey](https://chocolatey.org/)
or [MSYS2](https://msys2.org/).

### Winget
Windows Package Manager winget command-line tool is by default available on Windows 11 and modern versions of Windows 10 as a part of the App Installer.
You can get [App Installer from the Microsoft Store](https://www.microsoft.com/p/app-installer/9nblggh4nns1#activetab=pivot:overviewtab). If it's already installed, make sure it is updated with the latest version.

```sh
winget install Helix.Helix
```

### Scoop

```sh
scoop install helix
```

### Chocolatey

```sh
choco install helix
```

### MSYS2

For 64-bit Windows 8.1 or above:

```sh
pacman -S mingw-w64-ucrt-x86_64-helix
```
