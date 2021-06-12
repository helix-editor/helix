# Key Remapping

One-way key remapping is supported via a simple TOML configuration file.

To remap keys, write a `keymap.toml` file in your `helix` configuration
directory (default `~/.config/helix` in Linux systems) with a structure like
this:

```toml
# At most one section each of 'Normal', 'Insert' and 'Select'
[Normal]
a = "w" # Maps the 'a' key to 'w' (move to next word)
w = "i" # Maps the 'w' key to 'i' (enter insert mode)
C-S-Esc = "f" # Maps Control-Shift-Escape to 'f' (find char)

[Insert]
A-x = "Esc" # Maps Alt-X to 'Esc' (leave insert mode)
```

Control, Shift and Alt modifiers are encoded respectively with the prefixes
`C-`, `S-` and `A-`. Special keys are encoded as follows:

* Backspace => `Bs`
* Enter => `Enter`
* Left => `Left`
* Right => `Right`
* Up => `Up`
* Down => `Down`
* Home => `Home`
* End => `End`
* PageUp => `PageUp`
* PageDown => `PageDown`
* Tab => `Tab`
* BackTab => `BackTab`
* Delete => `Del`
* Insert => `Insert`
* Null => `Null` (No associated key)
* Esc => `Esc`
