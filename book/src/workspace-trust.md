# Workspace trust

Helix includes two potentially dangerous features, both of which can execute arbitrary code:

- Language servers (LSP)
- Local workspace configurations (`.helix/config.toml` and `.helix/languages.toml`)

To protect against this, Helix includes workspace trust protection, which prevents these features from running automatically unless the workspace is explicitly trusted.

## Default trust behavior

Helix does not trust any workspace by default and will prompt you to choose the trust level when you open a file in a workspace where trust has not yet been set.

## Changing workspace trust status

You can always make the current workspace trusted by running the `:workspace-trust` command, and untrust it using `:workspace-untrust` or `:workspace-exclude`, with latter disabling all further prompts in it. If you wish to trust workspace only once, use `:workspace-trust-once`.

Lists of trusted and excluded workspaces, delimited by newline characters, are stored in:

- Linux and macOS: `~/.local/share/helix/trusted_workspaces` and `~/.local/share/helix/excluded_workspaces`
- Windows: `%AppData%\Roaming\helix\trusted_workspaces` and `%AppData%\Roaming\helix\excluded_workspaces`


## Configuration

```toml
[editor.workspace-trust]
# This option will disable workspace trust feature altogether.
level = "all"


# This will make Helix prompt only when there is local configuration
# present in the workspace.
# LSP will start automatically without an explicit confirmation.
level = "lsp"


# This is the default option.
level = "none"


# Disable pop-up selector, leaving status bar reminder instead.
selector = false


# Trust recursively every workspace in `~/work` and exclude every workspace
# that contains "contrib" in its name.
globs = [ "~/work/**", "!*contrib*" ]
```

### Glob syntax

For more info about syntax see [globset docs](https://docs.rs/globset/latest/globset/#syntax)
(`literal_separator` option is enabled).

Additional syntax:

  - all `~/` will be expanded to `$HOME`;
  - globs prefixed with `!` will be excluded instead of trusted.

For example, this is valid:

````toml
[editor]
# Exclude `$HOME/coding/helix/contrib` and all subdirectories.
workspace-trust.globs = [ "!{~/coding/helix/contrib/**,~/coding/helix/contrib}" ]

# Same thing, but with a smaller nesting.
workspace-trust.globs = [ "!~/coding/helix/{contrib/**,contrib}" ]

# Same, but without nesting.
workspace-trust.globs = [ "!~/coding/helix/contrib/**", "!~/coding/helix/contrib" ]
````
