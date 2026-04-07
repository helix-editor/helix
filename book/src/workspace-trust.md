# Workspace trust

Helix has a number of potentially dangerous features, namely LSP and ability to use workspace-local configuration files. Those features can lead to unexpected code execution. To protect against code execution in dangerous contexts, Helix has a workspace trust protection, which will prevent these potentially dangerous features from running automatically.

Helix will not trust any workspace by default.

By default, it will prompt about trust when you open a new file in a workspace where you didn't make a decision about trust yet.

If you decide not to trust a workspace and don't want to be prompted about trust each session, you can exclude the workspace by choosing `Never` option in trust selection window.

You can always make current workspace trusted by running `:workspace-trust` command, and untrust it with `:workspace-untrust`.

The list of trusted and permanently untrusted workspaces, delimited by newline characters, are stored in `~/.local/share/helix/trusted_workspaces` and `~/.local/share/helix/excluded_workspaces` respectively.
<!-- TODO: Windows paths -->

# Configuration

You can return to the old behaviour to accept the risk of loading every local `.helix/config.toml` and `.helix/languages.toml` and starting LSPs without an explicit permission by setting the following option:

```toml
[editor.trust]
paths = [ "**" ]
```

In addition to trusting any workspaces with the wildcard glob `**`, it is possible to configure trust fine-grained with a `.gitignore` like syntax. E.g. consider the following:

```toml
[editor.trust]
paths = [
  "~/repos/helix",
  "~/repos/foo/*",
  "~/repos/bar/**",
  "!~/repos/bar/untrusted"
]
```

This would result in the following trust levels assuming the home directory `/home/user`:

| Path                                            | Decision            |
|:----------------------------------------------- |:------------------- |
| `/home/user/foobar`                             | undecided           |
| `/home/user/repos/helix`                        | trusted             |
| `/home/other/repos/helix`                       | undecided           |
| `/home/user/repos/helix/branch_a`               | undecided           |
| `/home/user/repos/foo`                          | undecided           |
| `/home/user/repos/foo/branch_a`                 | trusted             |
| `/home/user/repos/foo/remote_a/branch_a`        | undecided           |
| `/home/user/repos/bar/branch_a`                 | trusted             |
| `/home/user/repos/bar/remote_a/branch_a`        | trusted             |
| `/home/user/repos/bar/untrusted`                | untrusted           |

Specifically, the paths are processed one entry at a time by expanding a leading `~/` to the user's home directory, expanding a `*` to any single path segment, and expanding `**` to any number of path segments. An entry prefixed with `!` is a negated entry, instead of granting trust to a matching workspace it will deny it. After processing the full list, the most recently matched entry "wins". If no entry matched (e.g. because the default configuration with an empty list of `paths` was used), the user will be prompted as described in the beginning.

A secure configuration that will never prompt would be:

```toml
paths = [
  "!**",
  "~/repos/helix",
  ...
```

Above configuration will
