# Fork maintenance

This repo is a personal fork of [helix-editor/helix]. It is kept current
automatically; upstream work never gets merged into `main` by hand.

## Branch model

| branch   | contents                                   | rule                          |
| -------- | ------------------------------------------ | ----------------------------- |
| `master` | byte-for-byte mirror of `upstream/master`  | never commit here             |
| `main`  | every customization, one commit per feature| only rebased onto `master`    |

`main` is the default branch and what you build from. Your editor config
(`config.toml`, themes, keymaps) stays in `~/.config/helix`, not in `runtime/`,
so it never collides with upstream files.

## Syncing

Automatically, daily, via `.github/workflows/sync-upstream.yml`: it
fast-forwards `master`, rebases `main`, runs `cargo check`, and only pushes
`main` if the check passes. Conflicts and check failures become an issue.

Locally, the same thing:

```sh
scripts/sync-upstream.sh            # sync + build check
scripts/sync-upstream.sh --no-build # fast, no cargo check
```

Exit codes: `0` synced, `1` conflict, `2` check failed, `3` unsafe state.
On `1`/`2` `main` is left exactly where it was, so your daily driver keeps
building:

```sh
git switch main
git rebase master     # resolve, then:
HELIX_DISABLE_AUTO_GRAMMAR_BUILD=1 cargo check -p helix-term
git push --force-with-lease origin main
```

## Running your build

The binary does not locate the repo's `runtime/` by itself, so running
`./target/debug/hx` directly opens files without grammar queries (no syntax
highlighting). Run it through cargo, which exports the manifest dir:

```sh
cargo run -p helix-term -- <args>
# or
HELIX_RUNTIME=$PWD/runtime ./target/debug/hx <args>
```

## Writing patches that survive rebases

- One commit per feature; rebase needs to be able to drop or reorder them.
- Gate behavior behind a config field. `helix-view/src/editor.rs` (config
  structs) and `helix-term/src/ui/statusline.rs` are the files you will touch
  repeatedly; keep edits there small.
- Prefer new files (`helix-term/src/ui/dashboard.rs`) plus one hook over
  rewriting `helix-term/src/ui/editor.rs`, the hottest file upstream.
- `git rerere` is enabled, so a conflict resolved once is resolved again
  automatically on later rebases.
- After a rebase, review what moved:

  ```sh
  git range-diff master@{1} master main@{1} main
  ```

[helix-editor/helix]: https://github.com/helix-editor/helix
