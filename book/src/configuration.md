# Configuration

## Theme

Use a custom theme by placing a theme.toml in your config directory (i.e ~/.config/helix/theme.toml). The default theme.toml can be found [here](https://github.com/helix-editor/helix/blob/master/theme.toml), and user submitted themes [here](https://github.com/helix-editor/helix/blob/master/contrib/themes).

Styles in theme.toml are specified of in the form:

```toml
"key" = { fg = "#ffffff", bg = "#000000", modifiers = ["bold", "italic"] }
```
 where name represents what you want to style, fg specifies the foreground colour, bg the background colour, and modifiers is a list of style modifiers. bg or bold can be omitted to defer to the defaults.

 If you only want to specify the foreground color, you can do so as:

 ```toml
 "key" = "#ffffff"
 ```

 Possible entries for modifiers are:

| modifier |
------------
| bold |
| dim |
| italic |
| underlined |
| slow\_blink |
| rapid\_blink |
| reversed |
| hidden |
| crossed\_out |

 Possible entries for key are:

| key | notes |
---------------
| attribute | |
| keyword | |
| keyword.directive | preprocessor directives (\#if in C) |
| namespace | |
| punctuation | |
| punctuation.delimiter | |
| operator | |
| special | |
| property | |
| variable | |
| variable.parameter | |
| type | |
| type.builtin | |
| constructor | |
| function | |
| function.macro | |
| function.builtin | |
| comment | |
| variable.builtin | |
| constant | |
| constant.builtin | |
| string | |
| number | |
| escape | escaped characters |
| label | used for lifetimes |
| module | |
| ui.background | |
| ui.linenr | |
| ui.statusline | |
| ui.popup | |
| ui.window | |
| ui.help | |
| ui.text | |
| ui.text.focus | |
| ui.menu.selected | |
| warning | LSP warning |
| error | LSP error |
| info | LSP info |
| hint | LSP hint |

These keys match [tree-sitter scopes](https://tree-sitter.github.io/tree-sitter/syntax-highlighting#theme). We sorta half-follow the common scopes from [here](https://macromates.com/manual/en/language_grammars) with some differences.

For a given highlight produced, styling will be determined based on the longest matching theme key. So it's enough to provide function to highlight function.macro and function.builtin as well, but you can use more specific scopes to highlight specific cases differently.
