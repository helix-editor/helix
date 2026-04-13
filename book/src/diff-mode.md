# Diff mode

Diff mode opens two files side-by-side with hunks aligned. Added lines get a green tint, deleted lines red, and modified lines amber. Characters that changed within a line get their own highlight.

## Opening a diff

From the command line:

```sh
hx --diff file1 file2
hx -d file1 file2
```

From inside the editor, with two file paths:

```text
:diff-open file1 file2
```

To diff two buffers you already have open, run `:diff-this` in each view in turn. Running it in the second view links both into a diff session.

## Navigating

Use `]g` and `[g` to jump between changed hunks, the same bindings that navigate VCS diff hunks.

## Transferring changes

| Command              | Aliases              | What it does                                                                                                      |
| -------------------- | -------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `:diff-put`          | `:diffput`, `:diffp` | Push the hunk under the cursor to the partner buffer.                                                             |
| `:reset-diff-change` | `:diffget`, `:diffg` | In a diff session: pull the hunk from the partner buffer. Outside a diff session: reset the hunk to the VCS base. |

## Ending a diff session

`:diff-off` closes the diff session for the current view. Both buffers stay open as independent documents.

## Theme keys

These scopes apply in diff mode:

| Scope             | Purpose                       |
| ----------------- | ----------------------------- |
| `diff.delta`      | Modified line background      |
| `diff.delta.text` | Intra-line changed characters |
| `diff.minus`      | Deleted line background       |
| `diff.plus`       | Added line background         |

These scopes apply to the VCS diff gutter (the bar to the left of line numbers showing uncommitted changes), not diff mode:

| Scope                 | Purpose                          |
| --------------------- | -------------------------------- |
| `diff.delta.conflict` | Merge conflict marker in pickers |
| `diff.delta.gutter`   | `▍` bar for modified lines       |
| `diff.delta.moved`    | Renamed or moved file in pickers |
| `diff.minus.gutter`   | `▔` mark for deleted lines       |
| `diff.plus.gutter`    | `▍` bar for added lines          |
