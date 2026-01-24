## Workspace Trust

Helix includes a workspace trust system to protect against malicious code execution when opening untrusted repositories. When you open a workspace for the first time, Helix will prompt you to decide whether to trust it.

### Why Workspace Trust?

Modern editors can execute arbitrary code through various mechanisms:

- **Language Servers (LSP)**: Defined in workspace configuration, could run malicious binaries
- **Debug Adapters (DAP)**: Execute programs for debugging
- **Shell Commands**: Commands like `:sh` or `:pipe` execute shell code
- **Workspace Configuration**: `.helix/config.toml` and `.helix/languages.toml` can override settings

An attacker could craft a repository that, when opened, executes malicious code through any of these vectors. The workspace trust system prevents this by gating these features until you explicitly trust the workspace.

### Trust Behavior

| Feature | Trusted | Untrusted |
|---------|---------|-----------|
| Language Servers (LSP) | Enabled | Disabled |
| Debug Adapters (DAP) | Enabled | Disabled |
| Shell Commands | Enabled | Disabled |
| Workspace Config (`.helix/`) | Loaded | Ignored |
| Tree-sitter Highlighting | Enabled | Enabled |
| Global Config (`~/.config/helix/`) | Loaded | Loaded |

### Trust Prompt

When opening a workspace for the first time (with default settings), Helix displays a trust prompt:

```
┌─────────────── Workspace Trust ───────────────┐
│ Do you trust the authors of this workspace?   │
│                                               │
│ /path/to/workspace                            │
│                                               │
│ Trusting enables:                             │
│  - Language servers (LSP)                     │
│  - Shell commands                             │
│  - Workspace configuration                    │
│                                               │
│ [y] Trust   [n] Don't Trust   [Esc] Cancel    │
└───────────────────────────────────────────────┘
```

- **`y`**: Trust the workspace permanently (persisted to disk)
- **`n`**: Don't trust the workspace permanently (persisted to disk)
- **`Esc`**: Cancel without persisting (treated as untrusted for this session only)

### CLI Flags

You can bypass the trust prompt using command-line flags:

```bash
# Trust the workspace (skip prompt, enable all features)
hx --trust /path/to/project

# Don't trust the workspace (skip prompt, disable dangerous features)
hx --untrust /path/to/project
```

### Commands

Two commands are available to change trust status while editing:

| Command | Description |
|---------|-------------|
| `:trust` | Trust the current workspace. Persists the decision and updates permissions. Restart required for workspace config to load. |
| `:untrust` | Untrust the current workspace. Persists the decision and immediately disables LSP, DAP, and shell commands. |

### Nested Workspaces

Trust decisions support inheritance from parent directories. If you trust a parent directory, all subdirectories inherit that trust unless explicitly overridden.

**Most specific path wins** - if both a parent and child have explicit trust decisions, the child's decision takes precedence.

#### Examples

| Trust Store | Opening | Result |
|-------------|---------|--------|
| `~/Workspace/` → trusted | `~/Workspace/project-a/` | Trusted (inherits from parent) |
| `~/Workspace/` → trusted | `~/Workspace/project-a/src/` | Trusted (inherits from parent) |
| `~/Workspace/` → trusted, `~/Workspace/sketchy/` → untrusted | `~/Workspace/sketchy/` | Untrusted (specific overrides parent) |
| `~/Workspace/` → trusted, `~/Workspace/sketchy/` → untrusted | `~/Workspace/sketchy/src/` | Untrusted (inherits from sketchy) |
| Nothing set | `~/other/` | Unknown (prompts user) |

This allows you to:
- Trust your entire `~/projects/` directory with a single decision
- Explicitly untrust specific repositories within trusted directories
- Keep downloaded or cloned repositories untrusted by default

### Configuration

Trust behavior can be configured in your global `~/.config/helix/config.toml`:

```toml
[editor.trust]
# Default behavior for unknown workspaces: "prompt", "trust", or "untrust"
default = "prompt"

# What's allowed in trusted workspaces
[editor.trust.trusted]
lsp = true
dap = true
shell-commands = true
workspace-config = true

# What's allowed in untrusted workspaces
[editor.trust.untrusted]
lsp = false
dap = false
shell-commands = false
workspace-config = false
```

#### Per-Workspace Overrides

You can configure specific workspaces with custom permissions in your config. These overrides also support path inheritance:

```toml
# Trust all projects in ~/work/ with full permissions
[[editor.trust.workspaces]]
path = "~/work"
lsp = true
dap = true
shell-commands = true
workspace-config = true

# But disable DAP for a specific legacy project
[[editor.trust.workspaces]]
path = "~/work/legacy-project"
lsp = true
dap = false
shell-commands = true
workspace-config = true

# Completely restrict downloaded repos
[[editor.trust.workspaces]]
path = "~/downloads"
lsp = false
dap = false
shell-commands = false
workspace-config = false
```

### Trust Resolution Order

When determining the **trust level** for a workspace, Helix checks in this order:

1. **CLI flag**: `--trust` or `--untrust` (highest priority)
2. **Persisted trust store**: Check `~/.config/helix/workspace-trust.toml` (most specific match wins)
3. **Default setting**: Use `editor.trust.default` from config

The per-workspace override from `[[editor.trust.workspaces]]` does **not** determine the trust level. It is used only after the trust level is known, to select which features (LSP, DAP, shell commands, workspace config, etc.) are enabled for that workspace.
### Persistence

Trust decisions are stored in `~/.config/helix/workspace-trust.toml`:

```toml
version = 0

# Trust all projects under ~/projects/
[workspaces."/home/user/projects"]
level = "trusted"
decided_at = 1705612800

# But untrust a specific downloaded repo
[workspaces."/home/user/projects/external-dependency"]
level = "untrusted"
decided_at = 1705612900
```

### Security Considerations

- **Symlinks**: Workspace paths are canonicalized to prevent symlink-based bypasses. A symlink to a trusted directory will be resolved and trusted.
- **Path normalization**: Paths like `~/project` and `/home/user/project` resolve to the same workspace
- **Nested inheritance**: Child directories inherit trust from parents, but explicit child decisions override parent decisions
- **Session-only untrust**: Pressing `Esc` at the prompt doesn't persist, so you'll be prompted again next time

### Troubleshooting

**LSP not starting?**
- Check if the workspace is trusted: run `:trust` to enable LSP
- Verify with `:sh echo test` - if blocked, the workspace is untrusted
- Check if a parent directory is untrusted in `~/.config/helix/workspace-trust.toml`

**Workspace config not loading?**
- Trust decisions made with `:trust` require a restart to load workspace config
- Use `hx --trust .` to start with workspace config enabled

**Want to trust all projects in a directory?**
- Trust the parent directory: open `~/projects/` and run `:trust`
- All subdirectories will inherit the trust

**Want to reset trust decisions?**
- Edit or delete `~/.config/helix/workspace-trust.toml`
- Or use `:untrust` followed by `:trust` to re-decide

**Symlinked project not trusted?**
- Symlinks are resolved to their real paths. Trust the actual directory, not the symlink.
