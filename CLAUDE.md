# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a fork of Helix, a post-modern text editor written in Rust. The fork implements various additional features including an index command, local buffer search, icon support, customizable color swatches, vertical preview, rounded corners, welcome screen, picker titles, and more. Refer to the README.md for the complete list of merged pull requests.

## Architecture

Helix is structured as a Rust workspace with multiple crates:

- **helix-term**: The main terminal application crate
- **helix-core**: Core editing functionality, text manipulation, and syntax highlighting
- **helix-view**: UI components and view management
- **helix-tui**: Terminal user interface components
- **helix-lsp**: Language Server Protocol client implementation
- **helix-dap**: Debug Adapter Protocol implementation
- **helix-loader**: Runtime resource loading (grammars, queries, themes)
- **helix-vcs**: Version control system integration
- **helix-parsec**: Parser combinator utilities
- **helix-stdx**: Standard library extensions
- **helix-event**: Event handling system

The main executable is built from the `helix-term` crate, which depends on the other crates to provide a complete editor experience.

## Build Commands

```bash
# Install from source (recommended)
cargo install --path helix-term --locked

# Build for development
cargo build

# Build optimized release
cargo build --release

# Build with fat LTO optimization profile
cargo build --profile opt
```

## Testing

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p helix-core

# Run integration tests with optimized profile
cargo test --profile integration
```

## Development Tools (xtask)

Use the xtask utility for development tasks:

```bash
# Generate documentation files
cargo xtask docgen

# Check tree-sitter queries for all languages
cargo xtask query-check

# Check specific languages
cargo xtask query-check rust python

# Validate theme files
cargo xtask theme-check

# Validate specific themes
cargo xtask theme-check default gruvbox
```

## Key Configuration Features

The fork includes several customizable features:

**Window Resizing**: Configure panel resizing limits in `config.toml`:
```toml
[editor]
max-panel-width = 50      # 0 for dynamic limit
max-panel-height = 50     # 0 for dynamic limit
max-panel-width-percent = 0.8
max-panel-height-percent = 0.8
```

**Color Swatches**: Customize LSP color swatch display:
```toml
[editor.lsp]
display-color-swatches = true
color-swatches-string = "●"  # Default: "■"
```

## Key Keybindings (Fork-specific)

**Window Management:**
- `Alt+w h/l/j/k` - Resize windows
- `Alt+w f` - Toggle focus mode
- `Alt+W` - Enter sticky resize mode

**Line Movement:**
- `Ctrl+j` - Move lines down
- `Ctrl+k` - Move lines up

**Hover Documentation:**
- `Space+k` - Show hover popup
- `Space+K` - Open hover in navigable buffer

## Runtime Structure

The `runtime/` directory contains:
- `queries/`: Tree-sitter queries for syntax highlighting and navigation
- `themes/`: Color theme files
- `grammars/`: Tree-sitter grammar sources and build configurations

## Development Notes

- Helix uses Tree-sitter for syntax highlighting and code navigation
- LSP integration provides language-specific features
- The editor supports both terminal and potential GUI backends through the TUI abstraction
- Configuration uses TOML format
- Themes use a hierarchical inheritance system
- Grammar queries use Tree-sitter's S-expression syntax for pattern matching