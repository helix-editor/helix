# Workspace trust

Helix has two potentially dangerous features: language servers (LSP) and local workspace configurations, both of which can execute arbitrary code. To protect against this, Helix includes workspace trust protection, which prevents these features from running automatically unless the workspace is explicitly trusted.

## Default trust behavior

Helix does not trust any workspace by default and will prompt you to choose the trust level when you open a file in a workspace where trust has not yet been set.

If you decide not to trust a workspace and want to avoid repeated trust prompts when starting new sessions, you can exclude it by selecting `Never` in the trust selection window.

## Changing workspace trust status

You can always make the current workspace trusted by running the `:workspace-trust` command, and untrust it using `:workspace-untrust`.

## Where trust settings are stored

Lists of trusted and excluded workspaces, delimited by newline characters, are stored in `~/.local/share/helix/trusted_workspaces` and `~/.local/share/helix/excluded_workspaces` correspondingly.
<!-- TODO: Windows paths -->

## Configuration

You can return to the old behaviour of loading every local `.helix/config.toml` and `.helix/languages.toml` and starting language servers without an explicit permission by setting the following option:

```toml
[editor]
insecure = true
```
