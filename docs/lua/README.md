# Lua Configuration

Silicon uses Lua 5.4 for all configuration. Your config lives in `~/.config/silicon/init.lua` (global) and optionally `.silicon/init.lua` (per-workspace, overrides global).

## Quick Start

```lua
-- ~/.config/silicon/init.lua

si.theme.set("catppuccin_mocha")

si.config.scrolloff = 8
si.config.mouse = true
si.config.line_number = "relative"
si.config.cursorline = true
si.config.true_color = true
si.config.bufferline = "multiple"
si.config.indent_guides = { render = true, character = "│" }

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

Platform-conditional config works naturally:

```lua
if si.platform == "macos" then
    si.config.shell = { "/bin/zsh", "-c" }
end
```

## Editor Settings — `si.config.*`

Set fields directly on `si.config`. Only fields you set override defaults — everything else keeps its default value.

### Boolean Fields

| Field | Default | Description |
|-------|---------|-------------|
| `mouse` | `true` | Enable mouse support |
| `cursorline` | `false` | Highlight current line |
| `cursorcolumn` | `false` | Highlight current column |
| `middle_click_paste` | `true` | Paste on middle click |
| `auto_completion` | `true` | Show completion menu automatically |
| `path_completion` | `true` | Complete file paths |
| `auto_format` | `true` | Format on save |
| `preview_completion_insert` | `true` | Preview completion before accepting |
| `completion_replace` | `false` | Replace word on completion (vs insert) |
| `continue_comments` | `true` | Continue comment tokens on newline |
| `auto_info` | `false` | Show info popup automatically |
| `true_color` | `false` | Force 24-bit color |
| `undercurl` | `false` | Use undercurl for diagnostics |
| `color_modes` | `false` | Color the mode indicator |
| `insert_final_newline` | `true` | Ensure files end with newline |
| `atomic_save` | `true` | Write to temp file then rename |
| `trim_final_newlines` | `false` | Remove trailing blank lines on save |
| `trim_trailing_whitespace` | `false` | Remove trailing whitespace on save |
| `editor_config` | `true` | Respect `.editorconfig` files |
| `rainbow_brackets` | `false` | Colorize matching brackets |
| `commandline` | `false` | Use bottom command line |

### Number Fields

| Field | Default | Description |
|-------|---------|-------------|
| `scrolloff` | `5` | Minimum lines above/below cursor |
| `scroll_lines` | `3` | Lines to scroll per scroll event |
| `completion_trigger_len` | `2` | Chars before completion triggers |
| `text_width` | `80` | Text width for formatting/wrapping |

### Duration Fields (milliseconds)

| Field | Default | Description |
|-------|---------|-------------|
| `idle_timeout` | `250` | Idle time before background tasks |
| `completion_timeout` | `250` | Timeout for completion requests |

### String Enum Fields

| Field | Default | Options |
|-------|---------|---------|
| `line_number` | `"relative"` | `"absolute"`, `"relative"` |
| `bufferline` | `"never"` | `"never"`, `"always"`, `"multiple"` |
| `popup_border` | `"none"` | `"none"`, `"all"`, `"popup"`, `"menu"` |
| `default_line_ending` | `"native"` | `"native"`, `"lf"`, `"crlf"` |
| `indent_heuristic` | `"hybrid"` | `"simple"`, `"tree-sitter"`, `"hybrid"` |
| `end_of_line_diagnostics` | `"hint"` | `"disable"`, `"hint"`, `"info"`, `"warning"`, `"error"` |
| `clipboard_provider` | (auto) | `"pasteboard"`, `"wayland"`, `"xclip"`, `"xsel"`, `"win32-yank"`, `"tmux"`, `"termux"`, `"none"` |
| `kitty_keyboard_protocol` | `"auto"` | `"auto"`, `"disabled"`, `"enabled"` |

### Vector Fields

```lua
si.config.shell = { "/bin/zsh", "-c" }          -- shell command + args
si.config.rulers = { 80, 120 }                   -- column rulers
si.config.workspace_lsp_roots = {}               -- additional LSP root dirs
si.config.jump_label_alphabet = "asdfjkl"        -- chars for jump labels (string → char array)
```

### Character Fields

```lua
si.config.default_yank_register = "+"            -- single character
```

### Nested Table Fields

#### Indent Guides

```lua
si.config.indent_guides = {
    render = true,       -- show indent guides
    character = "│",     -- guide character
    skip_levels = 0,     -- levels to skip
}
```

#### LSP

```lua
si.config.lsp = {
    enable = true,
    display_inlay_hints = false,
    display_progress_messages = false,
    display_messages = false,
    auto_signature_help = true,
    display_signature_help_docs = true,
    display_color_swatches = true,
    snippets = true,
    goto_reference_include_declaration = true,
    inlay_hints_length_limit = nil,  -- optional max length
}
```

#### Search

```lua
si.config.search = {
    smart_case = true,
    wrap_around = true,
}
```

#### Statusline

```lua
si.config.statusline = {
    left = { "mode", "spinner", "file-name", "read-only-indicator", "file-modification-indicator" },
    center = {},
    right = { "diagnostics", "selections", "register", "position", "file-encoding" },
    separator = "│",
    mode = {},           -- per-mode overrides
}
```

#### Cursor Shape

```lua
si.config.cursor_shape = {
    normal = "block",     -- "block", "bar", "underline", "hidden"
    insert = "bar",
    select = "underline",
}
```

#### Soft Wrap

```lua
si.config.soft_wrap = {
    enable = true,
    max_wrap = nil,              -- max extra visual lines
    max_indent_retain = nil,     -- max indent to preserve
    wrap_indicator = "",
    wrap_at_text_width = false,
}
```

#### Auto Save

```lua
si.config.auto_save = {
    focus_lost = false,
    after_delay = { enable = false, timeout = 3000 },
}
```

#### Word Completion

```lua
si.config.word_completion = {
    enable = true,
    trigger_length = 2,
}
```

#### Whitespace

```lua
-- Simple: render all whitespace
si.config.whitespace = { render = "all" }   -- "none", "all"

