# rx-helix customization guide

Everything in this fork is configured through Helix's normal config file and
themes. There are no plugins and no separate config format.

## Where things live

| Path | Purpose |
| --- | --- |
| `~/.config/helix/config.toml` | Editor, statusline, and dashboard settings |
| `~/.config/helix/themes/<name>.toml` | Theme and `ui.*` scopes |
| `contrib/rx-helix/config.toml` | Complete drop-in sample |

After editing the config, run `:config-reload` inside Helix. Dashboard layout
changes (including `layout = "auto"`) are evaluated on the next redraw, so
resizing the terminal switches orientation without restarting.

> **You must actually use the config.** Upstream defaults apply when the
> `[editor.statusline]` / `[editor.dashboard]` sections are absent, and the
> dashboard is disabled unless `enable = true`. If you are testing the source
> build, run it through `cargo run` or set `HELIX_RUNTIME`; otherwise grammar
> queries are missing and syntax highlighting will not appear.

## Statusline

The statusline is one row. It is split into `left`, `center`, and `right`
regions; the regions can have independent backgrounds.

```toml
[editor.statusline]
left = ["mode", "spacer", "version-control", "diagnostics", "file-name"]
center = []
right = ["position", "file-type", "file-line-ending", "total-line-numbers"]
separator = "│"

transparent = ["center"]   # [] = all regions opaque
reserve-space = false
accent = ["position"]
mode-short-width = 100

mode.normal = "NORMAL"
mode.insert = "INSERT"
mode.select = "SELECT"
mode.command = "COMMAND"
mode.search = "SEARCH"

mode.normal-short = "NOR"
mode.insert-short = "INS"
mode.select-short = "SEL"
mode.command-short = "COM"
mode.search-short = "S"
```

### Options

| Key | Description | Default |
| --- | --- | --- |
| `left` / `center` / `right` | Element lists | upstream defaults |
| `separator` | Separator text | `"│"` |
| `transparent` | Regions drawn without a background: `left`, `center`, `right` | `[]` |
| `reserve-space` | Reserve cells for an idle spinner / unmodified buffer | `true` |
| `accent` | Elements painted with the current mode colors | `[]` |
| `mode-short-width` | Use `mode.*-short` at or below this column count | unset |
| `mode.normal` / `.insert` / `.select` | Mode labels | `NOR` / `INS` / `SEL` |
| `mode.command` / `.search` | Command/search chip labels | `COMMAND` / `SEARCH` |
| `mode.*-short` | Labels used at the short breakpoint | `NOR` / `INS` / `SEL` / `COM` / `S` |

`accent` accepts any statusline element name. Use it to make a single element
follow the mode chip, for example:

```toml
accent = ["position", "total-line-numbers"]
```

### Inline command and search

When `:` is open, the row renders only the mode chip, one blank cell, and the
command line; everything else is transparent. When `/` is open, the pattern is
followed by the current `line:column` right-aligned. If `position` is in
`accent`, that position uses the mode colors.

The chip values come from `mode.command` / `mode.command-short` and
`mode.search` / `mode.search-short`. Short labels apply at the
`mode-short-width` breakpoint.

### Theme scopes

```toml
"ui.statusline" = { fg = "onSurface", bg = "surfaceContainerLow" }
"ui.statusline.inactive" = { fg = "onSurface", bg = "surfaceContainerLowest" }
"ui.statusline.normal" = { fg = "onPrimary", bg = "primary", modifiers = ["bold"] }
"ui.statusline.insert" = { fg = "onTertiary", bg = "tertiary", modifiers = ["bold"] }
"ui.statusline.select" = { fg = "onSecondary", bg = "secondary", modifiers = ["bold"] }
```

Command and search currently use the normal-mode chip colors.

## Dashboard

The dashboard replaces the empty buffer when Helix starts without files. It is
an overlay: the scratch buffer underneath is not modified. Opening a file or
running `:new` dismisses it; cancelling a picker returns to it.

```toml
[editor.dashboard]
enable = true
layout = "auto"
```

### Layout

| Value | Behavior |
| --- | --- |
| `auto` | Pick the orientation that fits; recomputed on every resize |
| `vertical` | Header/subtitle/items/footer stacked and centered |
| `horizontal` | Header left, subtitle/items/footer right |

`auto` shrinks the item block to its content width before giving up, so a
horizontal layout can still fit a slightly narrow terminal. If both orientations
fit, vertical is preferred unless the terminal is at least three times as wide
as it is tall. If
neither orientation fits, the one with the smallest overflow is used and a
warning is printed in the statusline.

