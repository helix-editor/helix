# rx-helix customization guide

Everything in this fork is configured through Helix's normal config file and
themes. There are no plugins and no separate config format.

## Where things live

| Path | Purpose |
| --- | --- |
| `~/.config/helix/config.toml` | Editor, statusline, and dashboard settings (main) |
| `~/.config/helix/config.d/*.toml` | Modular overrides — all `*.toml` are merged alphabetically on top of `config.toml` (e.g. `10-keys.toml`, `20-vim.toml`) |
| `~/.config/helix/themes/<name>.toml` | Theme and `ui.*` scopes |
| `contrib/rx-helix/config.toml` | Complete drop-in sample |
| `contrib/config.d/vim.toml` | Vim muscle-memory module (copy of `LGUG2Z/helix-vim`) |

Modular config is implemented in `helix-term/src/config.rs`: the main `config.toml`
is loaded first, then every `config.d/*.toml` (sorted) is merged with
Each module's tables are deep-merged (nested keymap prefixes and statusline
mode tables keep their siblings); arrays are appended in file order and scalar
values are replaced. For
`hx -c /path/to/config.toml` the sibling `config.d` next to that file is used.
Workspace `.helix/config.toml` is still merged last if trusted.

```sh
mkdir -p ~/.config/helix/config.d
cp contrib/config.d/vim.toml ~/.config/helix/config.d/vim.toml  # vim bindings
cp contrib/rx-helix/config.d/vim.toml ~/.config/helix/config.d/vim.toml  # rx sample
# reload
:config-reload
```

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
mode.select-prompt = "SELECT"
mode.command = "COMMAND"
mode.search = "SEARCH"

mode.normal-short = "NOR"
mode.insert-short = "INS"
mode.select-short = "SEL"
mode.select-prompt-short = "SEL"
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

Command and search use `ui.statusline.command` / `ui.statusline.search` when
defined; otherwise they fall back to the secondary (`ui.statusline.select`)
and tertiary (`ui.statusline.insert`) mode chips so they stay distinct from
Normal. The `select:` regex prompt uses `ui.statusline.select`.

## Editor chrome

```toml
[editor]
cursorline = true
bufferline = "multiple"     # always | never | multiple
transparent-popups = true
popup-border = "all"        # none | popup | menu | all
```

`cursorline` highlights the line of every cursor with `ui.cursorline`.
`bufferline` renders open buffers at the top; `multiple` shows it only when
more than one buffer is open. Tabs have a fixed 24-cell width: a 1-cell left
indicator, a 20-cell label area, a gap, the modified dot and one trailing
padding cell. `TCP and UDP.md` is 14 cells and `Processes in OS.md` is 18, so
both fit; longer titles keep 16 cells and end with ` ...`. The tab row uses
`ui.bufferline.background` as the dark inactive bar; the current tab is lighter
(`ui.bufferline.active`) with white text, a thin `ui.bufferline.indicator`
line on its left, and a `ui.bufferline.modified` dot at its right edge for
modified buffers. Inactive tabs use `ui.bufferline`. A blank row separates the
tabs from the editor content.

The command palette completion list always uses a themed border.
`popup-border` additionally draws themed borders around hover, completion,
signature, and menu popups. Border foregrounds use the primary accent from
`ui.statusline.normal` (falling back to `ui.menu.selected`, then
`ui.cursor.primary`, then `ui.window`). Text uses the secondary
(`ui.statusline.select`) and tertiary (`ui.statusline.insert`) accents.
Border backgrounds follow the panel, so transparent panels keep transparent
borders. No fork-only theme scope is required for the fallback colors; a theme
may optionally define `ui.statusline.command` / `ui.statusline.search` to
override the Command/Search prompt chips.

Completion menus use `ui.menu.selected` for the selected row. Picker text and
selection use the same accent pair as the statusline chips: tertiary text,
secondary-on-primary selection, and a primary border. This keeps finder text
colored instead of resolving through the plain `ui.text` scopes.

`transparent-popups` replaces the background of completion docs, hover and
signature help, menus, and prompt completion with the terminal default. It
patches only those scopes at theme-load time (`ui.popup`, `ui.popup.info`,
`ui.menu`, `ui.help`), so foreground colors and generated theme files are left
untouched. Alternatively, set `bg = "none"` on individual scopes.

