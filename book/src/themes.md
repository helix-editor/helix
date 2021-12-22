# Themes

First you'll need to place selected themes in your `themes` directory (i.e `~/.config/helix/themes`), the directory might have to be created beforehand.

To use a custom theme add `theme = <name>` to your [`config.toml`](./configuration.md) or override it during runtime using `:theme <name>`.

The default theme.toml can be found [here](https://github.com/helix-editor/helix/blob/master/theme.toml), and user submitted themes [here](https://github.com/helix-editor/helix/blob/master/runtime/themes). 

## Creating a theme

First create a file with the name of your theme as file name (i.e `mytheme.toml`) and place it in your `themes` directory (i.e `~/.config/helix/themes`).

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
ui.background = "white"
ui.text = "black"

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
  - `operator` - `or`, `in`
  - `directive` - Preprocessor directives (`#if` in C) 
  - `function` - `fn`, `func`

- `operator` - `||`, `+=`, `>`

- `function`
  - `builtin`
  - `method`
  - `macro`
  - `special` (preprocesor in C)

- `tag` - Tags (e.g. `<body>` in HTML)

- `namespace`

- `markup`
  - `heading`
  - `list`
    - `unnumbered`
    - `numbered`
  - `bold`
  - `italic`
  - `link`
    - `url`
    - `label`
  - `quote`
  - `raw`
    - `inline`
    - `block`

#### Interface

These scopes are used for theming the editor interface.


| Key                      | Notes                               |
| ---                      | ---                                 |
| `ui.background`          |                                     |
| `ui.cursor`              |                                     |
| `ui.cursor.insert`       |                                     |
| `ui.cursor.select`       |                                     |
| `ui.cursor.match`        | Matching bracket etc.               |
| `ui.cursor.primary`      | Cursor with primary selection       |
| `ui.linenr`              |                                     |
| `ui.linenr.selected`     |                                     |
| `ui.statusline`          | Statusline                          |
| `ui.statusline.inactive` | Statusline (unfocused document)     |
| `ui.popup`               |                                     |
| `ui.window`              |                                     |
| `ui.help`                |                                     |
| `ui.text`                |                                     |
| `ui.text.focus`          |                                     |
| `ui.info`                |                                     |
| `ui.info.text`           |                                     |
| `ui.menu`                |                                     |
| `ui.menu.selected`       |                                     |
| `ui.selection`           | For selections in the editing area  |
| `ui.selection.primary`   |                                     |
| `warning`                | Diagnostics warning (gutter)        |
| `error`                  | Diagnostics error (gutter)          |
| `info`                   | Diagnostics info (gutter)           |
| `hint`                   | Diagnostics hint (gutter)           |
| `diagnostic`             | For text in editing area            |

