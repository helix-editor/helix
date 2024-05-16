## Building from source

- [Configuring Helix's runtime files](#configuring-helixs-runtime-files)
  - [Linux and macOS](#linux-and-macos)
  - [Windows](#windows)
  - [Multiple runtime directories](#multiple-runtime-directories)
  - [Note to packagers](#note-to-packagers)
- [Validating the installation](#validating-the-installation)
- [Configure the desktop shortcut](#configure-the-desktop-shortcut)

Requirements:

Clone the Helix GitHub repository into a directory of your choice. The
examples in this documentation assume installation into either `~/src/` on
Linux and macOS, or `%userprofile%\src\` on Windows.

- The [Rust toolchain](https://www.rust-lang.org/tools/install)
- The [Git version control system](https://git-scm.com/)
- A C++14 compatible compiler to build the tree-sitter grammars, for example GCC or Clang

If you are using the `musl-libc` standard library instead of `glibc` the following environment variable must be set during the build to ensure tree-sitter grammars can be loaded correctly:

```sh
RUSTFLAGS="-C target-feature=-crt-static"
```

1. Clone the repository:

   ```sh
   git clone https://github.com/helix-editor/helix
   cd helix
   ```

2. Compile from source:

   ```sh
   cargo install --path helix-term --locked
   ```

   This command will create the `hx` executable and construct the tree-sitter
   grammars in the local `runtime` folder.

> 💡 If you do not want to fetch or build grammars, set an environment variable `HELIX_DISABLE_AUTO_GRAMMAR_BUILD`

> 💡 Tree-sitter grammars can be fetched and compiled if not pre-packaged. Fetch
> grammars with `hx --grammar fetch` and compile them with
> `hx --grammar build`. This will install them in
> the `runtime` directory within the user's helix config directory (more
> [details below](#multiple-runtime-directories)).

### Configuring Helix's runtime files

#### Linux and macOS

The **runtime** directory is one below the Helix source, so either export a
`HELIX_RUNTIME` environment variable to point to that directory and add it to
your `~/.bashrc` or equivalent:

```sh
export HELIX_RUNTIME=~/src/helix/runtime
```

Or, create a symbolic link:

```sh
ln -Ts $PWD/runtime ~/.config/helix/runtime
```

If the above command fails to create a symbolic link because the file exists either move `~/.config/helix/runtime` to a new location or delete it, then run the symlink command above again.

#### Windows

Either set the `HELIX_RUNTIME` environment variable to point to the runtime files using the Windows setting (search for
`Edit environment variables for your account`) or use the `setx` command in
Cmd:

```sh
setx HELIX_RUNTIME "%userprofile%\source\repos\helix\runtime"
```

> 💡 `%userprofile%` resolves to your user directory like
> `C:\Users\Your-Name\` for example.

Or, create a symlink in `%appdata%\helix\` that links to the source code directory:

| Method     | Command                                                                                |
| ---------- | -------------------------------------------------------------------------------------- |
| PowerShell | `New-Item -ItemType Junction -Target "runtime" -Path "$Env:AppData\helix\runtime"`     |
| Cmd        | `cd %appdata%\helix` <br/> `mklink /D runtime "%userprofile%\src\helix\runtime"`       |

> 💡 On Windows, creating a symbolic link may require running PowerShell or
> Cmd as an administrator.

#### Multiple runtime directories

When Helix finds multiple runtime directories it will search through them for files in the
following order:

1. `runtime/` sibling directory to `$CARGO_MANIFEST_DIR` directory (this is intended for
  developing and testing helix only).
2. `runtime/` subdirectory of OS-dependent helix user config directory.
3. `$HELIX_RUNTIME`
4. Distribution-specific fallback directory (set at compile time—not run time—
   with the `HELIX_DEFAULT_RUNTIME` environment variable)
5. `runtime/` subdirectory of path to Helix executable.

This order also sets the priority for selecting which file will be used if multiple runtime
directories have files with the same name.

#### Note to packagers

If you are making a package of Helix for end users, to provide a good out of
the box experience, you should set the `HELIX_DEFAULT_RUNTIME` environment
variable at build time (before invoking `cargo build`) to a directory which
will store the final runtime files after installation. For example, say you want
to package the runtime into `/usr/lib/helix/runtime`. The rough steps a build
script could follow are:

1. `export HELIX_DEFAULT_RUNTIME=/usr/lib/helix/runtime`
1. `cargo build --profile opt --locked --path helix-term`
1. `cp -r runtime $BUILD_DIR/usr/lib/helix/`
1. `cp target/opt/hx $BUILD_DIR/usr/bin/hx`

This way the resulting `hx` binary will always look for its runtime directory in
`/usr/lib/helix/runtime` if the user has no custom runtime in `~/.config/helix`
or `HELIX_RUNTIME`.

### Validating the installation

To make sure everything is set up as expected you should run the Helix health
check:

```sh
hx --health
```

For more information on the health check results refer to
[Health check](https://github.com/helix-editor/helix/wiki/Healthcheck).

### Configure the desktop shortcut

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
