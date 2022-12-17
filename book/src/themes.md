# Themes

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

### Using the linter

Use the supplied linting tool to check for errors and missing scopes:

```sh
cargo xtask themelint onedark # replace onedark with <name>
```

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

These keys match [tree-sitter scopes](https://tree-sitter.github.io/tree-sitter/syntax-highlighting#theme).

When determining styling for a highlight, the longest matching theme key will be used. For example, if the highlight is `function.builtin.static`, the key `function.builtin` will be used instead of `function`.

We use a similar set of scopes as
[Sublime Text](https://www.sublimetext.com/docs/scope_naming.html). See also
[TextMate](https://macromates.com/manual/en/language_grammars) scopes.

**Note**: not all language keys are applicable to all languages, you can have a look at the `runtime/queries/*/highlights.scm` files to see what is defined or not for your language of choice.

| `ui.virtual.inlay-hint`           | Default style for inlay hints of all kinds                                                     |
| `ui.virtual.inlay-hint.parameter` | Style for inlay hints of kind `parameter` (LSPs are not required to set a kind)                |
| `ui.virtual.inlay-hint.type`      | Style for inlay hints of kind `type` (LSPs are not required to set a kind)                     |

{{#include generated/theme-table.md}}
