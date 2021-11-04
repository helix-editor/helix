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
a = "move_char_left" # Maps the 'a' key to the move_char_left command
w = "move_line_up" # Maps the 'w' key move_line_up
"C-S-esc" = "extend_line" # Maps Control-Shift-Escape to extend_line
g = { a = "code_action" } # Maps `ga` to show possible code actions

[keys.insert]
"A-x" = "normal_mode" # Maps Alt-X to enter normal mode
j = { k = "normal_mode" } # Maps `jk` to exit insert mode
```

Control, Shift and Alt modifiers are encoded respectively with the prefixes
`C-`, `S-` and `A-`. Special keys are encoded as follows:

| Key name     | Representation |
| ---          | ---            |
| Backspace    | `"backspace"`  |
| Space        | `"space"`      |
| Return/Enter | `"ret"`        |
| <            | `"lt"`         |
| \>           | `"gt"`         |
| \+           | `"plus"`       |
| \-           | `"minus"`      |
| ;            | `"semicolon"`  |
| %            | `"percent"`    |
| Left         | `"left"`       |
| Right        | `"right"`      |
| Up           | `"up"`         |
| Home         | `"home"`       |
| End          | `"end"`        |
| Page         | `"pageup"`     |
| Page         | `"pagedown"`   |
| Tab          | `"tab"`        |
| Back         | `"backtab"`    |
| Delete       | `"del"`        |
| Insert       | `"ins"`        |
| Null         | `"null"`       |
| Escape       | `"esc"`        |

Keys can be disabled by binding them to the `no_op` command.

Commands can be found in the source code at [`helix-term/src/commands.rs`](https://github.com/helix-editor/helix/blob/master/helix-term/src/commands.rs)
