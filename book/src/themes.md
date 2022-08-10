# Themes

To use a theme add `theme = "<name>"` to your [`config.toml`](./configuration.md) at the very top of the file before the first section or select it during runtime using `:theme <name>`.

## Creating a theme

Create a file with the name of your theme as file name (i.e `mytheme.toml`) and place it in your `themes` directory (i.e `~/.config/helix/themes`). The directory might have to be created beforehand.

The names "default" and "base16_default" are reserved for the builtin themes and cannot be overridden by user defined themes.

The default theme.toml can be found [here](https://github.com/helix-editor/helix/blob/master/theme.toml), and user submitted themes [here](https://github.com/helix-editor/helix/blob/master/runtime/themes). 

Each line in the theme file is specified as below:

```toml
key = { fg = "#ffffff", bg = "#000000", modifiers = ["bold", "italic"] }
```

where `key` represents what you want to style, `fg` specifies the foreground color, `bg` the background color, and `modifiers` is a list of style modifiers. `bg` and `modifiers` can be omitted to defer to the defaults.

To specify only the foreground color:

```toml
key = "#ffffff"
```

if the key contains a dot `'.'`, it must be quoted to prevent it being parsed as a [dotted key](https://toml.io/en/v1.0.0#keys).

```toml
"key.key" = "#ffffff"
```

### Color palettes

It's recommended define a palette of named colors, and refer to them from the
configuration values in your theme. To do this, add a table called
`palette` to your theme file:

```toml
"ui.background" = "white"
"ui.text" = "black"

[palette]
white = "#ffffff"
black = "#000000"
```

Remember that the `[palette]` table includes all keys after its header,
so you should define the palette after normal theme options.

The default palette uses the terminal's default 16 colors, and the colors names
are listed below. The `[palette]` section in the config file takes precedence
over it and is merged into the default palette.

| Color Name      |
| ---             |
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

The following values may be used as modifiers. 

Less common modifiers might not be supported by your terminal emulator.

| Modifier       |
| ---            |
| `bold`         |
| `dim`          |
| `italic`       |
| `underlined`   |
| `slow_blink`   |
| `rapid_blink`  |
| `reversed`     |
| `hidden`       |
| `crossed_out`  |

### Scopes

The following is a list of scopes available to use for styling.

#### Syntax highlighting

These keys match [tree-sitter scopes](https://tree-sitter.github.io/tree-sitter/syntax-highlighting#theme).

For a given highlight produced, styling will be determined based on the longest matching theme key. For example, the highlight `function.builtin.static` would match the key `function.builtin` rather than `function`.

We use a similar set of scopes as
[SublimeText](https://www.sublimetext.com/docs/scope_naming.html). See also
[TextMate](https://macromates.com/manual/en/language_grammars) scopes.

- `type` - Types
  - `builtin` - Primitive types provided by the language (`int`, `usize`)
- `constructor`

- `constant` (TODO: constant.other.placeholder for %v)
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
  - `block` - Block comments (e.g. (`/*     */`)
    - `documentation` - Documentation comments (e.g. `///` in Rust)

- `variable` - Variables
  - `builtin` - Reserved language variables (`self`, `this`, `super`, etc)
  - `parameter` - Function parameters
  - `other`
    - `member` - Fields of composite data types (e.g. structs, unions)
  - `function` (TODO: ?)

- `label`

- `punctuation`
  - `delimiter` - Commas, colons
  - `bracket` - Parentheses, angle brackets, etc.

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
  - `macro`
  - `special` (preprocessor in C)

- `tag` - Tags (e.g. `<body>` in HTML)

- `namespace`

- `markup`
  - `heading`
    - `marker`
    - `1`, `2`, `3`, `4`, `5`, `6` - heading text for h1 through h6
  - `list`
    - `unnumbered`
    - `numbered`
  - `bold`
  - `italic`
  - `link`
    - `url` - urls pointed to by links
    - `label` - non-url link references
    - `text` - url and image descriptions in links
  - `quote`
  - `raw`
    - `inline`
    - `block`

- `diff` - version control changes
  - `plus` - additions
  - `minus` - deletions
  - `delta` - modifications
    - `moved` - renamed or moved files/changes

#### Interface

These scopes are used for theming the editor interface.

- `markup`
  - `normal`
    - `completion` - for completion doc popup ui
    - `hover` - for hover popup ui
  - `heading`
    - `completion` - for completion doc popup ui
    - `hover` - for hover popup ui
  - `raw`
    - `inline`
      - `completion` - for completion doc popup ui
      - `hover` - for hover popup ui


| Key                       | Notes                                          |
| ---                       | ---                                            |
| `ui.background`           |                                                |
| `ui.background.separator` | Picker separator below input line              |
| `ui.cursor`               |                                                |
| `ui.cursor.insert`        |                                                |
| `ui.cursor.select`        |                                                |
| `ui.cursor.match`         | Matching bracket etc.                          |
| `ui.cursor.primary`       | Cursor with primary selection                  |
| `ui.linenr`               | Line numbers                                   |
| `ui.linenr.selected`      | Line number for the line the cursor is on      |
| `ui.statusline`           | Statusline                                     |
| `ui.statusline.inactive`  | Statusline (unfocused document)                |
| `ui.statusline.normal`    | Statusline mode during normal mode ([only if `editor.color-modes` is enabled][editor-section]) |
| `ui.statusline.insert`    | Statusline mode during insert mode ([only if `editor.color-modes` is enabled][editor-section]) |
| `ui.statusline.select`    | Statusline mode during select mode ([only if `editor.color-modes` is enabled][editor-section]) |
| `ui.statusline.separator` | Separator character in statusline              |
| `ui.popup`                | Documentation popups (e.g space-k)             |
| `ui.popup.info`           | Prompt for multiple key options                |
| `ui.window`               | Border lines separating splits                 |
| `ui.help`                 | Description box for commands                   |
| `ui.text`                 | Command prompts, popup text, etc.              |
| `ui.text.focus`           |                                                |
| `ui.text.info`            | The key: command text in `ui.popup.info` boxes |
| `ui.virtual.ruler`        | Ruler columns (see the [`editor.rulers` config][editor-section])|
| `ui.virtual.whitespace`   | Visible white-space characters                 |
| `ui.virtual.indent-guide` | Vertical indent width guides                   |
| `ui.menu`                 | Code and command completion menus              |
| `ui.menu.selected`        | Selected autocomplete item                     |
| `ui.menu.scroll`          | `fg` sets thumb color, `bg` sets track color of scrollbar |
| `ui.selection`            | For selections in the editing area             |
| `ui.selection.primary`    |                                                |
| `ui.cursorline.primary`   | The line of the primary cursor                 |
| `ui.cursorline.secondary` | The lines of any other cursors                 |
| `warning`                 | Diagnostics warning (gutter)                   |
| `error`                   | Diagnostics error (gutter)                     |
| `info`                    | Diagnostics info (gutter)                      |
| `hint`                    | Diagnostics hint (gutter)                      |
| `diagnostic`              | Diagnostics fallback style (editing area)      |
| `diagnostic.hint`         | Diagnostics hint (editing area)                |
| `diagnostic.info`         | Diagnostics info (editing area)                |
| `diagnostic.warning`      | Diagnostics warning (editing area)             |
| `diagnostic.error`        | Diagnostics error (editing area)               |

[editor-section]: ./configuration.md#editor-section
