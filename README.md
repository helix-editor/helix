# RX-Helix

RX-Helix is a custom fork of [Helix](https://github.com/helix-editor/helix) meant
to provide the best no-hassle experience: a single bottom line, inline
command/search prompts, and a startup dashboard that stays out of your way —
all driven by plain `config.toml`, with no plugins or package manager.

## Highlights

- **One-line UI** — the statusline, `:` command line, and `/` search share the
  bottom row.
- **Inline prompts** — `COMMAND` / `SEARCH` chips, inline text and cursor, no
  echoed `:` prefix.
- **Configurable statusline** — per-region transparency, responsive short mode
  labels, reserved-space control, accent elements.
- **Startup dashboard** — `auto` layout, live resize switching, content-sized
  menus, built-in dot-matrix/RX-HELIX logo, shortcuts that run any Helix command.
- **Bufferline** — fixed 24-cell tabs, a lighter active tab, a left indicator,
  a modified dot, and clean overflow for long names.
- **Themed pickers** — Telescope/mini.pick-style roles, centered Quick Switch
  titles, colored directory prefixes, and live theme reloads.
- **Markdown workspace** — rendered headings, callouts, tables, inline code
  backgrounds, local images, wiki links, and Hunspell spell checking.
- **Syntax on by default** — 300+ Tree-sitter grammars built into the install,
  with a generic `.conf` fallback and targeted nginx/systemd/hyprlang/bitbake,
  tmux, and fontconfig detection.
- **No plugins** — the whole experience is compiled in and configured through
  `~/.config/helix/config.toml`.
- **Upstream-safe** — `master` is a pure mirror of Helix; the daily workflow
  rebases `main` onto it automatically.

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

### Bufferline

`bufferline = "multiple"` shows open buffers above the editor. Tabs have a
fixed 24-cell width with a left indicator, a lighter active tab, a modified
dot, and a ` ...` suffix when a name is too long.

### Dashboard

Shown when Helix starts without files:

- `auto`, `vertical`, or `horizontal` layout. `auto` recomputes on resize, makes
  the item block shrink to fit, and falls back to the orientation with the
  smallest overflow.
- Optional header text, `header-file`, or the built-in dot-matrix mark (paired
  with the `RX-HELIX` wordmark in vertical layout).
- Shortcut rows with icons, descriptions, and any Helix command as the action.
- Key-hint and hover popups stay above the statusline.
- Unbound keys are swallowed while the dashboard is visible; cancelling a picker
  returns to it, opening a file dismisses it.

### Pickers and theme

Pickers follow the Neovim Telescope/mini.pick roles instead of collapsing to a
single accent color:

- `ui.picker.selected` selects a row, `ui.picker.match` highlights fuzzy
  matches, `ui.picker.border` draws the frame, and `ui.picker.title` labels it.
- Quick Switch centers its title and colors directory prefixes with
  `ui.text.directory`, keeping filenames in `ui.text`.
- Picker columns resolve styles from the live theme on every render, so folder
  colors update after a theme reload without reopening the picker.

The `rx-theme` child theme maps Helix UI colors to Neovim highlight roles,
including the dashboard, which-key/Info popup, Telescope-style pickers, and the
`Pmenu` selection colors.

### Obsidian vault

Optional built-in vault integration:

- `:obsidian-quick-switch` note picker.
- `:obsidian-search` vault-wide ripgrep search.
- `:obsidian-new` notes in the configured notes folder.
- `:obsidian-follow-link` for `[[wiki]]` and markdown links.
- `:obsidian-backlinks` for the current note.

### Markdown rendering

`[editor.markdown] render = true` replaces heading markers with icons, bullets
with `•`, task boxes with `󰄱`/`󰱒`, quotes with `│`, and rules with `─` using
buffer overlays, so the file text is unchanged.

- Callouts use `markup.callout.*` colors and never inherit diagnostic underlines.
- Tables render inline code, emphasis, strong text, strikethrough, links, wiki
  links, footnotes, and autolinks; a row stays rendered while the cursor is on it.
- Inline code backgrounds cover the whole span, not just the backticks.
- `images = true` draws local markdown images inline in Kitty-compatible
  terminals, with alt text kept visible below the anchor.

### Syntax highlighting

Tree-sitter grammars provide syntax highlighting, while language servers remain
optional for completion and diagnostics. The optimized install ships 300+
compiled grammars, so no separate grammar step is required.

Generic `.conf` files fall back to the `properties` grammar, and keys render in
the theme's bold `property` color while values use `string`. More specific
globs still win: nginx, systemd, hyprlang, and bitbake keep their grammars, tmux
config fragments use `bash`, and fontconfig XML uses `xml`.

### Spell checking

Hunspell integration underlines misspellings in red, offers `[s` / `]s` to jump
between diagnostics, and opens numbered or lettered suggestions with `z=`.

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
> A binary compiled from this checkout also records the source tree's
> `runtime/` directory at build time and falls back to it when no runtime is
> found next to the executable. `cargo install --path helix-term` therefore
> keeps syntax highlighting as long as the checkout stays in place. Set
> `HELIX_RUNTIME` to override the fallback.

First build note: `helix-term/build.rs` fetches tree-sitter grammars. If your
DNS is flaky, the fetch can take minutes; once the grammars and `.so` files are
built, later builds only do fast revision checks.

## Configuration

Helix reads `~/.config/helix/config.toml` and `~/.config/helix/themes/*.toml`.
Additional `~/.config/helix/config.d/*.toml` modules are merged alphabetically
on top of `config.toml`, including nested tables and appended arrays. A
ready-to-copy Vim keybind module lives in `contrib/config.d/vim.toml`.

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