| Key | Description | Default |
| --- | --- | --- |
| `layout` | `auto`, `vertical`, `horizontal` | `"auto"` |
| `align` | Vertical layout: `left` or `center` blocks | `"center"` |
| `vertical-align` | `top` or `center` | `"center"` |
| `section-gap` | Blank lines between blocks | `1` |
| `item-gap` | Blank lines between items | `0` |
| `menu-style` | `space-between` or `key-left` | `"space-between"` |
| `menu-width` | Fixed width of the item block for `space-between`; `0` sizes it from content | `0` |
| `menu-gap` | Gap between descriptions and keys when `menu-width` is `0`. A number applies to both layouts; a table can set `vertical`/`horizontal` separately | `8` |
| `key-width` | Key column width for `key-left` | `16` |
| `column-gap` | Gap between columns in horizontal layout | `8` |
| `default-logo` | Use the built-in logo when no header is set | `true` |

`menu-gap` accepts either form:

```toml
# same gap in both layouts
menu-gap = 8

# or per layout
[editor.dashboard.menu-gap]
vertical = 8
horizontal = 12
```

### Header and logo

`header` is a multiline string. `header-file` reads it from a file instead and
takes precedence. `%{version}` expands to the Helix version in any text block.

```toml
header = """
  your ascii art
"""
# or
header-file = "~/.config/helix/logo.txt"
```

When both are empty, the built-in logo is drawn. Horizontal uses the dot-matrix
mark with a small `ʜᴇʟɪx` caption; vertical pairs the dot mark with the `HELIX`
wordmark. Set `default-logo = false` to draw nothing instead. Content wider or
taller than the terminal is clipped and reported once in the statusline.

### Items

```toml
[[editor.dashboard.items]]
key = "f"
icon = " "
desc = "Find File"
action = "file_picker"

[[editor.dashboard.items]]
key = "n"
desc = "New File"
action = ":new"

[[editor.dashboard.items]]
key = ":tutor<enter>"
desc = "learn helix"
# no action: static row
```

`action` accepts the same values as keymap entries: a command name
(`file_picker`, `buffer_picker`, `global_search`, `insert_mode`) or a typable
command (`:new`, `:quit`, `:theme`). Omit it for a static row.

`key` is displayed and, if it parses to a single key (`f`, `<space>`,
`<C-x>`), also triggers the action. Multi-key strings are display-only. While
the dashboard is visible, keys that do not start a keymap sequence or open
command mode are swallowed.

### Theme scopes

The dashboard does not need custom `ui.dashboard.*` scopes. It maps onto scopes
that generated themes (matugen/Noctalia) always keep:

| Element | Source |
| --- | --- |
| Header, icons, primary logo shade | `ui.statusline.normal` accent |
| Secondary logo shade | `ui.statusline.select` accent |
| Tertiary logo shade | `ui.statusline.insert` accent |
| Menu items, keys, and icons | `ui.statusline.normal` accent |
| Subtitle, footer, muted logo shade | `ui.text.inactive` |

The accent is the background color of those statusline scopes, reused as a
foreground color so no background block is drawn. If a statusline scope is
missing, the dashboard falls back to `ui.text`. This means wallpaper changes
and theme regeneration cannot leave the dashboard uncolored.

## Markdown rendering

Optional display-only markdown rendering:

```toml
[editor.markdown]
render = true
```

Headings lose their `#` markers and gain level icons, bullet lists use `•`,
task boxes use `󰄱`/`󰱒`, block quotes use `│`, and horizontal rules use `─`.
The buffer text is unchanged: rendering is built from single-grapheme overlays,
so movement, editing, saving, and searching still see ordinary markdown. Only
visible lines are scanned.

Code fences are left as-is and keep their tree-sitter highlighting. Links,
callouts, and tables are not decorated yet.

Local images can be drawn inline when the terminal speaks the Kitty graphics
protocol:

```toml
[editor.markdown]
render = true
images = true
```

`![](assets/foo.png)` and `![[foo.png]]` resolve relative to the note, then the
vault root. PNG images are sent directly; JPEG/GIF are converted with `ffmpeg`
and SVG with `rsvg-convert` into a cache under `~/.cache/rx-helix/images`. Each
image is scaled to at most 50% of the window height and re-placed as you
scroll. Only the focused view is drawn. Remote URLs are skipped.

## Obsidian

Optional vault integration. Requires `rg` for vault search and a vault path.

