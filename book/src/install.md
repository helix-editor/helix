# Installation

<!--toc:start-->
- [Installation](#installation)
  - [Pre-built binaries](#pre-built-binaries)
  - [Official repositories for Linux, macOS and Windows](#official-repositories-for-linux-macos-and-windows)
  - [Third party repositories for Linux](#third-party-repositories-for-linux)
    - [Ubuntu](#ubuntu)
    - [Fedora/RHEL](#fedorarhel)
    - [Arch Linux community](#arch-linux-community)
    - [NixOS](#nixos)
  - [Building from source](#building-from-source)
  - [Validating the Installation](#validating-the-installation)
<!--toc:end-->

To install Helix, follow the instructions specific to your operating system.
Additionally:

- To get the latest pre-release version of Helix, you will need to
  [build from source](#building-from-source).

- To take full advantage of Helix, install the language servers for your
  preferred programming languages. Refer to the
  [Helix Wiki](https://github.com/helix-editor/helix/wiki/How-to-install-the-default-language-servers)
  for detailed instructions.

## Pre-built binaries

Download pre-built binaries from the
[GitHub Releases page](https://github.com/helix-editor/helix/releases). You will
need to add the binary to your system's `$PATH` to access it from the command
line.

## Official repositories for Linux, macOS and Windows

Helix is available for Linux, macOS and Windows via the official repositories listed below. For installation instructions consult the documentation.

[![Packaging status](https://repology.org/badge/vertical-allrepos/helix.svg)](https://repology.org/project/helix/versions)

If Helix is not available through the above list, you will need to use a
third-party repository or build from source.

## Third party repositories for Linux

The following third party repositories are available:

### Ubuntu

Helix is available via [Maveonair's PPA](https://launchpad.net/~maveonair/+archive/ubuntu/helix-editor):

```sh
sudo add-apt-repository ppa:maveonair/helix-editor
sudo apt update
sudo apt install helix
```

### Fedora/RHEL

Helix is available via `copr`:

```sh
sudo dnf copr enable varlad/helix
sudo dnf install helix
```

### Arch Linux community

Releases are available in the `community` repository. Additionally, a
[helix-git](https://aur.archlinux.org/packages/helix-git/) package is available
in the AUR, which builds the master branch.

### NixOS

Helix is available as a [flake](https://nixos.wiki/wiki/Flakes) in the project
root. Use `nix develop` to spin up a reproducible development shell. Outputs are
cached for each push to master using [Cachix](https://www.cachix.org/). The
flake is configured to automatically make use of this cache assuming the user
accepts the new settings on first use.

If you are using a version of Nix without flakes enabled,
[install Cachix CLI](https://docs.cachix.org/installation) and use
`cachix use helix` to configure Nix to use cached outputs when possible.

## Building from source

1. Clone the repository:

```sh
git clone https://github.com/helix-editor/helix
cd helix
```

2. Compile Helix:

```sh
cargo install --path helix-term --locked
```

This command will create the `hx` executable and construct the tree-sitter
grammars in the `runtime` folder, or in the folder specified in `HELIX_RUNTIME`
(as described below).

> 💡 If you are using the musl-libc instead of glibc the following environment variable must be set during the build
> to ensure tree sitter grammars can be loaded correctly:
>
> ```sh
> RUSTFLAGS="-C target-feature=-crt-static"
> ```

3. Configure Helix's runtime files

**IMPORTANT**: The runtime files must be accessible to the newly created binary.
They are currently located in the source code `runtime` directory. To make them
accessible, you must follow the instructions for your operating system:

> 💡 Tree-sitter grammars can be fetched and compiled if not pre-packaged. Fetch
> grammars with `hx --grammar fetch` (requires `git`) and compile them with
> `hx --grammar build` (requires a C++ compiler).

- Linux and macOS

Either,

1. Set the `HELIX_RUNTIME` environment variable on your system to tell Helix
   where to find the runtime files.

   Use the `HELIX_RUNTIME=/path/to/helix/runtime` format, for example:
   `HELIX_RUNTIME=/home/user-name/src/helix/runtime`. Add this variable to your
   `~/.bashrc` file or equivalent to persist it.

Or,

2. Create a symlink in `~/.config/helix` that links to the source code
   directory.

   ```sh
   ln -s $PWD/runtime ~/.config/helix/runtime
   ```

And optionally:

3. Configure the Desktop Shortcut

If your desktop environment supports the
[XDG desktop menu](https://specifications.freedesktop.org/menu-spec/menu-spec-latest.html)
you can configure Helix to show up in the application menu by copying the
provided `.desktop` and icon files to their correct folders:

```sh
cp contrib/Helix.desktop ~/.local/share/applications
cp contrib/helix.png ~/.icons # or ~/.local/share/icons
```

To use another terminal than the system default, you can modify the `.desktop`
file. For example, to use `kitty`:

```sh
sed -i "s|Exec=hx %F|Exec=kitty hx %F|g" ~/.local/share/applications/Helix.desktop
sed -i "s|Terminal=true|Terminal=false|g" ~/.local/share/applications/Helix.desktop
```

- Windows

Either,

1. Set the `HELIX_RUNTIME` environment variable on your system to tell Helix
   where to find the runtime files.

   You can either do this using the Windows setting (search for
   `Edit environment variables for your account`) or use the `setx` command in
   Cmd:

   ```sh
   setx HELIX_RUNTIME "%userprofile%\source\repos\helix\runtime"
   ```

   > 💡 `%userprofile%` resolves to your user directory like
   > `C:\Users\Your-Name\` for example.

Or,

2. Create a symlink in `%appdata%\helix\` that links to the source code
   directory.

   | Method     | Command                                                                                |
   | ---------- | -------------------------------------------------------------------------------------- |
   | PowerShell | `New-Item -ItemType SymbolicLink -Target "runtime" -Path "$Env:AppData\helix\runtime"` |
   | Cmd        | `cd %appdata%\helix` <br/> `mklink /D runtime "%userprofile%\src\helix\runtime"`       |

   > 💡 On Windows, creating a symbolic link may require running PowerShell or
   > Cmd as an administrator.

## Validating the Installation

To make sure everything is set up as expected you should run the Helix health
check:

```sh
hx --health
```

For more information on the health check results refer to
[Health check](https://github.com/helix-editor/helix/wiki/Healthcheck).
