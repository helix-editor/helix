<div align="center">

<h1>
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="logo_dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="logo_light.svg">
  <img alt="Helix" height="128" src="logo_light.svg">
</picture>
</h1>

[![Build status](https://github.com/helix-editor/helix/actions/workflows/build.yml/badge.svg)](https://github.com/helix-editor/helix/actions)
[![GitHub Release](https://img.shields.io/github/v/release/helix-editor/helix)](https://github.com/helix-editor/helix/releases/latest)
[![Documentation](https://shields.io/badge/-documentation-452859)](https://docs.helix-editor.com/)
[![GitHub contributors](https://img.shields.io/github/contributors/helix-editor/helix)](https://github.com/helix-editor/helix/graphs/contributors)
[![Matrix Space](https://img.shields.io/matrix/helix-community:matrix.org)](https://matrix.to/#/#helix-community:matrix.org)

</div>

![Screenshot](./screenshot.png)

A [Kakoune](https://github.com/mawww/kakoune) / [Neovim](https://github.com/neovim/neovim) inspired editor, written in Rust.

The editing model is very heavily based on Kakoune; during development I found
myself agreeing with most of Kakoune's design decisions.

For more information, see the [website](https://helix-editor.com) or
[documentation](https://docs.helix-editor.com/).

All shortcuts/keymaps can be found [in the documentation on the website](https://docs.helix-editor.com/keymap.html).

[Troubleshooting](https://github.com/helix-editor/helix/wiki/Troubleshooting)

# Features

- Vim-like modal editing
- Multiple selections
- Built-in language server support
- Smart, incremental syntax highlighting and code editing via tree-sitter

Although it's primarily a terminal-based editor, I am interested in exploring
a custom renderer (similar to Emacs) using wgpu or skulpin.

Note: Only certain languages have indentation definitions at the moment. Check
`runtime/queries/<lang>/` for `indents.scm`.

# Installation

[Installation documentation](https://docs.helix-editor.com/install.html).

[![Packaging status](https://repology.org/badge/vertical-allrepos/helix-editor.svg?exclude_unsupported=1)](https://repology.org/project/helix-editor/versions)

# `ARMv6l` Support

This fork is my (@NeilPandya) attempt to get Helix working on my Raspberry Pi Zero Wireless. I'm not sure if there're pre-existing branches dedicated towards legacy 32-bit ARM support, but I'm not aware of any. After some [poking around](https://github.com/helix-editor/helix/discussions/5841#discussioncomment-4876888), I'm not sure if this will ever be merged upstream, but I'm documenting my progress here.

## Building for 32-bit ARM

I'm using a Raspberry Pi Zero Wireless, which is a 32-bit `ARMv6l` device. I'm not sure if this will work on other devices; this is fork is solely meant for Raspberry Pi Zero Wireless devices.

### Prerequisites

I'm using `buildx` to build the container. I've included the `Dockerfile` and some convenience bash scripts to help with the build process. They will install the necessary packages and 32-bit libraries to build Helix.

### Building


```bash
# Clone the repo
git clone https://github.com/NeilPandya/helix-armv6l.git

# Change into the directory
cd helix-armv6l

# Change the -t argument to whatever you want to name the image; I have helix here, but you can use nano or any editor of your choice to change it.
hx ./build-image.sh

# Build the image
./build-image.sh

# Alter the ./run-container.sh script to use the image you just built.
hx ./run-container.sh

# Run the container
./run-container.sh
```
```bash
# Once inside the container, build Helix
./build-helix.sh
```

# Contributing

Contributing guidelines can be found [here](./docs/CONTRIBUTING.md).

# Getting help

Your question might already be answered on the [FAQ](https://github.com/helix-editor/helix/wiki/FAQ).

Discuss the project on the community [Matrix Space](https://matrix.to/#/#helix-community:matrix.org) (make sure to join `#helix-editor:matrix.org` if you're on a client that doesn't support Matrix Spaces yet).

# Credits

Thanks to [@jakenvac](https://github.com/jakenvac) for designing the logo!
