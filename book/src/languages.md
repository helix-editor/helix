# Languages

Language-specific settings and settings for particular language servers can be configured in a `languages.toml` file placed in your [configuration directory](./configuration.md). Helix actually uses two `languages.toml` files, the [first one](https://github.com/helix-editor/helix/blob/master/languages.toml) is in the main helix repository; it contains the default settings for each language and is included in the helix binary at compile time. Users who want to see the available settings and options can either reference the helix repo's `languages.toml` file, or consult the table in the [adding languages](./guides/adding_languages.md) section.

Changes made to the `languages.toml` file in a user's [configuration directory](./configuration.md) are merged with helix's defaults on start-up, such that a user's settings will take precedence over defaults in the event of a collision. For example, the default `languages.toml` sets rust's `auto-format` to `true`. If a user wants to disable auto-format, they can change the `languages.toml` in their [configuration directory](./configuration.md) to make the rust entry read like the example below; the new key/value pair `auto-format = false` will override the default when the two sets of settings are merged on start-up:

```toml
# in <config_dir>/helix/languages.toml

[[language]]
name = "rust"
auto-format = false
```

## Tree-sitter grammars

Tree-sitter grammars can also be configured in `languages.toml`:

```toml
# in <config_dir>/helix/languages.toml

[[grammar]]
name = "rust"
source = { git = "https://github.com/tree-sitter/tree-sitter-rust", rev = "a250c4582510ff34767ec3b7dcdd3c24e8c8aa68" }

[[grammar]]
name = "c"
source = { path = "/path/to/tree-sitter-c" }
```

If a user has a `languages.toml`, only the grammars in that `languages.toml` are evaluated when running `hx --fetch-grammars` and `hx --build-grammars`. Otherwise the [default `languages.toml`](https://github.com/helix-editor/helix/blob/master/languages.toml) is used.
