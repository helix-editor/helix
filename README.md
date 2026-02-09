<div align="center">

<h1>
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="logo_dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="logo_light.svg">
  <img alt="Silicon" height="128" src="logo_light.svg">
</picture>
</h1>

</div>

> My personal fork of [silicon-editor/silicon](https://github.com/silicon-editor/silicon)

![Screenshot](./screenshot.png)

# Install

**macOS / Linux:**

```sh
curl -sSf https://raw.githubusercontent.com/Rani367/silicon/master/install.sh | sh
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/Rani367/silicon/master/install.ps1 | iex
```

The scripts handle everything: prerequisites, removing old installations, cloning, building, and runtime setup. Re-run to update.

# Features

- Vim-like modal editing
- Multiple selections
- Built-in language server support
- Smart, incremental syntax highlighting and code editing via tree-sitter

For documentation, see the upstream [website](https://silicon-editor.com) and [docs](https://docs.silicon-editor.com/).
