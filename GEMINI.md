# About Me

I am an expert Rust programmer and an expert NVIM and an exprt Helix Editor user.

# Project Overview

This is a fork of Helix, a modern text editor written in Rust. The goal of this fork is to incorporate several pull requests from the upstream Helix repository that have not yet been merged.

The project is structured as a Rust workspace with several crates, including:
- `helix-core`: Core editor logic
- `helix-view`: The view layer of the editor
- `helix-term`: The terminal UI layer
- `helix-lsp`: Language Server Protocol support

# Building and Running

To build and install this fork of Helix, run the following command:

```bash
cargo install --path helix-term --locked
```

To run the locally built application on macOS, you may need to remove the quarantine attribute:

```bash
xattr -d com.apple.quarantine /path/to/your/app
```

# Development Conventions

The project follows standard Rust conventions. The codebase is organized into a workspace with multiple crates.

## Configuration

Helix is configured through a `config.toml` file. The `README.md` file provides details on how to configure various features, including:
- Window resizing
- Hover documentation
- Customizable color swatches

## Documentation

The project's documentation is built with `mdbook` and is located in the `book` directory. The `book/src/SUMMARY.md` file outlines the structure of the documentation.
