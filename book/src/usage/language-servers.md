# Language Servers

## Overview and Default Settings

Helix implements the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
(LSP) to support language-specific features such as:

- Intellisense,
- Goto definitions,
- View documentation on hover,
- Workspace symbol search,
- Diagnostics.

Default configurations are available for many languages and language servers.
A reference of which languages (and which LSP features) are supported
is available in the [Language Support](./../reference/lang-support.md) section.
A reference for the default language server configurations, and instructions
on how to install the language servers that have a default configuration,
are available in the [Language Server Configuration](./../reference/language-server-configs.md)
section.

## Checking LSP Status

`hx --health <langage>` can be used to check the status of a particular language.
This will show what features are configured for that language, and will
display whether the configured language servers have been successfully found
in the shell's `$PATH`. Certain language features (like syntax highlighting)
are available without needing to install a language server for that language.

For example, to check the status of `go`, we would type: `hx --health go`.
With no language servers installed, this would give the following output:

```
Configured language servers:
 ✘ gopls: 'gopls' not found in $PATH
 ✘ golangci-lint-lsp: 'golangci-lint-langserver' not found in $PATH
Configured debug adapter: dlv
Binary for debug adapter: 'dlv' not found in $PATH
Configured formatter: None
Highlight queries: ✓
Textobject queries: ✓
Indent queries: ✓
```

This shows us that the language server `gopls` and `golangci-lint-lsp` both
have valid configurations, but the actual language server binary was not
found in the path. After installing `gopls`, rerunning `hx health go` will
show:

```
Configured language servers:
 ✓ gopls: /path/to/gopls
 ✘ jedi-language-server: 'golangci-lint-langserver' not found in $PATH
Configured debug adapter: dlv
Binary for debug adapter: 'dlv' not found in $PATH
Configured formatter: None
Highlight queries: ✓
Textobject queries: ✓
Indent queries: ✓
```

## LSP Formatting

If a formatter has been configured for a language, it can be used to
format an open buffer which is using that language.

To manually invoke the formatter for a language, you can type `:format`
or `:fmt`. Helix can be configured to auto-format on save for any
language with a formatter available. This can also be configured
on a per-language level (see [Configuration/Editor](./../configuration/editor.md)
and [Configuration/Languages](./../configuration/languages.md)).

## Useful Keymaps

Helix inludes default keymaps for a lot of language server functionality.
Some commonly used `NOR` mode keymaps are:

- `Space` + `r`: renames the symbol under the cursor across the project.
- `Space` + `k`: Show documentation for the symbol under the cursor
  (`Ctrl`-`c` will close it).
- `Space` + `a`: Show suggested fixes for the diagnostic under the cursor,
  if available.

Some commonly used `INS` mode keymaps are:

- `Ctrl`-`x` will open auto-completion suggestions for the string
  before the current insert-mode cursor.
  - `Ctrl`-`n` (or `Tab`) will move forward one suggestion.
  - `Ctrl`-`p` (or `Shift`-`Tab`) will move forward one suggestion.
  - `Enter` will select the currently-highlighted completion.
  - `Ctrl`-`c` will cancel the auto-completion and close the popup.
  