# Language Support

For more information like arguments passed to default LSP server,
extensions assosciated with a filetype, custom LSP settings, filetype
specific indent settings, etc see the default
[`languages.toml`][languages.toml] file.

{{#include ./generated/lang-support.md}}

[languages.toml]: https://github.com/helix-editor/helix/blob/master/languages.toml

# Local Configuration

A local `languages.toml` can be created within a `.helix` directory. Its settings will be merged with both the global and default configs.
