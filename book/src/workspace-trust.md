# Workspace trust

Helix includes two potentially dangerous features, both of which can execute arbitrary code:

- Language servers (LSP)
- Local workspace configurations (`.helix/config.toml` and `.helix/languages.toml`)

To protect against this, Helix includes workspace trust protection, which prevents these features from running automatically unless the workspace is explicitly trusted.

## Default trust behavior

Helix does not trust any workspace by default and will prompt you to choose the trust level when you open a file in a workspace where trust has not yet been set.

## Changing workspace trust status

You can always make the current workspace trusted by running the `:workspace-trust` command, and untrust it using `:workspace-untrust`.

Lists of trusted and excluded workspaces, delimited by newline characters, are stored in:

- Linux and macOS: `~/.local/share/helix/trusted_workspaces` and `~/.local/share/helix/excluded_workspaces`
- Windows: `%AppData%/Roaming/helix/trusted_workspaces` and `%AppData%/Roaming/helix/excluded_workspaces` 

## Configuration

You can disable workspace trust completely with:

```toml
[editor]
insecure = true
```
