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

Possible modifiers:

| Modifier       |
| ---            |
| `bold`         |
| `dim`          |
| `italic`       |
| `underlined`   |
| `slow\_blink`  |
| `rapid\_blink` |
| `reversed`     |
| `hidden`       |
| `crossed\_out` |

Possible keys:

| Key                      | Notes                               |
| ---                      | ---                                 |
| `attribute`              |                                     |
| `keyword`                |                                     |
| `keyword.directive`      | Preprocessor directives (\#if in C) |
| `keyword.control`        | Control flow                        |
| `namespace`              |                                     |
| `punctuation`            |                                     |
| `punctuation.delimiter`  |                                     |
| `operator`               |                                     |
| `special`                |                                     |
| `property`               |                                     |
| `variable`               |                                     |
| `variable.parameter`     |                                     |
| `type`                   |                                     |
| `type.builtin`           |                                     |
| `type.enum.variant`      | Enum variants                       |
| `constructor`            |                                     |
| `function`               |                                     |
| `function.macro`         |                                     |
| `function.builtin`       |                                     |
| `comment`                |                                     |
| `variable.builtin`       |                                     |
| `constant`               |                                     |
| `constant.builtin`       |                                     |
| `string`                 |                                     |
| `number`                 |                                     |
| `escape`                 | Escaped characters                  |
| `label`                  | For lifetimes                       |
| `module`                 |                                     |
| `ui.background`          |                                     |
| `ui.cursor`              |                                     |
| `ui.cursor.insert`       |                                     |
| `ui.cursor.select`       |                                     |
| `ui.cursor.match`        | Matching bracket etc.               |
| `ui.cursor.primary`      | Cursor with primary selection       |
| `ui.linenr`              |                                     |
| `ui.linenr.selected`     |                                     |
| `ui.statusline`          |                                     |
| `ui.statusline.inactive` |                                     |
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
| `warning`                | LSP warning                         |
| `error`                  | LSP error                           |
| `info`                   | LSP info                            |
| `hint`                   | LSP hint                            |

These keys match [tree-sitter scopes](https://tree-sitter.github.io/tree-sitter/syntax-highlighting#theme). We half-follow the common scopes from [macromates language grammars](https://macromates.com/manual/en/language_grammars) with some differences.

For a given highlight produced, styling will be determined based on the longest matching theme key. So it's enough to provide function to highlight `function.macro` and `function.builtin` as well, but you can use more specific scopes to highlight specific cases differently.

## Color palettes

You can define a palette of named colors, and refer to them from the
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

If there is no `[palette]` section, a default palette which uses the terminal's default 16 colors are used:

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
