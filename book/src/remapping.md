## Key remapping

Helix currently supports one-way key remapping through a simple TOML configuration
file. (More powerful solutions such as rebinding via commands will be
available in the future).

To remap keys, create a `config.toml` file in your `helix` configuration
directory (default `~/.config/helix` on Linux systems) with a structure like
this:

```toml
# At most one section each of 'keys.normal', 'keys.insert' and 'keys.select'
[keys.normal]
C-s = ":w" # Maps Ctrl-s to the typable command :w which is an alias for :write (save file)
C-o = ":open ~/.config/helix/config.toml" # Maps Ctrl-o to opening of the helix config file
a = "move_char_left" # Maps the 'a' key to the move_char_left command
w = "move_line_up" # Maps the 'w' key move_line_up
"C-S-esc" = "extend_line" # Maps Ctrl-Shift-Escape to extend_line
g = { a = "code_action" } # Maps `ga` to show possible code actions
"ret" = ["open_below", "normal_mode"] # Maps the enter key to open_below then re-enter normal mode

[keys.insert]
"A-x" = "normal_mode"     # Maps Alt-X to enter normal mode
j = { k = "normal_mode" } # Maps `jk` to exit insert mode
```

## Minor modes

Minor modes are accessed by pressing a key (usually from normal mode), giving access to dedicated bindings. Bindings
can be modified or added by nesting definitions.

```toml
[keys.insert.j]
k = "normal_mode" # Maps `jk` to exit insert mode

[keys.normal.g]
a = "code_action" # Maps `ga` to show possible code actions

# invert `j` and `k` in view mode
[keys.normal.z]
j = "scroll_up"
k = "scroll_down"

# create a new minor mode bound to `+`
[keys.normal."+"]
m = ":run-shell-command make"
c = ":run-shell-command cargo build"
t = ":run-shell-command cargo test"
```

## Special keys and modifiers

Ctrl, Shift and Alt modifiers are encoded respectively with the prefixes
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

## Commands

There are three kinds of commands that can be used in keymaps:

* Static commands: commands like `move_char_right` which are usually bound to
  keys and used for movement and editing. A list of static commands is
  available in the [Keymap](./keymap.html) documentation and in the source code
  in [`helix-term/src/commands.rs`](https://github.com/helix-editor/helix/blob/master/helix-term/src/commands.rs)
  at the invocation of `static_commands!` macro and the `TypableCommandList`.
* Typable commands: commands that can be executed from command mode (`:`), for
  example `:write!`. See the [Commands](./commands.html) documentation for a
  list of available typeable commands.
* Macros: sequences of keys that are executed in order. These keybindings
  start with `@` and then list any number of keys to be executed. For example
  `@miw` can be used to select the surrounding word. For now, macro keybindings
  are not allowed in keybinding sequences due to limitations in the way that
  command sequences are executed.
