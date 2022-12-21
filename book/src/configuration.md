# Configuration

To override global configuration parameters, create a `config.toml` file located in your config directory:

* Linux and Mac: `~/.config/helix/config.toml`
* Windows: `%AppData%\helix\config.toml`

> Hint: You can easily open the config file by typing `:config-open` within Helix normal mode.

Example config:

```toml
theme = "onedark"

[editor]
line-number = "relative"
mouse = false

[editor.cursor-shape]
insert = "bar"
normal = "block"
select = "underline"

[editor.file-picker]
hidden = false
```

You may also specify a file to use for configuration with the `-c` or
`--config` CLI argument: `hx -c path/to/custom-config.toml`.

It is also possible to trigger configuration file reloading by sending the `USR1`
signal to the helix process, e.g. via `pkill -USR1 hx`. This is only supported 
on unix operating systems.

## Editor

### `[editor]` Section

| Key | Description | Default |
|--|--|---------|
| `scrolloff` | Number of lines of padding around the edge of the screen when scrolling. | `5` |
| `mouse` | Enable mouse mode. | `true` |
| `middle-click-paste` | Middle click paste support. | `true` |
| `scroll-lines` | Number of lines to scroll per scroll wheel step. | `3` |
| `shell` | Shell to use when running external commands. | Unix: `["sh", "-c"]`<br/>Windows: `["cmd", "/C"]` |
| `line-number` | Line number display: `absolute` simply shows each line's number, while `relative` shows the distance from the current line. When unfocused or in insert mode, `relative` will still show absolute line numbers. | `absolute` |
| `cursorline` | Highlight all lines with a cursor. | `false` |
| `cursorcolumn` | Highlight all columns with a cursor. | `false` |
| `gutters` | Gutters to display: Available are `diagnostics` and `diff` and `line-numbers` and `spacer`, note that `diagnostics` also includes other features like breakpoints, 1-width padding will be inserted if gutters is non-empty | `["diagnostics", "spacer", "line-numbers", "spacer", "diff"]` |
| `auto-completion` | Enable automatic pop up of auto-completion. | `true` |
| `auto-format` | Enable automatic formatting on save. | `true` |
| `auto-save` | Enable automatic saving on focus moving away from Helix. Requires [focus event support](https://github.com/helix-editor/helix/wiki/Terminal-Support) from your terminal. | `false` |
| `idle-timeout` | Time in milliseconds since last keypress before idle timers trigger. Used for autocompletion, set to 0 for instant. | `400` |
| `completion-trigger-len` | The min-length of word under cursor to trigger autocompletion | `2` |
| `auto-info` | Whether to display infoboxes | `true` |
| `true-color` | Set to `true` to override automatic detection of terminal truecolor support in the event of a false negative. | `false` |
| `rulers` | List of column positions at which to display the rulers. Can be overridden by language specific `rulers` in `languages.toml` file. | `[]` |
| `bufferline` | Renders a line at the top of the editor displaying open buffers. Can be `always`, `never` or `multiple` (only shown if more than one buffer is in use) | `never` |
| `color-modes` | Whether to color the mode indicator with different colors depending on the mode itself | `false` |

### `[editor.statusline]` Section

Allows configuring the statusline at the bottom of the editor.

The configuration distinguishes between three areas of the status line:

`[ ... ... LEFT ... ... | ... ... ... ... CENTER ... ... ... ... | ... ... RIGHT ... ... ]`

Statusline elements can be defined as follows:

```toml
[editor.statusline]
left = ["mode", "spinner"]
center = ["file-name"]
right = ["diagnostics", "selections", "position", "file-encoding", "file-line-ending", "file-type"]
separator = "│"
mode.normal = "NORMAL"
mode.insert = "INSERT"
mode.select = "SELECT"
```
The `[editor.statusline]` key takes the following sub-keys:

| Key           | Description | Default |
| ---           | ---         | ---     |
| `left`        | A list of elements aligned to the left of the statusline | `["mode", "spinner", "file-name"]` |
| `center`      | A list of elements aligned to the middle of the statusline | `[]` |
| `right`       | A list of elements aligned to the right of the statusline | `["diagnostics", "selections", "position", "file-encoding"]` |
| `separator`   | The character used to separate elements in the statusline | `"│"` |
| `mode.normal` | The text shown in the `mode` element for normal mode | `"NOR"` |
| `mode.insert` | The text shown in the `mode` element for insert mode | `"INS"` |
| `mode.select` | The text shown in the `mode` element for select mode | `"SEL"` |

The following statusline elements can be configured:

| Key    | Description |
| ------ | ----------- |
| `mode` | The current editor mode (`mode.normal`/`mode.insert`/`mode.select`) |
| `spinner` | A progress spinner indicating LSP activity |
| `file-name` | The path/name of the opened file |
| `file-encoding` | The encoding of the opened file if it differs from UTF-8 |
| `file-line-ending` | The file line endings (CRLF or LF) |
| `total-line-numbers` | The total line numbers of the opened file |
| `file-type` | The type of the opened file |
| `diagnostics` | The number of warnings and/or errors |
| `workspace-diagnostics` | The number of warnings and/or errors on workspace |
| `selections` | The number of active selections |
| `primary-selection-length` | The number of characters currently in primary selection |
| `position` | The cursor position |
| `position-percentage` | The cursor position as a percentage of the total number of lines |
| `separator` | The string defined in `editor.statusline.separator` (defaults to `"│"`) |
| `spacer` | Inserts a space between elements (multiple/contiguous spacers may be specified) |

### `[editor.lsp]` Section

| Key                   | Description                                                 | Default |
| ---                   | -----------                                                 | ------- |
| `display-messages`    | Display LSP progress messages below statusline[^1]          | `false` |
| `auto-signature-help` | Enable automatic popup of signature help (parameter hints)  | `true`  |
| `display-signature-help-docs` | Display docs under signature help popup             | `true`  |

[^1]: By default, a progress spinner is shown in the statusline beside the file path.

### `[editor.cursor-shape]` Section

Defines the shape of cursor in each mode. Note that due to limitations
of the terminal environment, only the primary cursor can change shape.
Valid values for these options are `block`, `bar`, `underline`, or `hidden`.

| Key      | Description                                | Default |
| ---      | -----------                                | ------- |
| `normal` | Cursor shape in [normal mode][normal mode] | `block` |
| `insert` | Cursor shape in [insert mode][insert mode] | `block` |
| `select` | Cursor shape in [select mode][select mode] | `block` |

[normal mode]: ./keymap.md#normal-mode
[insert mode]: ./keymap.md#insert-mode
[select mode]: ./keymap.md#select--extend-mode

### `[editor.file-picker]` Section

Sets options for file picker and global search. All but the last key listed in
the default file-picker configuration below are IgnoreOptions: whether hidden
files and files listed within ignore files are ignored by (not visible in) the
helix file picker and global search. There is also one other key, `max-depth`
available, which is not defined by default.

All git related options are only enabled in a git repository.

| Key | Description | Default |
|--|--|---------|
|`hidden` | Enables ignoring hidden files. | true
|`parents` | Enables reading ignore files from parent directories. | true
|`ignore` | Enables reading `.ignore` files. | true
|`git-ignore` | Enables reading `.gitignore` files. | true
|`git-global` | Enables reading global .gitignore, whose path is specified in git's config: `core.excludefile` option. | true
|`git-exclude` | Enables reading `.git/info/exclude` files. | true
|`max-depth` | Set with an integer value for maximum depth to recurse. | Defaults to `None`.

### `[editor.auto-pairs]` Section

Enables automatic insertion of pairs to parentheses, brackets, etc. Can be a
simple boolean value, or a specific mapping of pairs of single characters.

To disable auto-pairs altogether, set `auto-pairs` to `false`:

```toml
[editor]
auto-pairs = false # defaults to `true`
```

The default pairs are <code>(){}[]''""``</code>, but these can be customized by
setting `auto-pairs` to a TOML table:

```toml
[editor.auto-pairs]
'(' = ')'
'{' = '}'
'[' = ']'
'"' = '"'
'`' = '`'
'<' = '>'
```

Additionally, this setting can be used in a language config. Unless
the editor setting is `false`, this will override the editor config in
documents with this language.

Example `languages.toml` that adds <> and removes ''

```toml
[[language]]
name = "rust"

[language.auto-pairs]
'(' = ')'
'{' = '}'
'[' = ']'
'"' = '"'
'`' = '`'
'<' = '>'
```

### `[editor.search]` Section

Search specific options.

| Key | Description | Default |
|--|--|---------|
| `smart-case` | Enable smart case regex searching (case insensitive unless pattern contains upper case characters) | `true` |
| `wrap-around`| Whether the search should wrap after depleting the matches | `true` |

### `[editor.whitespace]` Section

Options for rendering whitespace with visible characters. Use `:set whitespace.render all` to temporarily enable visible whitespace.

| Key | Description | Default |
|-----|-------------|---------|
| `render` | Whether to render whitespace. May either be `"all"` or `"none"`, or a table with sub-keys `space`, `tab`, and `newline`. | `"none"` |
| `characters` | Literal characters to use when rendering whitespace. Sub-keys may be any of `tab`, `space`, `nbsp`, `newline` or `tabpad` | See example below |

Example

```toml
[editor.whitespace]
render = "all"
# or control each character
[editor.whitespace.render]
space = "all"
tab = "all"
newline = "none"

[editor.whitespace.characters]
space = "·"
nbsp = "⍽"
tab = "→"
newline = "⏎"
tabpad = "·" # Tabs will look like "→···" (depending on tab width)
```

### `[editor.indent-guides]` Section

Options for rendering vertical indent guides.

| Key           | Description                                             | Default |
| ---           | ---                                                     | ---     |
| `render`      | Whether to render indent guides.                        | `false` |
| `character`   | Literal character to use for rendering the indent guide | `│`     |
| `skip-levels` | Number of indent levels to skip                         | `0`     |

Example:

```toml
[editor.indent-guides]
render = true
character = "╎" # Some characters that work well: "▏", "┆", "┊", "⸽"
skip-levels = 1
```
