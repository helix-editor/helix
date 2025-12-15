# Integrated Terminal

Helix includes a built-in terminal emulator that allows you to run shell commands without leaving the editor.

## Opening the Terminal

| Key | Command | Description |
|-----|---------|-------------|
| <code>Ctrl-`</code> | `:terminal-toggle` | Toggle terminal panel visibility |
| <code>Ctrl-Shift-`</code> | `:terminal-new` | Open a new terminal tab |

## Terminal Panel

The terminal opens in a panel at the bottom of the screen. Multiple terminal tabs are supported:

- **Tab bar**: Shows all open terminals at the top of the panel
- **Active terminal**: The currently focused terminal tab
- **Exit indicator**: Shows "(exited)" when the shell process has terminated

## Keybindings

When the terminal panel is focused, Helix enters **Terminal mode**. In this mode, special keybindings are available while other keys are sent directly to the shell.

### Default Terminal Mode Keybindings

| Key | Command | Description |
|-----|---------|-------------|
| `Escape` | `terminal_exit` | Return focus to the editor |
| `Ctrl-\` | `terminal_exit` | Alternative: return focus to the editor |
| `Ctrl-PageDown` | `terminal_next` | Switch to next terminal tab |
| `Ctrl-PageUp` | `terminal_prev` | Switch to previous terminal tab |
| `Ctrl-Shift-T` | `terminal_open` | Create new terminal tab |
| `Ctrl-W` | `terminal_close` | Close current terminal tab |

### Customizing Terminal Keybindings

You can customize terminal keybindings in your `config.toml` using the `[keys.terminal]` section:

```toml
[keys.terminal]
"esc" = "terminal_exit"
"C-n" = "terminal_next"
"C-p" = "terminal_prev"
"C-t" = "terminal_open"
```

### Mouse Support

You can switch between terminal tabs by clicking on the tab bar.

### Terminal Input

When the terminal is focused and a key doesn't match any terminal keybinding, it is sent directly to the shell. This includes:

- All printable characters
- Arrow keys, Home, End, Page Up/Down
- Function keys (F1-F12)
- Tab, Enter, Backspace, Escape
- Ctrl+key combinations (e.g., `Ctrl-C` for interrupt)
- Alt+key combinations

## Commands

| Command | Description |
|---------|-------------|
| `:terminal-toggle` | Toggle terminal panel |
| `:terminal-new` | Create new terminal |
| `:terminal-close` | Close current terminal |
| `:terminal-next` | Switch to next terminal |
| `:terminal-prev` | Switch to previous terminal |
| `:terminal-focus` | Focus the terminal panel |

## Configuration

The terminal uses the following settings:

- **Shell**: Uses the `$SHELL` environment variable, or `/bin/sh` as fallback
- **Working directory**: Defaults to the current document's directory or workspace root
- **Terminal type**: Set to `xterm-256color` for full color support

## Features

- **Full color support**: 256 colors and true color (24-bit RGB)
- **Text attributes**: Bold, italic, underline, inverse
- **Scrollback**: 1000 lines of scrollback history
- **Window title**: Automatically updates from shell escape sequences
- **Resize**: Terminal automatically resizes with the panel

## Tips

1. **Quick commands**: Toggle the terminal with `Ctrl-\`` to run a quick command and return to editing
2. **Multiple shells**: Open different terminals for different tasks (build, test, git, etc.)
3. **Working directory**: The terminal opens in the current file's directory by default

## Limitations

- Mouse support is limited
- Some advanced terminal features (e.g., alternate screen switching) may not be fully supported
- Copy/paste uses system clipboard integration
