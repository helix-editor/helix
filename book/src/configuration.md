# Configuration

## Theme

Use a custom theme by placing a theme.toml in your config directory (i.e ~/.config/helix/theme.toml). A sample theme.toml can be found [here](https://github.com/helix-editor/helix/blob/master/theme.toml)

Styles in theme.toml are specified of in the form:

```toml
"name" = { fg = "#ffffff", bg = "#000000", modifiers = ["bold", "italic"] }
```
 where name represents what you want to style, fg specifies the foreground colour, bg the background colour, and modifiers is a list of style modifiers. bg or bold can be omitted to defer to the defaults.

 If you only want to specify the foreground color, you can do so as:

 ```toml
 "name" = "#ffffff"
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

 Possible entries for name are:

| name | notes |
----------------
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
