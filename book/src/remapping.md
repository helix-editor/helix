# Key Remapping

One-way key remapping is temporarily supported via a simple TOML configuration
file. (More powerful solutions such as rebinding via commands will be
available in the future).

To remap keys, write a `config.toml` file in your `helix` configuration
directory (default `~/.config/helix` in Linux systems) with a structure like
this:

```toml
# At most one section each of 'keys.normal', 'keys.insert' and 'keys.select'
[keys.normal]
C-s = ":w" # Maps the Control-s to the typable command :w which is an alias for :write (save file)
C-o = ":open ~/.config/helix/config.toml" # Maps the Control-o to opening of the helix config file
a = "move_char_left" # Maps the 'a' key to the move_char_left command
w = "move_line_up" # Maps the 'w' key move_line_up
"C-S-esc" = "extend_line" # Maps Control-Shift-Escape to extend_line
g = { a = "code_action" } # Maps `ga` to show possible code actions
"ret" = ["open_below", "normal_mode"] # Maps the enter key to open_below then re-enter normal mode

[keys.insert]
"A-x" = "normal_mode" # Maps Alt-X to enter normal mode
j = { k = "normal_mode" } # Maps `jk` to exit insert mode
```
> NOTE: Typable commands can also be remapped, remember to keep the `:` prefix to indicate it's a typable command.

Control, Shift and Alt modifiers are encoded respectively with the prefixes
`C-`, `S-` and `A-`. Special keys are encoded as follows:

| Key name     | Representation |
| ---          | ---            |
| Backspace    | `"backspace"`  |
| Space        | `"space"`      |
| Return/Enter | `"ret"`        |
| \-           | `"minus"`      |
| Left         | `"left"`       |
| Right        | `"right"`      |
| Up           | `"up"`         |
| Down         | `"down"`       |
| Home         | `"home"`       |
| End          | `"end"`        |
| Page Up      | `"pageup"`     |
| Page Down    | `"pagedown"`   |
| Tab          | `"tab"`        |
| Delete       | `"del"`        |
| Insert       | `"ins"`        |
| Null         | `"null"`       |
| Escape       | `"esc"`        |

Keys can be disabled by binding them to the `no_op` command.

Commands can be found at [Keymap](https://docs.helix-editor.com/keymap.html) Commands.
> Commands can also be found in the source code at [`helix-term/src/commands.rs`](https://github.com/helix-editor/helix/blob/master/helix-term/src/commands.rs) at the invocation of `static_commands!` macro and the `TypableCommandList`.
