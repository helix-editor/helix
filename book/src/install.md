# Installing Helix

To install Helix, follow the instructions specific to your operating system.
Note that:

- To get the latest nightly version of Helix, you need to
  [build from source](./building-from-source.md).

- To take full advantage of Helix, install the language servers for your
  preferred programming languages. See the
  [wiki](https://github.com/helix-editor/helix/wiki/How-to-install-the-default-language-servers)
  for instructions.

## Pre-built binaries

Download pre-built binaries from the [GitHub Releases page](https://github.com/helix-editor/helix/releases).
The tarball contents include an `hx` binary and a `runtime` directory.

- Add the `hx` binary to your system's `$PATH` to use it from the command line.
- Copy the `runtime` directory into a location searched by `hx` (for example `~/.config/helix/runtime` on Linux/macOS).

The runtime directories searched by `hx` are shown in `hx --health`. The runtime location can be overriden via the `HELIX_RUNTIME` environment variable.

