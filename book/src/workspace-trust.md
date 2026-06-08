# Workspace trust

Helix has two potentially dangerous features, both of which can execute
arbitrary code:

- Language servers (LSP)
- Local workspace configuration (`.helix/config.toml`, `.helix/languages.toml`)

To protect against malicious projects (a checked-out PR, a freshly cloned
repository, etc.) Helix gates them behind explicit per-workspace trust.
By default language servers and debug adapters still start automatically
(their binaries come from `$PATH`, not from the workspace), but loading
`.helix/config.toml` or `.helix/languages.toml` requires opting in. The
model is intentionally similar to [direnv](https://direnv.net/): you run
`:workspace-trust` once per workspace and Helix remembers across sessions.

## Granting trust

When Helix opens a file inside a workspace it has never seen before, a
modal trust prompt asks:

- **Trust** — allow the workspace permanently.
- **Never** — exclude the workspace; never prompt again.

`<Esc>` (or any other dismissal) caches "untrusted for this session" so
the prompt doesn't re-fire for every file you open in the workspace. The
next time you start Helix in that workspace, it'll prompt again.

A small `[⚠]` indicator appears in the bottom-right of the editor (next
to the macro-recording `[@]`) whenever the workspace is in restricted mode
*and* running `:workspace-trust` would change observable behavior — i.e.
when there's a local config to load or an LSP that would start.

You can also run `:workspace-trust` / `:workspace-untrust` /
`:workspace-exclude` directly from the typed command prompt.

## Revoking trust

Run `:workspace-untrust` to revoke a workspace's trust grant. The next time
you open a file in that workspace, you're back to the untrusted hint.

## Detecting changes after trust was granted

When you trust a workspace, Helix records a hash of every file under
`.helix/`. If those files change afterwards (a malicious checkout, an
inadvertent rebase, etc.) Helix detects the mismatch on the next open and
reports the workspace as *stale*:

```
Workspace `.helix/` config changed since `:workspace-trust`. Local config
not loaded. Run `:workspace-trust` to re-allow.
```

In the stale state, language servers continue to run (they use the
globally-configured binaries on `$PATH`, which are unchanged), but
`.helix/config.toml` and `.helix/languages.toml` are not loaded. Run
`:workspace-trust` again to re-pin the new hash.

## Storage

Trust grants live in `data_dir()/workspace_trust/`, one small file per
workspace. The filename is the SHA-256 of the workspace's absolute path;
the contents look like:

```
path = /home/user/proj1
hash = sha256:abc123...
excluded = false
```

- Linux, macOS: `~/.local/share/helix/workspace_trust/`
- Windows: `%AppData%\Roaming\helix\workspace_trust\`

The one-file-per-workspace shape is safe under multiple concurrent Helix
instances — different workspaces never write the same file.

## Configuration

Two settings live under `[editor.workspace-trust]`:

| Key      | Values                         | Default     | Effect                                                                       |
| ---      | ---                            | ---         | ---                                                                          |
| `level`  | `"none"`, `"servers"`, `"all"` | `"servers"` | What is auto-trusted in every workspace. See below.                          |
| `prompt` | `true`, `false`                | `true`      | Whether to surface the modal popup. The `[⚠]` indicator is shown regardless. |

### Recommended setups

**Default: trust servers, prompt before loading workspace config.**

```toml
[editor.workspace-trust]
level = "servers"
prompt = true
```

Language servers and debug adapters start automatically in every
workspace — their binaries come from `$PATH` and are not
workspace-controlled. The modal only appears when opening a file in a
workspace whose `.helix/config.toml` or `.helix/languages.toml` would
unlock something. Trust everything else with one keystroke per
workspace, deny with another.

**Maximum security: never prompt, trust each workspace by hand.**

```toml
[editor.workspace-trust]
level = "none"
prompt = false
```

Nothing trusts implicitly: language servers, debug adapters, local
config, and git `Trust::Full` are all off until you run
`:workspace-trust`. The popup never appears; the `[⚠]` indicator in the
bottom-right is your only signal that the current workspace is
restricted. Suited to users who would rather grant trust as a
deliberate action than dismiss a dialog.

> [!WARNING]
> `level = "all"` is highly discouraged. It implicitly trusts every
> workspace you open, which defeats the protection entirely: a
> checked-out PR with a malicious `.helix/config.toml` would get its
> configuration loaded and any language server it defines launched, with
> no prompt and no indicator. Only set this if you accept full
> responsibility for what's in every project directory you `cd` into.

## Git trust

Workspace trust also gates how Helix opens git repositories. Untrusted
workspaces are opened in [gix](https://github.com/Byron/gitoxide)'s
`Trust::Reduced` mode, which disables risky configuration like
`core.fsmonitor`, `core.sshCommand`, `gpg.openpgp.program`, and similar
options that can execute arbitrary commands from `.git/config`. Trusted
workspaces use `Trust::Full`.
