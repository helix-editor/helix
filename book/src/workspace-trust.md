# Workspace trust

Helix has a concept of workspace trust. Workspace that is not deemed to be trusted cannot: 

  - load `.helix/languages.toml` and `.helix/config.toml` files found in the workspace;
  - start a language server.

Helix will not trust any workspace by default.

When you open a file in an untrusted workspace, you will be prompted about trust. You can make a choice with arrow keys, `<Tab>`, `<C-n>` and `<C-p>`, confirming selection with `<Enter>`. Typing anything else will close the menu selecting 'Not now' option.

You can always make active workspace trusted by running `:workspace-trust` command, or you can remove trust with `:workspace-untrust`.

Lists of trusted and excluded workspaces, delimited by newline characters, are stored in:
  - Linux and macOS: `~/.local/share/helix/trusted_workspaces` and `~/.local/share/helix/excluded_workspaces`
  - Windows: `%AppData%/Roaming/helix/trusted_workspaces` and `%AppData%/Roaming/helix/excluded_workspaces` 

# Configuration

You can return to the old behavior of implicitly trusting every workspace by setting configuration option:

```toml
[editor]
insecure = true
```
