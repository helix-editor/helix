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
"ret" = ["open_below", "normal_mode"] # Maps the enter key to open_below then re-enter normal mode

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
| Down         | `"down"`       |
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

## Suggestions
These are some keys which can be re-mapped without clashes with other commands.

### Normal Mode

> nums: `1 2 3 4 5 6 7 8 9 0`
> arrow: `left right up down`
> KEYS: All captial alphabet keys

|`H`  |`M`      |
|`@`  |`C-~`    |
|`#`  |`C-nums` |
|`^`  |`C--`    |
|`-`  |`C-=`    |
|`+`  |`C-q`    |
|`\`  |`C-e`    |
|`.`  |`C-r`    |
|`D`  |`C-t`    |
|`L`  |`C-y`    |
|`V`  |`C-p`    |
|`C-j`|`C-[`    |
|`C-k`|`C-]`    |
|`C-l`|`C-\`    |
|`C-;`|`C-{`    |
|`C-'`|`C-}`    |
|`C-:`|`C-|`    |
|`C-"`|`C-g`    |
|`C-v`|`C-h`    |
|`C-b`|`C-.`    |
|`C-n`|`C-/`    |
|`C-m`|`C-<`    |
|`C->`|`C-arrow`|
|`C-/`|`C-KEYS` |
|`C-?`|`C-!`    |
|`C-@`|`C-^`    |
|`C-#`|`C-&`    |
|`C-$`|`C-*`    |
|`C-%`|`C-(`    |
|`C-_`|`C-+`    |
|`A--`|`A-nums` |
|`A-=`|`A-\`    |
|`A-_`|`A-a`    |
|`A-+`|`A-d`    |
|`A-~`|`A-f`    |
|`A-q`|`A-g`    |
|`A-w`|`A-h`    |
|`A-e`|`A-j`    |
|`A-r`|`A-k`    |
|`A-t`|`A-l`    |
|`A-y`|`A-:`    |
|`A-u`|`A-'`    |
|`A-i`|`A-"`    |
|`A-o`|`A-z`    |
|`A-p`|`A-x`    |
|`A-[`|`A-c`    |
|`A-]`|`A-v`    |
|`A-n`|`A-KEYS` [^1]|
|`A-m`|`A-@`    |
|`A-/`|`A-#`    |
|`A-<`|`A-$`    |
|`A->`|`A-%`    |
|`A-?`|`A-\`    |
|`A-{`|`A-}`    |
|`A-^`|`A-*`    |
|`A-&`|`A-arrow`|
|`S-arrow`|`C-arrow`|

[^1]: All KEYS other than U, K and C
