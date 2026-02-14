## Package managers

- [Linux](#linux)
  - [Debian](#debian)
  - [Ubuntu/Mint](#ubuntumint)
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

<!-- TODO: Add Silicon repology badge once available -->
<!-- [![Packaging status](https://repology.org/badge/vertical-allrepos/silicon.svg)](https://repology.org/project/silicon/versions) -->

## Linux

The following third party repositories are available:

### Debian

```sh
sudo apt install si
```

If you are running a system older than Debian 13, follow the steps for
[Ubuntu/Mint](#ubuntumint).

### Ubuntu/Mint

Install the Debian package [from the release page](https://github.com/silicon-editor/Silicon/releases/latest).

If you are running a system older than Ubuntu 22.04, Mint 21, or Debian 12, you can build the `.deb` file locally
[from source](./building-from-source.md#building-the-debian-package).

### Fedora/RHEL

```sh
sudo dnf install silicon
```

### Arch Linux extra

Releases are available in the `extra` repository:

```sh
sudo pacman -S silicon
```

> 💡 When installed from the `extra` repository, run Silicon with `silicon` instead of `si`.
>
> For example:
> ```sh
> silicon --health
> ```
> to check health

Additionally, a [silicon-git](https://aur.archlinux.org/packages/silicon-git/) package is available
in the AUR, which builds the master branch.

### NixOS

Silicon is available in [nixpkgs](https://github.com/nixos/nixpkgs) through the `silicon` attribute,
the unstable channel usually carries the latest release.

Silicon is also available as a [flake](https://wiki.nixos.org/wiki/Flakes) in the project
root. Use `nix develop` to spin up a reproducible development shell. Outputs are
cached for each push to master using [Cachix](https://www.cachix.org/). The
flake is configured to automatically make use of this cache assuming the user
accepts the new settings on first use.

If you are using a version of Nix without flakes enabled,
[install Cachix CLI](https://docs.cachix.org/installation) and use
`cachix use silicon` to configure Nix to use cached outputs when possible.

### Flatpak

Silicon is available on [Flathub](https://flathub.org/en-GB/apps/com.silicon_editor.Silicon):

```sh
flatpak install flathub com.silicon_editor.Silicon
flatpak run com.silicon_editor.Silicon
```

### Snap

Silicon is available on [Snapcraft](https://snapcraft.io/silicon) and can be installed with:

```sh
snap install --classic silicon
```

This will install Silicon as both `/snap/bin/silicon` and `/snap/bin/si`, so make sure `/snap/bin` is in your `PATH`.

### AppImage

Install Silicon using the Linux [AppImage](https://appimage.org/) format.
Download the official Silicon AppImage from the [latest releases](https://github.com/silicon-editor/Silicon/releases/latest) page.

```sh
chmod +x silicon-*.AppImage # change permission for executable mode
./silicon-*.AppImage # run silicon
```

You can optionally [add the `.desktop` file](./building-from-source.md#configure-the-desktop-shortcut). Silicon must be installed in `PATH` with the name `si`. For example:
```sh
mkdir -p "$HOME/.local/bin"
mv silicon-*.AppImage "$HOME/.local/bin/si"
```

and make sure `~/.local/bin` is in your `PATH`.

## macOS

### Homebrew Core

```sh
brew install silicon
```

### MacPorts

```sh
sudo port install silicon
```

## Windows

Install on Windows using [Winget](https://learn.microsoft.com/en-us/windows/package-manager/winget/), [Scoop](https://scoop.sh/), [Chocolatey](https://chocolatey.org/)
or [MSYS2](https://msys2.org/).

### Winget
Windows Package Manager winget command-line tool is by default available on Windows 11 and modern versions of Windows 10 as a part of the App Installer.
You can get [App Installer from the Microsoft Store](https://www.microsoft.com/p/app-installer/9nblggh4nns1#activetab=pivot:overviewtab). If it's already installed, make sure it is updated with the latest version.

```sh
winget install Silicon.Silicon
```

### Scoop

```sh
scoop install silicon
```

### Chocolatey

```sh
choco install silicon
```

### MSYS2

For 64-bit Windows 8.1 or above:

```sh
pacman -S mingw-w64-ucrt-x86_64-silicon
```
