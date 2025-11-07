## Workspace Trust

### Workspace
In Helix, a workspace is defined as either a singular file or a folder which itself or one of its parents contain the following folders:
`.helix`
`.git`
`.jj`
`.svn`

### Trust
When a workspace is trusted, LSPs, formatters, debuggers and workspace config are allowed.
> ⚠️
> Any of the above can lead to remote code execution. Ensure you trust the source of a workspace before trusting it.

Conversely, all of these are not allowed to be used when the workspace is untrusted.

### Default behavior
By default, you will see a `[Untrusted]` indicator on documents which are untrusted. All documents which haven't been trusted are untrusted.
Each time you open a new folder/file Helix will ask you if to trust it or not. See `Configuration` if you wish to adjust this behavior.

## Configuration
There are 4 options for workspace trust:

```toml
workspace-trust = "ask"
```
This is the default behavior of Helix. It will bring up a dialog each time you open a new workspace/file. Any workspace/file which haven't been explicitly trusted are considered untrusted.

```toml
workspace-trust = "manual"
```
This option is the same as "ask", except it won't ask on startup. You may bring up the dialog manually with `:trust-dialog`. You can also trust/untrust with `:trust-workspace`/`:untrust-workspace`.


```toml
workspace-trust = "always"
```
Will always trust any workspace.

```toml
workspace-trust = "never"
```
Will never trust any workspace.
