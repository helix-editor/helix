<div align="center">
  <img alt="Silicon" src="logo.png" width="256" align="middle" style="vertical-align: middle; margin-right: 16px;">
  <h1 style="display: inline; vertical-align: middle;">Silicon</h1>
</div>

> Improved fork of [helix-editor/helix](https://github.com/helix-editor/helix)

![Screenshot](./screenshot.png)

# Install

**macOS / Linux:**

```sh
curl -fL silicon-editor.github.io/Silicon/install.sh | sh
```

**Windows (PowerShell):**

```powershell
irm silicon-editor.github.io/Silicon/install.ps1 | iex
```

The scripts handle everything: prerequisites, removing old installations, cloning, building, and runtime setup. Re-run to update.

# Features

- Vim-like modal editing
- Multiple selections
- Built-in language server support
- Smart, incremental syntax highlighting and code editing via tree-sitter
- **Lua configuration** — `~/.config/silicon/init.lua` with hot reload
- **Built-in terminal** — toggle with `Space+t`

# Configuration

Silicon is configured with Lua. Create `~/.config/silicon/init.lua`:

```lua
-- example config
si.theme.set("catppuccin_mocha")

si.config.scrolloff = 8
si.config.mouse = true
si.config.line_number = "relative"
si.config.cursorline = true

si.keymap.set("normal", "space", {
    label = "space",
    f = "file_picker",
    b = "buffer_picker",
})

si.language_server("rust-analyzer", {
    command = "rust-analyzer",
    config = { checkOnSave = { command = "clippy" } },
})
```

Config reloads automatically when you save the file. See the full [Lua configuration reference](docs/lua/README.md) for all options.

If you have an existing Helix `config.toml`, Silicon will offer to convert it automatically on first launch.

# Documentation

- [Lua configuration reference](docs/lua/README.md)
- [Architecture](docs/architecture.md)
- [Contributing](docs/CONTRIBUTING.md)
- [Built-in terminal](docs/terminal.md)
