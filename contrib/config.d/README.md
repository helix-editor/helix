# Modular Config

Helix now supports modular configuration via `~/.config/helix/config.d/*.toml`.

## How it works

`helix-term/src/config.rs` loads `~/.config/helix/config.toml` first, then
alphabetically merges every `*.toml` in `~/.config/helix/config.d/` on top of
it (and on top of each other) using `merge_toml_values(..., 3)`. If you pass
`hx -c /path/to/config.toml`, the sibling `config.d` next to that file is used
instead.

This lets you split your config into logical modules:

```
~/.config/helix/
  config.toml              # theme, editor, statusline, dashboard
  config.d/
    10-keys.toml           # general keys (K = hover, etc.)
    20-vim.toml            # vim muscle-memory (helix-vim)
    30-obsidian.toml       # obsidian pickers
    40-work.toml           # work-specific overrides
```

Order is alphabetical, so prefix with `10-`, `20-` to control precedence.
Local `.helix/config.toml` (workspace) is still merged last if the workspace
is trusted.

## Example

Copy the vim module:

```sh
mkdir -p ~/.config/helix/config.d
cp contrib/config.d/vim.toml ~/.config/helix/config.d/vim.toml
# or for rx-helix sample:
cp contrib/rx-helix/config.d/vim.toml ~/.config/helix/config.d/vim.toml
```

Then reload: `:config-reload` or `pkill -USR1 hx`.

## Included modules

- `vim.toml` — Full Vim muscle-memory from
  [helix-vim](https://github.com/LGUG2Z/helix-vim) (see that file for details).
  It overrides `w`, `b`, `e`, `W`, `B`, `E`, `0`, `$`, `^`, `G`, `%`, `V`, `C`,
  `D`, `S`, `x`, `p`, `P`, `Y`, `*`, `#`, `j`, `k`, `u`, `esc`, and adds
  `d`/`y` submodes. `C-o`/`C-r` map to `:config-open`/`:config-reload`.

Add your own: `cp config.toml config.d/my-keys.toml` and trim to just the
`[keys]` tables you want.
