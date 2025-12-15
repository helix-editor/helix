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

### Panel Controls

| Key | Description |
|-----|-------------|
| `Ctrl-\` | Return focus to the editor |
| `Ctrl-PageDown` | Switch to next terminal tab |
| `Ctrl-PageUp` | Switch to previous terminal tab |
| `Ctrl-Shift-T` | Create new terminal tab |
| `Ctrl-W` | Close current terminal tab |

### Terminal Input

When the terminal is focused, most keys are sent directly to the shell. This includes:

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