Terminal cursor effects (for example Kitty's `cursor_trail`) follow the native
terminal cursor. Helix's default `block` cursor is drawn in software and hides
that native cursor. Use `normal = "block-hardware"` in
`[editor.cursor-shape]` to keep the native cursor visible on top of the themed
block, or use `bar`/`underline`.

The active theme file is watched for changes and reloaded automatically with a
short debounce. Generated themes can therefore rewrite
`~/.config/helix/themes/<name>.toml` while the editor is open. A parse error
keeps the current theme instead of falling back to the default.

### Generated (Noctalia/matugen) themes

Do not edit a generated theme such as
`~/.config/helix/themes/noctalia.toml`; it is rewritten whenever the wallpaper
palette changes. Instead, install the `rx-theme` child theme:

```sh
cp contrib/themes/rx-theme.toml ~/.config/helix/themes/rx-theme.toml
cp contrib/rx-helix/config.d/99-theme.toml ~/.config/helix/config.d/99-theme.toml
```

`rx-theme.toml` uses `inherits = "noctalia"` and references the generated
palette roles (`primary`, `secondary`, `tertiary`, `surfaceContainer`, etc.),
so it follows wallpaper changes automatically. The `config.d` module keeps
`theme = "rx-theme"` selected even when the main `config.toml` is rewritten
back to `theme = "noctalia"` by the generator. Other bundled themes are
unaffected and continue to use their own styles; picker and heading rendering
fall back to standard theme scopes when the rx-specific scopes are absent.

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
| Header and primary logo shade | `ui.statusline.normal` accent |
| Menu items, keys, icons, secondary logo shade | `ui.statusline.select` accent |
| Tertiary logo shade | `ui.statusline.insert` accent |
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

Headings lose their `#` markers and gain level icons (`󰲡`–`󰲫`), bullet lists
use `● ○ ◆ ◇` by nesting depth, task boxes use `󰄱`/`󰱒`, block quotes use `▋`,
and horizontal rules (`---`/`***`/`___`) become a full-width `─` line. Heading
rows use an explicit `markup.heading.1`–`.6` background when present; otherwise
a theme can opt into render-markdown-style Diff backgrounds with
`markup.heading.adaptive`, which derives H1–H6 colors from the theme palette.
The fallback still uses existing UI scopes (`ui.selection`, `ui.menu`,
`ui.popup`, `ui.cursorline`). Icons use `markup.heading.1`–`.6`; rules use
`ui.linenr`; code blocks get a full-width `ui.cursorline` background with a
`󰅩 language` header on the fence line and inline code gets a background.
The buffer text is unchanged: rendering paints over the rendered document after
each line, so movement, editing, saving, and searching still see ordinary
markdown. The cursor line and visual selections show raw markdown so markers
stay editable and readable.

Links, wiki links, and callouts are decorated without shifting text:
`[[target|alias]]` shows `󱗖 alias`, `[label](url)` keeps the label and puts an
icon where the destination was, `<url>` shows its icon plus the address,
images show `󰥶`, and footnotes show `󰯔`. Inline-link icons adapt to the
destination host (`󰊤` GitHub, `󰊭` Google, `󰌹` generic web, etc.), with custom
icons taking precedence:

```toml
[[editor.markdown.link-icons]]
pattern = "intranet"
icon = "󰢩"
```

Links inside
headings, list items, quotes, tables, and callout titles are all decorated.
Obsidian callouts (`> [!NOTE]`, `[!WARNING]-`, etc.) render their icon and
title with `markup.callout.*` colors (falling back to the matching diagnostic
color without its curl underline), custom titles replace the default name, and
the `▋` markers of following quote lines reuse the same color until a blank or
non-quote line ends the block.
Pipe tables replace `|` with `│` (delimiter `-`/`:` become `─`/`━`) and give
the header row a background. Inline code, emphasis/strong/strikethrough,
links, wiki links, and footnotes inside table cells are rendered before the
cell is padded, so markers do not leak into the table. Inline code gets a
background across the whole span, not just the backticks. The cursor line and
selections still show raw Markdown so markers stay editable.
Code fences keep their tree-sitter highlighting.

Local images are drawn inline when the terminal speaks the Kitty graphics
protocol:

```toml
[editor.markdown]
render = true
images = true
```

`![](assets/foo.png)` and `![[foo.png]]` resolve relative to the note, then the
vault root. The Markdown line keeps its alt text/icon visible and the image
starts below it, matching image.nvim's placement. Each image reserves virtual
lines after its anchor (like image.nvim's `with_virtual_padding`), so
following text reflows instead of being covered. Images that scroll partially
out of view are cropped to the visible rows via a source rectangle instead of
being squished. Remote `http(s)` images download once with `curl` into the
cache (disable with `download_remote_images = false`) and appear on later
frames. Opening an image file directly shows it full-view with the buffer text
blanked. PNG images are sent directly; JPEG/GIF are converted with `ffmpeg`
and SVG with `rsvg-convert` into a cache under `~/.cache/rx-helix/images`.
Each image is scaled to at most 50% of the window height and re-placed as you
scroll. Only the focused view is drawn and images are hidden while a
picker/prompt/menu covers the editor.

## Spell checking

Markdown, plain text and similar prose files are spell checked automatically
with `hunspell` when it is available:

```toml
[editor.spellcheck]
enable = true
language = "en_US"
```

Misspelled words get the red `diagnostic.spell` curl underline (falling back
to `diagnostic.error` in other themes). Code blocks, inline code, links, URLs,
HTML tags, math, and YAML frontmatter are ignored. Navigation and suggestions
mirror Neovim:

| Key | Action |
| --- | ------ |
| `[s` | Previous misspelled word |
| `]s` | Next misspelled word |
| `z=` | Open numbered/lettered suggestions for the word under the cursor; press `1`–`9`, `0`, `a`–`z` to apply one |

Spell diagnostics are hints, so they do not add inline diagnostic popups or
compete with LSP warnings/errors.

## Buffer navigation

In normal mode, `H` and `L` switch to the previous/next open buffer when more
than one buffer is open. They are no-ops with a single buffer and do not
affect the `C-w H` / `C-w L` window-swap bindings.

The sample Vim keymap module also opens a new line and enters Insert mode:

```toml
[keys.normal]
o = ["open_below", "insert_mode"]
O = ["open_above", "insert_mode"]
```

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

**No syntax highlighting.** The source build was launched directly and the
runtime could not be found. `cargo run -p helix-term -- file` sets the runtime
automatically. A binary built from this checkout also falls back to the source
tree's `runtime/` directory at compile time, so an installed `hx` keeps working
while that checkout exists. Otherwise set `HELIX_RUNTIME="$PWD/runtime"`.

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
