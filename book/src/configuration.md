# Configuration

## LSP

To disable language server progress report from being displayed in the status bar add this option to your `config.toml`:
```toml
lsp-progress = false
```

## Theme

Use a custom theme by placing a theme.toml in your config directory (i.e ~/.config/helix/theme.toml). The default theme.toml can be found [here](https://github.com/helix-editor/helix/blob/master/theme.toml), and user submitted themes [here](https://github.com/helix-editor/helix/blob/master/contrib/themes).

Styles in theme.toml are specified of in the form:

```toml
key = { fg = "#ffffff", bg = "#000000", modifiers = ["bold", "italic"] }
```

where `name` represents what you want to style, `fg` specifies the foreground color, `bg` the background color, and `modifiers` is a list of style modifiers. `bg` and `modifiers` can be omitted to defer to the defaults.

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
| `slow_blink`  |
| `rapid_blink` |
| `reversed`     |
| `hidden`       |
| `crossed_out` |

Possible keys:

| Key                     | Notes                               |
| ---                     | ---                                 |
| `attribute`             |                                     |
| `keyword`               |                                     |
| `keyword.directive`     | Preprocessor directives (\#if in C) |
| `namespace`             |                                     |
| `punctuation`           |                                     |
| `punctuation.delimiter` |                                     |
| `operator`              |                                     |
| `special`               |                                     |
| `property`              |                                     |
| `variable`              |                                     |
| `variable.parameter`    |                                     |
| `type`                  |                                     |
| `type.builtin`          |                                     |
| `constructor`           |                                     |
| `function`              |                                     |
| `function.macro`        |                                     |
| `function.builtin`      |                                     |
| `comment`               |                                     |
| `variable.builtin`      |                                     |
| `constant`              |                                     |
| `constant.builtin`      |                                     |
| `string`                |                                     |
| `number`                |                                     |
| `escape`                | Escaped characters                  |
| `label`                 | For lifetimes                       |
| `module`                |                                     |
| `ui.background`         |                                     |
| `ui.linenr`             |                                     |
| `ui.linenr.selected`    | For lines with cursors              |
| `ui.statusline`         |                                     |
| `ui.popup`              |                                     |
| `ui.window`             |                                     |
| `ui.help`               |                                     |
| `ui.text`               |                                     |
| `ui.text.focus`         |                                     |
| `ui.menu.selected`      |                                     |
| `ui.selection`          | For selections in the editing area  |
| `warning`               | LSP warning                         |
| `error`                 | LSP error                           |
| `info`                  | LSP info                            |
| `hint`                  | LSP hint                            |

These keys match [tree-sitter scopes](https://tree-sitter.github.io/tree-sitter/syntax-highlighting#theme). We half-follow the common scopes from [macromates language grammars](https://macromates.com/manual/en/language_grammars) with some differences.

For a given highlight produced, styling will be determined based on the longest matching theme key. So it's enough to provide function to highlight `function.macro` and `function.builtin` as well, but you can use more specific scopes to highlight specific cases differently.

