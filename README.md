<div align="center">

<h1>
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="logo_dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="logo_light.svg">
  <img alt="Helix" height="128" src="logo_light.svg">
</picture>
</h1>

</div>

> Personal fork of [helix-editor/helix](https://github.com/helix-editor/helix) — a [Kakoune](https://github.com/mawww/kakoune) / [Neovim](https://github.com/neovim/neovim) inspired editor, written in Rust.

![Screenshot](./screenshot.png)

# Install

**macOS / Linux:**

```sh
curl -sSf https://raw.githubusercontent.com/Rani367/helix/master/install.sh | sh
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/Rani367/helix/master/install.ps1 | iex
```

The scripts handle everything: prerequisites, removing old installations, cloning, building, and runtime setup. Re-run to update.

# Features

- Vim-like modal editing
- Multiple selections
- Built-in language server support
- Smart, incremental syntax highlighting and code editing via tree-sitter

For documentation, see the upstream [website](https://helix-editor.com) and [docs](https://docs.helix-editor.com/).