```toml
[editor.obsidian]
enable = true
vault = "~/Documents/Obsidian/Lessons"
notes-subdir = "6 - Main Notes"
templates-subdir = "5 - Templates"
attachments-subdir = "assets"
date-format = "%d-%m-%Y %H:%M %p"
time-format = "%H:%M"
```

| Command | Description |
| ------- | ----------- |
| `:obsidian-quick-switch` | Fuzzy note picker across the vault |
| `:obsidian-search [query]` | Ripgrep the vault; prompts when no query is given |
| `:obsidian-new [title]` | Create `notes-subdir/<title>.md` and open it |
| `:obsidian-rename [title]` | Rename the current note and rewrite links to it |
| `:obsidian-template [name]` | Apply a template at the cursor |
| `:obsidian-follow-link` | Open a `[[wiki]]` or `[markdown](link)` under the cursor |
| `:obsidian-backlinks` | Picker of notes that link to the current note |
| `:obsidian-link` | Replace the selection with a link to a picked note |
| `:obsidian-link-new` | Create a note from the selection and link it |
| `:obsidian-extract-note` | Move the selection into a new note and link it |
| `:obsidian-paste-image` | Paste a clipboard PNG/JPEG into `attachments-subdir` |
| `:obsidian-toggle-checkbox` | Toggle `[ ]` / `[x]` on the current line |
| `:obsidian-toc` | Insert a heading table of contents |
| `:obsidian-links` | Pick and follow a link in the current note |
| `:obsidian-tags` | Pick a vault tag and insert it |
| `:obsidian-smart-action` | Follow a link or toggle a checkbox under the cursor |

Suggested keymaps:

```toml
[keys.normal.space.o]
o = ":obsidian-quick-switch"
g = ":obsidian-search"
n = ":obsidian-new"
m = ":obsidian-rename"
r = ":obsidian-rename"
t = ":obsidian-template"
f = ":obsidian-follow-link"
b = ":obsidian-backlinks"
i = ":obsidian-paste-image"
c = ":obsidian-toggle-checkbox"
d = ":obsidian-toc"
l = ":obsidian-links"
s = ":obsidian-tags"

[keys.select.space.o]
l = ":obsidian-link"
n = ":obsidian-link-new"
e = ":obsidian-extract-note"
```

Notes:

- Note IDs are file stems; links resolve by title or vault-relative path.
- Templates are excluded from quick switch and backlinks.
- Templates replace `{{title}}`, `{{date}}`, and `{{time}}` placeholders.
- Rename rewrites wiki links and markdown note links across the vault.
- `paste-image` requires `wl-paste` and a PNG/JPEG image on the Wayland
  clipboard.
- `follow-link`, `backlinks`, `links`, and `toc` require the current buffer to
  be a file inside the vault.
- The quick-switch picker uses a telescope-like footer: `<CR>` opens the
  selected note, `<C-x>` creates a note from the typed query, and `<C-l>`
  inserts `[[Title]]` for the selected note into the current buffer.
- Typing `[[` in a markdown note opens the note picker and completes a wiki
  link after you choose a note.

A mergeable snippet lives in
[`contrib/rx-helix/obsidian.example.toml`](../contrib/rx-helix/obsidian.example.toml).

## Complete example

The full drop-in sample lives at
[`contrib/rx-helix/config.toml`](../contrib/rx-helix/config.toml). It enables the
single-line statusline, inline command/search labels, per-region transparency,
accent position, and the auto dashboard with six shortcuts.

```sh
cp contrib/rx-helix/config.toml ~/.config/helix/config.toml
```

## Troubleshooting

**No syntax highlighting.** The source build was launched directly. Use
`cargo run -p helix-term -- file` or
`HELIX_RUNTIME="$PWD/runtime" ./target/debug/hx file`.

**No dashboard.** Check `enable = true`, that Helix was started without files,
and that the focused buffer is the startup scratch. `:new`, `:vnew`, `:hnew`,
`:tutor`, and opening a file all dismiss it.

**Dashboard clipped.** The statusline reports the chosen layout size and the
terminal size. Use `layout = "auto"`, reduce `menu-width`/`column-gap`, or set a
smaller header.

**Header file ignored.** Check the path and its permissions; a read failure is
reported once in the statusline and falls back to `header` / the built-in logo.

**Changes do not appear.** Configuration must be in
`~/.config/helix/config.toml` (or a local `.helix/config.toml`) and reloaded with
`:config-reload`. Source changes require a rebuild.

## Fork maintenance

- `master` — pure mirror of `helix-editor/helix`.
- `main` — all rx-helix changes, rebased on `master` by the daily workflow.

See [FORK.md](../FORK.md) for the branch model and recovery commands.
