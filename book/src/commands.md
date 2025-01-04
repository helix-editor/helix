# Commands

- [Typable commands](#typable-commands)
  - [Using variables](#using-variables-in-typable-commands)
- [Static commands](#static-commands)

## Typable commands

Typable commands are used from command mode and may take arguments. Command mode can be activated by pressing `:`. The built-in typable commands are:

{{#include ./generated/typable-cmd.md}}


### Using variables in typable commands

Helix provides several variables that can be used when typing commands or creating custom shortcuts. These variables are listed below:

| Variable                | Description |
| ---                     | ---                      |
| `%{basename}` or `%{b}` | The name and extension of the currently focused file. |
| `%{dirname}`  or `%{d}` | The absolute path of the parent directory of the currently focused file. |
| `%{cwd}`                | The absolute path of the current working directory of Helix. |
| `%{repo}`               | The absolute path of the VCS repository helix is opened in. Fallback to `cwd` if not inside a VCS repository|
| `%{filename}` or `%{f}` | The absolute path of the currently focused file. |
| `%{filename:rel}`       | The relative path of the file according to `cwd` (will give absolute path if the file is not a child of the current working directory) |
| `%{filename:repo_rel}`  | The relative path of the file according to `repo` (will give absolute path if the file is not a child of the VCS directory or the cwd) |
| `%{ext}`                | The extension of the current file |
| `%{lang}`               | The language of the current file   |
| `%{linenumber}`         | The line number where the primary cursor is positioned. |
| `%{cursorcolumn}`       | The position of the primary cursor inside the current line. |
| `%{selection}`          | The text selected by the primary cursor. |
| `%sh{cmd}`              | Executes `cmd` with the default shell and returns the command output, if any. |

#### Example

```toml
[keys.normal]
# Print blame info for the line where the main cursor is.
C-b = ":echo %sh{git blame -L %{linenumber} %{filename}}"
```

## Static Commands

Static commands take no arguments and can be bound to keys. Static commands can also be executed from the command picker (`<space>?`). The built-in static commands are:

{{#include ./generated/static-cmd.md}}
