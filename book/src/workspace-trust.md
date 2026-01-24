# Workspace trust

Helix has a number of potentially dangerous features, namely LSP and ability to use local to workspace configurations. Those features can lead to unexpected code execution. To protect against code execution in dangerous contexts, Helix has a workspace trust protection, which will prevent these potentially dangerous features from running automatically.

Helix will not trust any workspace by default.

By default, it will prompt about trust when you open new file in a workspace where you didn't make a decision about trust yet.

If you decide not to trust a workspace and don't want to be prompted about trust every time you start a new session in it, you can exclude the workspace by choosing `Never` option in trust selection window.

You can always make current workspace trusted by running `:workspace-trust` command, and untrust it with `:workspace-untrust`.

Lists of trusted and exluded workspaces, delimited by newline characters, are stored in `~/.local/share/helix/trusted_workspaces` and `~/.local/share/helix/excluded_workspaces` correspondingly.
<!-- TODO: macOS/Windows paths -->

# Configuration

You can return to the old behaviour of loading every local `.helix/config.toml` and `.helix/languages.toml` and starting LSP's without an explicit permission by setting following option:

```toml
[editor]
insecure = true
```
