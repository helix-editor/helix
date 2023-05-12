# Configuration

To override global configuration parameters, create a `config.toml` file located in your config directory:

- Linux and Mac: `~/.config/helix/config.toml`
- Windows: `%AppData%\helix\config.toml`

> 💡 You can easily open the config file by typing `:config-open` within Helix normal mode.

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

You can use a custom configuration file by specifying it with the `-c` or
`--config` command line argument, for example `hx -c path/to/custom-config.toml`.
Additionally, you can reload the configuration file by sending the USR1
signal to the Helix process on Unix operating systems, such as by using the command `pkill -USR1 hx`.

Finally, you can have a `config.toml` local to a project by putting it under a `.helix` directory in your repository.
Its settings will be merged with the configuration directory `config.toml` and the built-in configuration.

## Editor

### `[editor]` Section

| Key                      | Description                                                                                                                                                                                                                 | Default                                                       |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| `scrolloff`              | Number of lines of padding around the edge of the screen when scrolling                                                                                                                                                     | `5`                                                           |
| `mouse`                  | Enable mouse mode                                                                                                                                                                                                           | `true`                                                        |
| `middle-click-paste`     | Middle click paste support                                                                                                                                                                                                  | `true`                                                        |
| `scroll-lines`           | Number of lines to scroll per scroll wheel step                                                                                                                                                                             | `3`                                                           |
| `shell`                  | Shell to use when running external commands                                                                                                                                                                                 | Unix: `["sh", "-c"]`<br/>Windows: `["cmd", "/C"]`             |
| `line-number`            | Line number display: `absolute` simply shows each line's number, while `relative` shows the distance from the current line. When unfocused or in insert mode, `relative` will still show absolute line numbers              | `absolute`                                                    |
| `cursorline`             | Highlight all lines with a cursor                                                                                                                                                                                           | `false`                                                       |
| `cursorcolumn`           | Highlight all columns with a cursor                                                                                                                                                                                         | `false`                                                       |
| `gutters`                | Gutters to display: Available are `diagnostics` and `diff` and `line-numbers` and `spacer`, note that `diagnostics` also includes other features like breakpoints, 1-width padding will be inserted if gutters is non-empty | `["diagnostics", "spacer", "line-numbers", "spacer", "diff"]` |
| `auto-completion`        | Enable automatic pop up of auto-completion                                                                                                                                                                                  | `true`                                                        |
| `auto-format`            | Enable automatic formatting on save                                                                                                                                                                                         | `true`                                                        |
| `auto-save`              | Enable automatic saving on the focus moving away from Helix. Requires [focus event support](https://github.com/helix-editor/helix/wiki/Terminal-Support) from your terminal                                                 | `false`                                                       |
| `idle-timeout`           | Time in milliseconds since last keypress before idle timers trigger. Used for autocompletion, set to 0 for instant                                                                                                          | `400`                                                         |
| `completion-trigger-len` | The min-length of word under cursor to trigger autocompletion                                                                                                                                                               | `2`                                                           |
| `completion-replace`     | Set to `true` to make completions always replace the entire word and not just the part before the cursor                                                                                                                    | `false`                                                       |
| `auto-info`              | Whether to display info boxes                                                                                                                                                                                               | `true`                                                        |
| `true-color`             | Set to `true` to override automatic detection of terminal truecolor support in the event of a false negative                                                                                                                | `false`                                                       |
| `undercurl`              | Set to `true` to override automatic detection of terminal undercurl support in the event of a false negative                                                                                                                | `false`                                                       |
| `rulers`                 | List of column positions at which to display the rulers. Can be overridden by language specific `rulers` in `languages.toml` file                                                                                           | `[]`                                                          |
| `bufferline`             | Renders a line at the top of the editor displaying open buffers. Can be `always`, `never` or `multiple` (only shown if more than one buffer is in use)                                                                      | `never`                                                       |
| `color-modes`            | Whether to color the mode indicator with different colors depending on the mode itself                                                                                                                                      | `false`                                                       |
| `text-width`             | Maximum line length. Used for the `:reflow` command and soft-wrapping if `soft-wrap.wrap-at-text-width` is set                                                                                                              | `80`                                                          |
| `workspace-lsp-roots`    | Directories relative to the workspace root that are treated as LSP roots. Should only be set in `.helix/config.toml`                                                                                                        | `[]`                                                          |

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

| Key           | Description                                                | Default                                                      |
| ------------- | ---------------------------------------------------------- | ------------------------------------------------------------ |
| `left`        | A list of elements aligned to the left of the statusline   | `["mode", "spinner", "file-name"]`                           |
| `center`      | A list of elements aligned to the middle of the statusline | `[]`                                                         |
| `right`       | A list of elements aligned to the right of the statusline  | `["diagnostics", "selections", "position", "file-encoding"]` |
| `separator`   | The character used to separate elements in the statusline  | `"│"`                                                        |
| `mode.normal` | The text shown in the `mode` element for normal mode       | `"NOR"`                                                      |
| `mode.insert` | The text shown in the `mode` element for insert mode       | `"INS"`                                                      |
| `mode.select` | The text shown in the `mode` element for select mode       | `"SEL"`                                                      |

The following statusline elements can be configured:

| Key                           | Description                                                                                         |
| ----------------------------- | --------------------------------------------------------------------------------------------------- |
| `mode`                        | The current editor mode (`mode.normal`/`mode.insert`/`mode.select`)                                 |
| `spinner`                     | A progress spinner indicating LSP activity                                                          |
| `file-name`                   | The path/name of the opened file                                                                    |
| `file-base-name`              | The basename of the opened file                                                                     |
| `file-modification-indicator` | The indicator to show whether the file is modified (a `[+]` appears when there are unsaved changes) |
| `file-encoding`               | The encoding of the opened file if it differs from UTF-8                                            |
| `file-line-ending`            | The file line endings (CRLF or LF)                                                                  |
| `total-line-numbers`          | The total line numbers of the opened file                                                           |
| `file-type`                   | The type of the opened file                                                                         |
| `diagnostics`                 | The number of warnings and/or errors                                                                |
| `workspace-diagnostics`       | The number of warnings and/or errors on workspace                                                   |
| `selections`                  | The number of active selections                                                                     |
| `primary-selection-length`    | The number of characters currently in primary selection                                             |
| `position`                    | The cursor position                                                                                 |
| `position-percentage`         | The cursor position as a percentage of the total number of lines                                    |
| `separator`                   | The string defined in `editor.statusline.separator` (defaults to `"│"`)                             |
| `spacer`                      | Inserts a space between elements (multiple/contiguous spacers may be specified)                     |
| `version-control`             | The current branch name or detached commit hash of the opened workspace                             |

### `[editor.lsp]` Section

| Key                                  | Description                                                                                                           | Default |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------- | ------- |
| `enable`                             | Enables LSP integration. Setting to false will completely disable language servers regardless of language settings.   | `true`  |
| `display-messages`                   | Display LSP progress messages below statusline[^1]                                                                    | `false` |
| `auto-signature-help`                | Enable automatic popup of signature help (parameter hints)                                                            | `true`  |
| `display-inlay-hints`                | Display inlay hints[^2]                                                                                               | `false` |
| `display-signature-help-docs`        | Display docs under signature help popup                                                                               | `true`  |
| `snippets`                           | Enables snippet completions. Requires a server restart (`:lsp-restart`) to take effect after `:config-reload`/`:set`. | `true`  |
| `goto-reference-include-declaration` | Include declaration in the goto references popup.                                                                     | `true`  |

[^1]: By default, a progress spinner is shown in the statusline beside the file path.
[^2]:
    You may also have to activate them in the LSP config for them to appear, not just in Helix.
    Inlay hints in Helix are still being improved on and may be a little bit laggy/janky under some circumstances, please report any bugs you see so we can fix them!

### `[editor.cursor-shape]` Section

Defines the shape of cursor in each mode.
Valid values for these options are `block`, `bar`, `underline`, or `hidden`.

> 💡 Due to limitations of the terminal environment, only the primary cursor can
> change shape.

| Key      | Description                                | Default |
| -------- | ------------------------------------------ | ------- |
| `normal` | Cursor shape in [normal mode][normal mode] | `block` |
| `insert` | Cursor shape in [insert mode][insert mode] | `block` |
| `select` | Cursor shape in [select mode][select mode] | `block` |

[normal mode]: ./keymap.md#normal-mode
[insert mode]: ./keymap.md#insert-mode
[select mode]: ./keymap.md#select--extend-mode

### `[editor.file-picker]` Section

Set options for file picker and global search. Ignoring a file means it is
not visible in the Helix file picker and global search.

All git related options are only enabled in a git repository.

| Key                 | Description                                                                                             | Default             |
| ------------------- | ------------------------------------------------------------------------------------------------------- | ------------------- |
| `hidden`            | Enables ignoring hidden files                                                                           | true                |
| `follow-symlinks`   | Follow symlinks instead of ignoring them                                                                | true                |
| `deduplicate-links` | Ignore symlinks that point at files already shown in the picker                                         | true                |
| `parents`           | Enables reading ignore files from parent directories                                                    | true                |
| `ignore`            | Enables reading `.ignore` files                                                                         | true                |
| `git-ignore`        | Enables reading `.gitignore` files                                                                      | true                |
| `git-global`        | Enables reading global `.gitignore`, whose path is specified in git's config: `core.excludefile` option | true                |
| `git-exclude`       | Enables reading `.git/info/exclude` files                                                               | true                |
| `max-depth`         | Set with an integer value for maximum depth to recurse                                                  | Defaults to `None`. |

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

| Key           | Description                                                                                        | Default |
| ------------- | -------------------------------------------------------------------------------------------------- | ------- |
| `smart-case`  | Enable smart case regex searching (case-insensitive unless pattern contains upper case characters) | `true`  |
| `wrap-around` | Whether the search should wrap after depleting the matches                                         | `true`  |

### `[editor.whitespace]` Section

Options for rendering whitespace with visible characters. Use `:set whitespace.render all` to temporarily enable visible whitespace.

| Key          | Description                                                                                                                     | Default           |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| `render`     | Whether to render whitespace. May either be `"all"` or `"none"`, or a table with sub-keys `space`, `nbsp`, `tab`, and `newline` | `"none"`          |
| `characters` | Literal characters to use when rendering whitespace. Sub-keys may be any of `tab`, `space`, `nbsp`, `newline` or `tabpad`       | See example below |

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
| ------------- | ------------------------------------------------------- | ------- |
| `render`      | Whether to render indent guides                         | `false` |
| `character`   | Literal character to use for rendering the indent guide | `│`     |
| `skip-levels` | Number of indent levels to skip                         | `0`     |

Example:

```toml
[editor.indent-guides]
render = true
character = "╎" # Some characters that work well: "▏", "┆", "┊", "⸽"
skip-levels = 1
```

### `[editor.gutters]` Section

For simplicity, `editor.gutters` accepts an array of gutter types, which will
use default settings for all gutter components.

```toml
[editor]
gutters = ["diff", "diagnostics", "line-numbers", "spacer"]
```

To customize the behavior of gutters, the `[editor.gutters]` section must
be used. This section contains top level settings, as well as settings for
specific gutter components as subsections.

| Key      | Description                    | Default                                                       |
| -------- | ------------------------------ | ------------------------------------------------------------- |
| `layout` | A vector of gutters to display | `["diagnostics", "spacer", "line-numbers", "spacer", "diff"]` |

Example:

```toml
[editor.gutters]
layout = ["diff", "diagnostics", "line-numbers", "spacer"]
```

#### `[editor.gutters.line-numbers]` Section

Options for the line number gutter

| Key         | Description                             | Default |
| ----------- | --------------------------------------- | ------- |
| `min-width` | The minimum number of characters to use | `3`     |

Example:

```toml
[editor.gutters.line-numbers]
min-width = 1
```

#### `[editor.gutters.diagnostics]` Section

Currently unused

#### `[editor.gutters.diff]` Section

Currently unused

#### `[editor.gutters.spacer]` Section

Currently unused

### `[editor.soft-wrap]` Section

Options for soft wrapping lines that exceed the view width:

| Key                  | Description                                                                 | Default |
| -------------------- | --------------------------------------------------------------------------- | ------- |
| `enable`             | Whether soft wrapping is enabled.                                           | `false` |
| `max-wrap`           | Maximum free space left at the end of the line.                             | `20`    |
| `max-indent-retain`  | Maximum indentation to carry over when soft wrapping a line.                | `40`    |
| `wrap-indicator`     | Text inserted before soft wrapped lines, highlighted with `ui.virtual.wrap` | `↪ `    |
| `wrap-at-text-width` | Soft wrap at `text-width` instead of using the full viewport size.          | `false` |

Example:

```toml
[editor.soft-wrap]
enable = true
max-wrap = 25 # increase value to reduce forced mid-word wrapping
max-indent-retain = 0
wrap-indicator = ""  # set wrap-indicator to "" to hide it
```

### `[editor.explorer]` Section

Sets explorer side width and style.

| Key            | Description                                 | Default |
| -------------- | ------------------------------------------- | ------- |
| `column-width` | explorer side width                         | 30      |
| `position`     | explorer widget position, `left` or `right` | `left`  |
