# Key Remapping

One-way key remapping is supported via a simple TOML configuration file.

To remap keys, write a `keymap.toml` file in your `helix` configuration
directory (default `~/.config/helix` in Linux systems) with a structure like
this:

```toml
# At most one section each of 'Normal', 'Insert' and 'Select'
[Normal]
a = "move_char_left" # Maps the 'a' key to the move_char_left command
w = "move_line_up" # Maps the 'w' key move_line_up
C-S-Esc = "select_line" # Maps Control-Shift-Escape to select_line

[Insert]
A-x = "normal_mode" # Maps Alt-X to enter normal mode
```

Control, Shift and Alt modifiers are encoded respectively with the prefixes
`C-`, `S-` and `A-`. Special keys are encoded as follows:

* Backspace => "backspace"
* Space => "space"
* Return/Enter => "ret"
* < => "lt"
* > => "gt"
* + => "plus"
* - => "minus"
* ; => "semicolon"
* % => "percent"
* Left => "left"
* Right => "right"
* Up => "up"
* Home => "home"
* End => "end"
* Page Up => "pageup"
* Page Down => "pagedown"
* Tab => "tab"
* Back Tab => "backtab"
* Delete => "del"
* Insert => "ins"
* Null => "null"
* Escape => "esc"

Commands can be found in the source code at `../../helix-term/src/commands.rs`