-- Detailed:
si.config.whitespace = {
    render = {
        space = "all",      -- "none", "all", "trailing"
        tab = "all",
        nbsp = "all",
        newline = "none",
    },
    characters = {
        space = "·",
        tab = "→",
        nbsp = "⍽",
        newline = "⏎",
        tabpad = "·",
    },
}
```

#### Inline Diagnostics

```lua
si.config.inline_diagnostics = {
    cursor_line = "warning",   -- "disable", "hint", "info", "warning", "error"
    other_lines = "disable",
    min_diagnostic_width = 40,
    prefix_len = 1,
    max_wrap = 20,
    max_diagnostics = 10,
}
```

#### Gutters

```lua
-- Simple: just a layout list
si.config.gutters = { "diagnostics", "spacer", "line-numbers", "diff" }

-- Full:
si.config.gutters = {
    layout = { "diagnostics", "spacer", "line-numbers", "diff" },
    line_numbers = { min_width = 3 },
}
```

#### Auto Pairs

```lua
-- Enable/disable
si.config.auto_pairs = true

-- Custom pairs
si.config.auto_pairs = { ["("] = ")", ["{"] = "}", ["["] = "]" }
```

#### Smart Tab

```lua
si.config.smart_tab = false                           -- disable
si.config.smart_tab = true                            -- enable with defaults
si.config.smart_tab = { enable = true, supersede_menu = false }
```

#### File Picker

```lua
si.config.file_picker = {
    hidden = true,
    follow_symlinks = true,
    deduplicate_links = true,
    parents = true,
    ignore = true,
    git_ignore = true,
    git_global = true,
    git_exclude = true,
    max_depth = nil,
}
```

#### Buffer Picker

```lua
si.config.buffer_picker = { start_position = "current" }  -- "current" or "previous"
```

#### Terminal

```lua
si.config.terminal = { command = "/bin/zsh", args = {} }
si.config.terminal = false  -- disable built-in terminal
```

---

## Keybindings — `si.keymap`

User keybindings merge on top of built-in defaults. Only keys you define are overridden.

### `si.keymap.set(mode, key, action)`

```lua
-- Simple command
si.keymap.set("normal", "j", "move_line_down")
si.keymap.set("normal", "C-a", "select_all")     -- Ctrl+A
si.keymap.set("insert", "S-Tab", "unindent")      -- Shift+Tab
si.keymap.set("normal", "A-f", "move_next_word_end")  -- Alt+F

-- Command sequence (multiple commands on one key)
si.keymap.set("normal", "Q", { "select_all", "yank" })

-- Submenu (key chord / nested menu)
si.keymap.set("normal", "g", {
    label = "goto",
    d = "goto_definition",
    r = "goto_reference",
    i = "goto_implementation",
})

-- Nested submenus
si.keymap.set("normal", "space", {
    label = "space",
    f = "file_picker",
    b = "buffer_picker",
    w = {
        label = "window",
        s = "hsplit",
        v = "vsplit",
        q = "wclose",
    },
})

