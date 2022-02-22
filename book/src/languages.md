# Languages

Language-specific settings and settings for particular language servers can be configured in a `languages.toml` file placed in your [configuration directory](./configuration.md). Helix actually uses two `languages.toml` files, the [first one](https://github.com/helix-editor/helix/blob/master/languages.toml) is in the main helix repository; it contains the default settings for each language and is included in the helix binary at compile time. Users who want to see the available settings and options can either reference the helix repo's `languages.toml` file, or consult the table in the [adding languages](./guides/adding_languages.md) section.

Changes made to the `languages.toml` file in a user's [configuration directory](./configuration.md) are merged with helix's defaults on start-up, such that a user's settings will take precedence over defaults in the event of a collision. For example, the default `languages.toml` sets rust's `auto-format` to `true`. If a user wants to disable auto-format, they can change the `languages.toml` in their [configuration directory](./configuration.md) to make the rust entry read like the example below; the new key/value pair `auto-format = false` will override the default when the two sets of settings are merged on start-up:

```
# in <config_dir>/helix/languages.toml

[[language]]
name = "rust"
auto-format = false
```
