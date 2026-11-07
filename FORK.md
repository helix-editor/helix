# Fork maintenance

This repo is a personal fork of [helix-editor/helix]. It is kept current
automatically; upstream work never gets merged into `local` by hand.

## Branch model

| branch   | contents                                   | rule                          |
| -------- | ------------------------------------------ | ----------------------------- |
| `master` | byte-for-byte mirror of `upstream/master`  | never commit here             |
| `local`  | every customization, one commit per feature| only rebased onto `master`    |

`local` is the default branch and what you build from. Your editor config
(`config.toml`, themes, keymaps) stays in `~/.config/helix`, not in `runtime/`,
so it never collides with upstream files.

## Syncing

Automatically, daily, via `.github/workflows/sync-upstream.yml`: it
fast-forwards `master`, rebases `local`, runs `cargo check`, and only pushes
`local` if the check passes. Conflicts and check failures become an issue.

Locally, the same thing:

```sh
scripts/sync-upstream.sh            # sync + build check
scripts/sync-upstream.sh --no-build # fast, no cargo check
```

Exit codes: `0` synced, `1` conflict, `2` check failed, `3` unsafe state.
On `1`/`2` `local` is left exactly where it was, so your daily driver keeps
building:

```sh
git switch local
git rebase master     # resolve, then:
HELIX_DISABLE_AUTO_GRAMMAR_BUILD=1 cargo check -p helix-term
git push --force-with-lease origin local
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
  git range-diff master@{1} master local@{1} local
  ```

[helix-editor/helix]: https://github.com/helix-editor/helix
