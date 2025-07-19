## Themes

To use a theme add `theme = "<name>"` to the top of your [`config.toml`](./configuration.md) file, or select it during runtime using `:theme <name>`.

## Creating a theme

Create a file with the name of your theme as the file name (i.e `mytheme.toml`) and place it in your `themes` directory (i.e `~/.config/helix/themes` or `%AppData%\helix\themes` on Windows). The directory might have to be created beforehand.

> 💡 The names "default" and "base16_default" are reserved for built-in themes
> and cannot be overridden by user-defined themes.

### Overview

Each line in the theme file is specified as below:

```toml
key = { fg = "#ffffff", bg = "#000000", underline = { color = "#ff0000", style = "curl"}, modifiers = ["bold", "italic"] }
```

Where `key` represents what you want to style, `fg` specifies the foreground color, `bg` the background color, `underline` the underline `style`/`color`, and `modifiers` is a list of style modifiers. `bg`, `underline` and `modifiers` can be omitted to defer to the defaults.

To specify only the foreground color:

```toml
key = "#ffffff"
```

If the key contains a dot `'.'`, it must be quoted to prevent it being parsed as a [dotted key](https://toml.io/en/v1.0.0#keys).

```toml
"key.key" = "#ffffff"
```

For inspiration, you can find the default `theme.toml`
[here](https://github.com/helix-editor/helix/blob/master/theme.toml) and
user-submitted themes
[here](https://github.com/helix-editor/helix/blob/master/runtime/themes).


## The details of theme creation

### Color palettes

It's recommended to define a palette of named colors, and refer to them in the
configuration values in your theme. To do this, add a table called
`palette` to your theme file:

```toml
"ui.background" = "white"
"ui.text" = "black"

[palette]
white = "#ffffff"
black = "#000000"
```

Keep in mind that the `[palette]` table includes all keys after its header,
so it should be defined after the normal theme options.

The default palette uses the terminal's default 16 colors, and the colors names
are listed below. The `[palette]` section in the config file takes precedence
over it and is merged into the default palette.

| Color Name      |
| ---             |
| `default`       |
| `black`         |
| `red`           |
| `green`         |
| `yellow`        |
| `blue`          |
| `magenta`       |
| `cyan`          |
| `gray`          |
| `light-red`     |
| `light-green`   |
| `light-yellow`  |
| `light-blue`    |
| `light-magenta` |
| `light-cyan`    |
| `light-gray`    |
| `white`         |

### Modifiers

The following values may be used as modifier, provided they are supported by
your terminal emulator.

| Modifier             |
| ---                  |
| `bold`               |
| `dim`                |
| `italic`             |
| `underlined`         |
| `slow_blink`         |
| `rapid_blink`        |
| `reversed`           |
| `hidden`             |
| `crossed_out`        |

> 💡 The `underlined` modifier is deprecated and only available for backwards compatibility.
> Its behavior is equivalent to setting `underline.style="line"`.

### Underline style

One of the following values may be used as a value for `underline.style`, providing it is
supported by your terminal emulator.

| Modifier       |
| ---            |
| `line`         |
| `curl`         |
| `dashed`       |
| `dotted`       |
| `double_line`  |


### Inheritance

Extend other themes by setting the `inherits` property to an existing theme.

```toml
inherits = "boo_berry"

# Override the theming for "keyword"s:
"keyword" = { fg = "gold" }

# Override colors in the palette:
[palette]
berry = "#2A2A4D"
```

### Scopes

The following is a list of scopes available to use for styling:

#### Syntax highlighting

These keys match [tree-sitter scopes](https://tree-sitter.github.io/tree-sitter/3-syntax-highlighting.html#highlights).

When determining styling for a highlight, the longest matching theme key will be used. For example, if the highlight is `function.builtin.static`, the key `function.builtin` will be used instead of `function`.

We use a similar set of scopes as
[Sublime Text](https://www.sublimetext.com/docs/scope_naming.html). See also
[TextMate](https://macromates.com/manual/en/language_grammars) scopes.

- `attribute` - Class attributes, HTML tag attributes

- `type` - Types
  - `builtin` - Primitive types provided by the language (`int`, `usize`)
  - `parameter` - Generic type parameters (`T`)
  - `enum`
    - `variant`
- `constructor`

- `constant` (TODO: constant.other.placeholder for `%v`)
  - `builtin` Special constants provided by the language (`true`, `false`, `nil` etc)
    - `boolean`
  - `character`
    - `escape`
  - `numeric` (numbers)
    - `integer`
    - `float`

- `string` (TODO: string.quoted.{single, double}, string.raw/.unquoted)?
  - `regexp` - Regular expressions
  - `special`
    - `path`
    - `url`
    - `symbol` - Erlang/Elixir atoms, Ruby symbols, Clojure keywords

- `comment` - Code comments
  - `line` - Single line comments (`//`)
    - `documentation` - Line documentation comments (e.g. `///` in Rust)
  - `block` - Block comments (e.g. (`/* */`)
    - `documentation` - Block documentation comments (e.g. `/** */` in Rust)
  - `unused` - Unused variables and patterns, e.g. `_` and `_foo`

- `variable` - Variables
  - `builtin` - Reserved language variables (`self`, `this`, `super`, etc.)
  - `parameter` - Function parameters
  - `other`
    - `member` - Fields of composite data types (e.g. structs, unions)
      - `private` - Private fields that use a unique syntax (currently just ECMAScript-based languages)

- `label` - `.class`, `#id` in CSS, etc.

- `punctuation`
  - `delimiter` - Commas, colons
  - `bracket` - Parentheses, angle brackets, etc.
  - `special` - String interpolation brackets.

- `keyword`
  - `control`
    - `conditional` - `if`, `else`
    - `repeat` - `for`, `while`, `loop`
    - `import` - `import`, `export`
    - `return`
    - `exception`
  - `operator` - `or`, `in`
  - `directive` - Preprocessor directives (`#if` in C)
  - `function` - `fn`, `func`
  - `storage` - Keywords describing how things are stored
    - `type` - The type of something, `class`, `function`, `var`, `let`, etc.
    - `modifier` - Storage modifiers like `static`, `mut`, `const`, `ref`, etc.

- `operator` - `||`, `+=`, `>`

- `function`
  - `builtin`
  - `method`
    - `private` - Private methods that use a unique syntax (currently just ECMAScript-based languages)
  - `macro`
  - `special` (preprocessor in C)

- `tag` - Tags (e.g. `<body>` in HTML)
  - `builtin`

- `namespace`

- `special` - `derive` in Rust, etc.

- `markup`
  - `heading`
    - `marker`
    - `1`, `2`, `3`, `4`, `5`, `6` - heading text for h1 through h6
  - `list`
    - `unnumbered`
    - `numbered`
    - `checked`
    - `unchecked`
  - `bold`
  - `italic`
  - `strikethrough`
  - `link`
    - `url` - URLs pointed to by links
    - `label` - non-URL link references
    - `text` - URL and image descriptions in links
  - `quote`
  - `raw`
    - `inline`
    - `block`

- `diff` - version control changes
  - `plus` - additions
    - `gutter` - gutter indicator
  - `minus` - deletions
    - `gutter` - gutter indicator
  - `delta` - modifications
    - `moved` - renamed or moved files/changes
    - `conflict` - merge conflicts
    - `gutter` - gutter indicator

#### Interface

These scopes are used for theming the editor interface:

- `markup`
  - `normal`
    - `completion` - for completion doc popup UI
    - `hover` - for hover popup UI
  - `heading`
    - `completion` - for completion doc popup UI
    - `hover` - for hover popup UI
  - `raw`
    - `inline`
      - `completion` - for completion doc popup UI
      - `hover` - for hover popup UI


| Key                               | Notes                                                                                          |
| ---                               | ---                                                                                            |
| `ui.background`                   |                                                                                                |
| `ui.background.separator`         | Picker separator below input line                                                              |
| `ui.cursor`                       |                                                                                                |
| `ui.cursor.normal`                |                                                                                                |
| `ui.cursor.insert`                |                                                                                                |
| `ui.cursor.select`                |                                                                                                |
| `ui.cursor.match`                 | Matching bracket etc.                                                                          |
| `ui.cursor.primary`               | Cursor with primary selection                                                                  |
| `ui.cursor.primary.normal`        |                                                                                                |
| `ui.cursor.primary.insert`        |                                                                                                |
| `ui.cursor.primary.select`        |                                                                                                |
| `ui.debug.breakpoint`             | Breakpoint indicator, found in the gutter                                                      |
| `ui.debug.active`                 | Indicator for the line at which debugging execution is paused at, found in the gutter          |
| `ui.gutter`                       | Gutter                                                                                         |
| `ui.gutter.selected`              | Gutter for the line the cursor is on                                                           |
| `ui.linenr`                       | Line numbers                                                                                   |
| `ui.linenr.selected`              | Line number for the line the cursor is on                                                      |
| `ui.statusline`                   | Statusline (support element style e.g. `ui.statusline.file-name`)                              |
| `ui.statusline.inactive`          | Statusline (unfocused document)                                                                |
| `ui.statusline.normal`            | Statusline mode during normal mode ([only if `editor.color-modes` is enabled][editor-section]) |
| `ui.statusline.insert`            | Statusline mode during insert mode ([only if `editor.color-modes` is enabled][editor-section]) |
| `ui.statusline.select`            | Statusline mode during select mode ([only if `editor.color-modes` is enabled][editor-section]) |
| `ui.statusline.separator`         | Separator character in statusline                                                              |
| `ui.bufferline`                   | Style for the buffer line                                                                      |
| `ui.bufferline.active`            | Style for the active buffer in buffer line                                                     |
| `ui.bufferline.background`        | Style for bufferline background                                                                |
| `ui.popup`                        | Documentation popups (e.g. Space + k)                                                          |
| `ui.popup.info`                   | Prompt for multiple key options                                                                |
| `ui.picker.header`                | Header row area in pickers with multiple columns                                               |
| `ui.picker.header.column`         | Column names in pickers with multiple columns                                                  |
| `ui.picker.header.column.active`  | The column name in pickers with multiple columns where the cursor is entering into.            |
| `ui.window`                       | Borderlines separating splits                                                                  |
| `ui.help`                         | Description box for commands                                                                   |
| `ui.text`                         | Default text style, command prompts, popup text, etc.                                          |
| `ui.text.focus`                   | The currently selected line in the picker                                                      |
| `ui.text.inactive`                | Same as `ui.text` but when the text is inactive (e.g. suggestions)                             |
| `ui.text.info`                    | The key: command text in `ui.popup.info` boxes                                                 |
| `ui.text.directory`               | Directory names in prompt completion                                                           |
| `ui.virtual.ruler`                | Ruler columns (see the [`editor.rulers` config][editor-section])                               |
| `ui.virtual.whitespace`           | Visible whitespace characters                                                                  |
| `ui.virtual.indent-guide`         | Vertical indent width guides                                                                   |
| `ui.virtual.inlay-hint`           | Default style for inlay hints of all kinds                                                     |
| `ui.virtual.inlay-hint.parameter` | Style for inlay hints of kind `parameter` (language servers are not required to set a kind)    |
| `ui.virtual.inlay-hint.type`      | Style for inlay hints of kind `type` (language servers are not required to set a kind)         |
| `ui.virtual.wrap`                 | Soft-wrap indicator (see the [`editor.soft-wrap` config][editor-section])                      |
| `ui.virtual.jump-label`           | Style for virtual jump labels                                                                  |
| `ui.menu`                         | Code and command completion menus                                                              |
| `ui.menu.selected`                | Selected autocomplete item                                                                     |
| `ui.menu.scroll`                  | `fg` sets thumb color, `bg` sets track color of scrollbar                                      |
| `ui.selection`                    | For selections in the editing area                                                             |
| `ui.selection.primary`            |                                                                                                |
| `ui.highlight`                    | Highlighted lines in the picker preview                                                        |
| `ui.highlight.frameline`          | Line at which debugging execution is paused at                                                 |
| `ui.cursorline.primary`           | The line of the primary cursor ([if cursorline is enabled][editor-section])                    |
| `ui.cursorline.secondary`         | The lines of any other cursors ([if cursorline is enabled][editor-section])                    |
| `ui.cursorcolumn.primary`         | The column of the primary cursor ([if cursorcolumn is enabled][editor-section])                |
| `ui.cursorcolumn.secondary`       | The columns of any other cursors ([if cursorcolumn is enabled][editor-section])                |
| `warning`                         | Diagnostics warning (gutter)                                                                   |
| `error`                           | Diagnostics error (gutter)                                                                     |
| `info`                            | Diagnostics info (gutter)                                                                      |
| `hint`                            | Diagnostics hint (gutter)                                                                      |
| `diagnostic`                      | Diagnostics fallback style (editing area)                                                      |
| `diagnostic.hint`                 | Diagnostics hint (editing area)                                                                |
| `diagnostic.info`                 | Diagnostics info (editing area)                                                                |
| `diagnostic.warning`              | Diagnostics warning (editing area)                                                             |
| `diagnostic.error`                | Diagnostics error (editing area)                                                               |
| `diagnostic.unnecessary`          | Diagnostics with unnecessary tag (editing area)                                                |
| `diagnostic.deprecated`           | Diagnostics with deprecated tag (editing area)                                                 |
| `tabstop`                         | Snippet placeholder                                                                            |

[editor-section]: ./configuration.md#editor-section
