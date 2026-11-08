# rx-helix

A personal fork of [Helix](https://github.com/helix-editor/helix) with UI and
quality-of-life changes built around one idea: everything the editor shows on
its last line should be useful, compact, and configurable.

The fork tracks upstream `master` automatically. `master` is a pure mirror of
Helix; all custom work lives on `main`.

## Features

### One-line statusbar

The statusline and the command/status area share a single bottom row.

- `:` opens an inline command line: a `COMMAND` / `C` chip, one blank cell, then
  the command text and cursor. The `:` prefix is not echoed.
- `/` opens an inline search prompt: a `SEARCH` / `S` chip, the pattern, and the
  current `line:column` right-aligned with the accent style.
- Status messages and pending keys render in the middle region instead of
  painting over the mode chip and the right-side modules.

### Statusbar configuration

- Per-region transparency (`left`, `center`, `right`).
- Responsive mode labels: full labels on wide terminals, short labels below a
  configurable column count.
- Configurable labels for normal, insert, select, command, and search.
- `reserve-space` to stop empty spinner/modification elements from leaving
  padded blocks.
- `accent` to paint selected elements with the current mode colors.

### Dashboard

Shown when Helix starts without files:

- `auto`, `vertical`, or `horizontal` layout. `auto` recomputes on resize, makes
  the item block shrink to fit, and falls back to the orientation with the
  smallest overflow.
- Optional header text, `header-file`, or the built-in dot-matrix mark (paired
  with the `HELIX` wordmark in vertical layout).
- Shortcut rows with icons, descriptions, and any Helix command as the action.
- Key-hint and hover popups stay above the statusline.
- Unbound keys are swallowed while the dashboard is visible; cancelling a picker
  returns to it, opening a file dismisses it.

### Upstream sync

A daily GitHub Actions workflow fast-forwards `master`, rebases `main`, runs a
build check, and opens an issue if a human is needed. See
[FORK.md](FORK.md).

## Build and run

```sh
cargo build -p helix-term
cargo run -p helix-term -- path/to/file
```

> **Important:** run the development build through `cargo run`, or point it at
> the repo's runtime explicitly:
>
> ```sh
> HELIX_RUNTIME="$PWD/runtime" ./target/debug/hx path/to/file
> ```
>
> Running `./target/debug/hx` without `HELIX_RUNTIME` opens files without
> grammar queries, so syntax highlighting is missing.

First build note: `helix-term/build.rs` fetches tree-sitter grammars. If your
DNS is flaky, the fetch can take minutes; once the grammars and `.so` files are
built, later builds only do fast revision checks.

## Configuration

Helix reads `~/.config/helix/config.toml` and `~/.config/helix/themes/*.toml`.

A complete drop-in example is in
[`contrib/rx-helix/config.toml`](contrib/rx-helix/config.toml). Copy it to
`~/.config/helix/config.toml` to see all the fork features at once:

```sh
mkdir -p ~/.config/helix
cp contrib/rx-helix/config.toml ~/.config/helix/config.toml
```

You must actually use the config for the fork features to appear: with no
`[editor.statusline]`/`[editor.dashboard]` section, Helix uses its upstream
defaults. The dashboard is disabled unless `enable = true`.

Detailed documentation:

- [Customization guide](docs/CUSTOMIZATION.md) — statusline, dashboard, themes,
  sample config, and troubleshooting.
- [Statusline reference](book/src/editor.md#editorstatusline-section)
- [Dashboard reference](book/src/editor.md#editordashboard-section)

`:config-reload` re-reads the config. Dashboard layout changes take effect on the
next redraw and orientation switches live as the terminal is resized.

## What this fork is not

- Not a Vim emulation layer. Helix motions are the point; learn them and keep
  the selection-first model.
- Not a plugin-based configuration. Everything is compiled in; configuration is
  data-driven through `config.toml` and themes.

## Credits

- [Helix](https://github.com/helix-editor/helix) — the editor this fork is built
  on.
- [rx-neovim](https://github.com/RanXom) — the statusline and dashboard layout
  this fork ports to Helix.
- The dot-matrix logo and shading scheme come from
  [helix-editor/helix#13197](https://github.com/helix-editor/helix/pull/13197).

## License

MPL-2.0, same as Helix. See [LICENSE](LICENSE).
