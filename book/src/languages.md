# Languages

Language-specific settings and settings for particular language servers can be configured in a `languages.toml` file placed in your [configuration directory](./configuration.md). An example which includes the defaults used by helix can be found in the main helix repository's [languages.toml](https://github.com/helix-editor/helix/blob/master/languages.toml) file.

If, for example, a user wants to disable format on save for rust files, they can change the default rust entry's `auto-format`, such that the rust entry reads as follows:

```
# in <config_dir>/helix/languages.toml

[[language]]
name = "rust"
scope = "source.rust"
injection-regex = "rust"
file-types = ["rs"]
roots = []
auto-format = false
comment-token = "//"
language-server = { command = "rust-analyzer" }
indent = { tab-width = 4, unit = "    " }
[language.config]
cargo = { loadOutDirsFromCheck = true }
procMacro = { enable = false }
```