-- Sticky submenu (stays open after each key)
si.keymap.set("normal", "z", {
    label = "view",
    is_sticky = true,
    j = "scroll_down",
    k = "scroll_up",
})
```

### `si.keymap.set_many(mode, mappings)`

```lua
si.keymap.set_many("normal", {
    j = "move_line_down",
    k = "move_line_up",
    ["C-s"] = "write",
})
```

### Modes

- `"normal"` — Normal mode
- `"insert"` — Insert mode
- `"select"` — Select mode

### Key Notation

| Notation | Key |
|----------|-----|
| `"j"` | j |
| `"C-a"` | Ctrl+A |
| `"S-Tab"` | Shift+Tab |
| `"A-f"` | Alt+F |
| `"space"` | Space |
| `"ret"` | Enter |
| `"backspace"` | Backspace |
| `"esc"` | Escape |
| `"tab"` | Tab |
| `"del"` | Delete |
| `"F1"`–`"F12"` | Function keys |
| `"up"`, `"down"`, `"left"`, `"right"` | Arrow keys |

Each key string must be a single key event. Multi-character chords like `gf` must use submenus: `si.keymap.set("normal", "g", { f = "goto_file" })`.

---

## Themes — `si.theme`

### `si.theme.set(name)`

Select a theme by name. Loads from `runtime/themes/` or `~/.config/silicon/themes/`.

```lua
si.theme.set("catppuccin_mocha")
```

### `si.theme.adaptive(opts)`

Auto-switch based on terminal light/dark mode.

```lua
si.theme.adaptive({
    light = "catppuccin_latte",
    dark = "catppuccin_mocha",
    fallback = "catppuccin_mocha",  -- if terminal doesn't report mode
})
```

### `si.theme.define(name, spec)`

Define a custom inline theme.

```lua
si.theme.define("my_theme", {
    inherits = "catppuccin_mocha",  -- optional parent

    ["ui.background"] = { bg = "#1a1a2e" },
    ["ui.cursor"] = { bg = "#e94560", modifiers = { "bold" } },
    ["ui.statusline"] = { fg = "#e0e0e0", bg = "#16213e" },
    ["comment"] = { fg = "#6c7a89", modifiers = { "italic" } },
    ["keyword"] = { fg = "#e94560" },
    ["string"] = { fg = "#533483" },

    palette = {
        red = "#e94560",
        blue = "#0f3460",
    },
})
```

Style spec fields: `fg`, `bg`, `modifiers` (array of `"bold"`, `"italic"`, `"underline"`, etc.), `underline` (`{ style = "curl", color = "#ff0000" }`).

---

## Languages — `si.language()` & `si.language_server()`

User definitions merge with built-in `languages.toml`. Only fields you set are overridden.

### `si.language_server(name, config)`

```lua
si.language_server("rust-analyzer", {
    command = "rust-analyzer",
    config = {
        checkOnSave = { command = "clippy" },
        cargo = { allFeatures = true },
    },
})

si.language_server("pyright", {
    command = "pyright-langserver",
    args = { "--stdio" },
})
```

The `config` subtable is passed directly to the language server — use whatever case the LSP expects (usually camelCase).

### `si.language(name, config)`

```lua
si.language("python", {
    language_servers = { "pyright" },
    auto_format = true,
    formatter = { command = "ruff", args = { "format", "-" } },
})

si.language("rust", {
    language_servers = { "rust-analyzer" },
})

-- Define a new language not in built-ins
si.language("mylang", {
    scope = "source.mylang",
    file_types = { "ml" },
    roots = { "myproject.toml" },
    comment_token = "#",
    indent = { tab_width = 2, unit = "  " },
    language_servers = { "mylang-lsp" },
})
```

---

## Runtime Constants — `si.*`

| Field | Example | Description |
|-------|---------|-------------|
| `si.platform` | `"macos"` | OS: `"macos"`, `"linux"`, `"windows"` |
| `si.config_dir` | `"/Users/me/.config/silicon"` | Config directory path |
| `si.home_dir` | `"/Users/me"` | Home directory |
| `si.hostname` | `"my-machine"` | Machine hostname |

---

## Config Locations

| Path | Purpose |
|------|---------|
| `~/.config/silicon/init.lua` | Global config |
| `.silicon/init.lua` | Workspace config (overrides global) |
| `~/.config/silicon/lua/` | Custom Lua modules (`require("mymodule")`) |
| `~/.config/silicon/themes/` | Custom theme files |

Both configs run in the same Lua VM. Workspace config fields override global config fields — only fields explicitly set in the workspace config take effect.

---

## Safety

The Lua VM is sandboxed:

- **Memory limit**: 64 MB
- **Instruction limit**: 1,000,000 instructions (catches infinite loops)
- **Safe stdlib only**: No `debug` library. `io`, `os`, `require` are available.
- **Error isolation**: Lua errors show in the statusline and fall back to defaults — they never crash the editor.

---

## Hot Reload

Config reloads automatically when you save `init.lua`. You can also:

- `:config-reload` — Reload config manually
- `SIGUSR1` — Signal-based reload (`kill -USR1 $(pgrep si)`)

Each reload creates a fresh Lua VM — no stale state carries over.

---

## Migrating from TOML

If you have an existing `config.toml` but no `init.lua`, Silicon will offer to convert automatically on startup. The conversion:

1. Reads `config.toml` and `languages.toml`
2. Generates equivalent `init.lua`
3. Backs up originals as `*.toml.bak`

Key differences from TOML config:
- `kebab-case` keys become `snake_case` (e.g., `line-number` → `line_number`)
- `[editor]` section becomes `si.config.*`
- `[keys.normal]` section becomes `si.keymap.set("normal", ...)`
- `theme = "name"` becomes `si.theme.set("name")`
