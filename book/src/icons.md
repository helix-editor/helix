# Icons

## Requirements

File-type and symbol-kind icons require a patched font such as [NerdFonts](https://www.nerdfonts.com/) to be installed and configured in your terminal emulator. These types of fonts are called *patched* fonts because they define arbitrary symbols for a range of Unicode values, which may vary from one font to another. Therefore, you need to use an icon flavor adapted to your configured terminal font, otherwise you may end up with undefined characters and mismatched icons.

To enable file-type and symbol-kind icons within the editor, see the `[editor.icons]` section of the [configuration file](./configuration.md).

To use an icon flavor add `icons = "<name>"` to your [`config.toml`](./configuration.md) at the very top of the file before the first section or select it during runtime using `:icons <name>`.

## Creating an icon flavor

Create a file with the name of your icon flavor as file name (i.e `myicons.toml`) and place it in your `icons` directory (i.e `~/.config/helix/icons`). The directory might have to be created beforehand.

The name "default" is reserved for the builtin icons and cannot be overridden by user defined icons.

The name of the icon flavor must be set using the `name` key.

The default icons.toml can be found [here](https://github.com/helix-editor/helix/blob/master/icons.toml), and user submitted icon flavors [here](https://github.com/helix-editor/helix/blob/master/runtime/icons). 

Icons flavors have five sections:

- Diagnostics
- Breakpoints
- Diff
- Symbol kinds
- Mime types

Each line in these sections is specified as below:

```toml
key = { icon = "…", color = "#ff0000" }
```

where `key` represents what you want to style, `icon` specifies the character to show as the icon, and `color` specifies the foreground color of the icon. `color` can be omitted to defer to the defaults.

### Diagnostic icons

The `[diagnostic]` section defines four **required** diagnostic icons:

- `error`
- `warning`
- `info`
- `hint`

These icons appear in the gutter, in the diagnostic pickers as well as in the status line diagnostic component.
By default, they have the foreground color defined in the current theme's corresponding keys.

> An icon flavor TOML file must define all of these icons.

### Diff icons

The `[diff]` section defines three **required** diffing icons:

- `added`
- `deleted`
- `modified`

These icons appear in the gutter.
By default, they have the foreground color defined in the current theme's corresponding keys.

> An icon flavor TOML file must define all of these icons.

### Breakpoint icons

The `[breakpoint]` section defines two **required** breakpoint icons:

- `verified`
- `unverified`

These icons appear in the gutter while using the Debug Adapter Protocol (DAP). Their color depends on the breakpoint's condition and log message, it cannot be overridden by the `color` key.

> An icon flavor TOML file must define all of these icons.

### Symbol kinds icons

The `[symbol-kind]` section defines **optional** icons for the following required LSP-defined symbol kinds:

- `file` (this icon is also used on files for which the mime type has not been defined in the next section, as a "generic file" icon)
- `module`
- `namespace`
- `package`
- `class`
- `method`
- `property`
- `field`
- `constructor`
- `enumeration`
- `interface`
- `variable`
- `function`
- `constant`
- `string`
- `number`
- `boolean`
- `array`
- `object`
- `key`
- `null`
- `enum-member`
- `structure`
- `event`
- `operator`
- `type-parameter`

By default, these icons have the same style as the loaded theme's `keyword` key. Their style can be customized using the `symbolkind` key in the theme configuration file, or it can individually be overridden by their `color` key.

> An icon flavor TOML file must define either none or all of these icons.

### Mime types icons

The `[mime-type]` section defines **optional** icons for mime types or filename, such as:

```toml
[mime-type]
".bashrc" = { icon = "…", color = "#…" }
"LICENSE" = { icon = "…", color = "#…" }
"rs" = { icon = "…", color = "#…" }
```

These icons appear in the file picker, in the statusline `file-type-icon` component, and in the bufferline (when enabled).

> An icon flavor TOML file can define none, some or all of these icons.

### Inheritance

Extend upon other icon flavors by setting the `inherits` property to an existing theme.

```toml
inherits = "nerdfonts"
name = "custom_nerdfonts"

# Override the icon for generic files:
[symbol-kind]
file = {icon = "…"}

# Override the icon for Rust files
[mime-type]
"rs" = { icon = "…", color = "#…" }
```
