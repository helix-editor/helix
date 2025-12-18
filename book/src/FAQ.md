# FAQ

## Table of Contents

- [How to...](#how-to)
  - [Run Helix](#run-helix)
  - [Learn Helix](#learn-helix)
  - [Collapse to single cursor after using multiple cursors / Keep only primary cursor](#collapse-to-single-cursor-after-using-multiple-cursors--keep-only-primary-cursor)
  - [Change cursor shape on mode change (bar cursor on insert mode, block on normal mode, etc)](#change-cursor-shape-on-mode-change-bar-cursor-on-insert-mode-block-on-normal-mode-etc)
  - [Map jk or jj to exit insert mode](#map-jk-or-jj-to-exit-insert-mode)
  - [Map unicode characters like ö to keybinding](#map-unicode-characters-like-ö-to-keybinding)
  - [Use my terminal’s 16 color palette as a theme](#use-my-terminal’s-16-color-palette-as-a-theme)
  - [Perform find-and-replace](#perform-find-and-replace)
  - [Strip whitespace or format the buffer](#strip-whitespace-or-format-the-buffer)
  - [Access the Helix config directory](#access-the-helix-config-directory)
  - [Access the log file](#access-the-log-file)
  - [Add a language](#add-a-language)
  - [Change grammars at project level](#change-grammars-at-project-level)
  - [Close the LSP documentation popup](#close-the-lsp-documentation-popup)
- [General Questions](#general-questions)
  - [How to write plugins / Is there a plugin system in place yet?](#how-to-write-plugins--is-there-a-plugin-system-in-place-yet)
  - [When will the next release be?](#when-will-the-next-release-be)
  - [Is a Vi/Vim keymap planned?](#is-a-vivim-keymap-planned)
  - [Can the j/k bindings be changed to ignore soft wrapping when using a count like 3j](#can-the-jk-bindings-be-changed-to-ignore-soft-wrapping-when-using-a-count-like-3j)
  - [Pressing x when on an empty line selects the next line, is that a bug/how do I change that?](#pressing-x-when-on-an-empty-line-selects-the-next-line,-is-that-a-bughow-do-i-change-that)
  - [How do I build or run code from within Helix?](#how-do-i-build-or-run-code-from-within-helix)
  - [Save file without formatting](#save-file-without-formatting)
  - [Are LSP extensions supported?](#are-lsp-extensions-supported)
- [Installation](#installation)
  - [Error when building tree-sitter language grammars in Fedora](#error-when-building-tree-sitter-language-grammars-in-fedora)


## How to...

### Run Helix

In the terminal:

```shell
hx
```

### Learn Helix

Start Helix tutorial:

```shell
hx --tutor
```

### Collapse to single cursor after using multiple cursors / Keep only primary cursor

Use the default keybind `,` bound to the `keep_primary_selection` command.

### Change cursor shape on mode change (bar cursor on insert mode, block on normal mode, etc)

Add this to your [`config.toml`](https://docs.helix-editor.com/configuration.html#configuration):

```toml
[editor.cursor-shape]
insert = "bar"
normal = "block"
select = "underline"
```

### Map `jk` or `jj` to exit insert mode

Add this to your [`config.toml`](https://docs.helix-editor.com/configuration.html#configuration):

```toml
[keys.insert]
j = { k = "normal_mode" }
```

### Map unicode characters like `ö` to keybinding

The TOML standard requires that these characters are quoted:

```toml
[keys.normal]
ö = "extend_line_up" # This line is invalid TOML
"ö" = "extend_line_up" # This line is valid TOML
```

### Use my terminal's 16 color palette as a theme

Add this to your [`config.toml`](https://docs.helix-editor.com/configuration.html#configuration):

```toml
# to see more "adaptive" themes,
# type `:theme 16_` in Normal mode.
theme = "base16_terminal"
```

Refer to [Theme docs](https://github.com/helix-editor/helix/wiki/Themes)

- You can also use color names like `red`, `light-blue`, etc to refer to
  the terminal's colors in a theme file; refer the
  https://docs.helix-editor.com/themes.html#color-palettes[theme color palette]
  documentation.

See also: ["Ability to define second theme to sync with OS"](https://github.com/helix-editor/helix/discussions/10281)

### Perform find-and-replace

Type `%` to select the entire file, then `s` to bring up a `select:` prompt.
Enter your search, and press enter. All matches in the file will be selected;
you can now use `c` to change them all simultaneously.

To make search fully case sensitive add the following to `config.toml`:

```toml
[editor.search]
smart-case = false
```

### Strip whitespace or format the buffer

If the LSP for the language is active and supports autoformat, and the auto-format
option is on (check your and the repo's `languages.toml`), then this will happen
on save. If there is an alternative command you can run in the terminal to format,
you can pipe the whole buffer to it manually with `%|<formatter><enter>`.

### Access the Helix config directory

You can use `:config-open` to open the config in Helix.

| Platform          | Location                                       |
|-------------------|------------------------------------------------|
| Mac OS/Linux      | `~/.config/helix`                              |
| Windows           | `C:\Users\<User>\AppData\Roaming\helix`        | 

### Access the log file

Enable logging via the `-v` flag, with each use (up to `-vvv`) increasing the verbosity.
However, `-vv` and `-vvv` are only useful for developing Helix.
`hx -v` is sufficient for diagnosing issues with language servers.

You can use `:log-open` to open the log in Helix.

| Platform          | Location                                                |
|-------------------|---------------------------------------------------------|
| Mac OS/Linux      | `~/.cache/helix/helix.log`                              |
| Windows           | `C:\Users\____\AppData\Local\helix\helix.log`           | 

### Add a language

Check https://docs.helix-editor.com/guides/adding_languages.html

### Change grammars at project level

You can specify custom grammars per-project/per-directory by placing the `languages.toml`
in `.helix/languages.toml` at the root of your project. See https://docs.helix-editor.com/languages.html

### Close the LSP documentation popup

`Ctrl`-`c` closes popups like LSP signature-help, hover, and auto-completion.

## General Questions

#### How to write plugins / Is there a plugin system in place yet?

Status as of December 2022 (originally posted [here](https://github.com/helix-editor/helix/discussions/3806#discussioncomment-4438007_)):

> There's two prototypes we're exploring that could potentially exist side by
> side: a typed list/ML-like implementation for scripting and a Rust based interface
> for things that require performance. Could potentially run both in wasm but
> I'm personally a bit unhappy with how big wasm implementations are,
> easily several orders of magnitude compared to the editor


As of February 2024, this is being worked on in https://github.com/helix-editor/helix/pull/8675

Past discussions:

- [Initial discussion](https://github.com/helix-editor/helix/issues/122)
- [Pre-RFC discussion](https://github.com/helix-editor/helix/discussions/580)
- [Plugin system MVP](https://github.com/helix-editor/helix/pull/455)

#### When will the next release be?

Releases don't have exact timelines. The maintainers aim for a few releases per
year and cut a release when they feel that enough changes have collected in
master and the branch has stabilized.

#### Is a Vi/Vim keymap planned?

We are not interested in supporting alternative paradigms. The core of Helix's
editing is based on `Selection -> Action`, and it would require extensive changes
to create a true Vi/Vim keymap. However, there is a third-party keymap: https://github.com/LGUG2Z/helix-vim

#### Can the `j`/`k` bindings be changed to ignore soft wrapping when using a count like `3j`

`j` and `k` are intentionally mapped to *visual* vertical movement. This is a
more intuitive default that makes working with heavily soft-wrapped text much easier.
**Textual** vertical movement is bound to `gk` and `gj`. So you can use `3gj`
and `3gk` instead of `3j` and `3k` to jump to a relative line number.

These commands are intentionally separate (with no special casing for `count
!= 0`) as they represent the fundamental vertical movement primitives. All
other vertical movement behavior can be created by combining these commands
using conditions. For example: 

```scheme
(if (!= count 0) (move_line_up count) (move_vertical_line_up 0))
```

If these fundamental primitives had such special handling built in,
that would limit what could be implemented. Furthermore, helix is slightly
opinionated towards unsurprising and consistent behavior.

#### Pressing `x` when on an empty line selects the next line, is that a bug/how do I change that?

This behavior is by design. Pressing `x` will extend the selection to the current
line unless the current line is already selected. If the line is already selected
it will extend the selection to the next line. This allows repeatedly pressing x
to quickly select a few lines.

In the case of an empty line, the entire line is already selected (since there is only a newline character on the line).
The intention is to use selections interactively so you would not press x in this case because the line is already selected.
For example, if you wanted to delete an empty line you would just press `d`.

In cases where you always want to extend the selection to the line bounds and never
want to extend to the next line you can use `X`. For example, if you want a key
combination to blindly mash to delete a line that would be `Xd`. 

#### How do I build or run code from within Helix?

You can `:run-shell-command` like this `:sh echo "hello world!"` and it'll show the output in a pop-up. Examples:

- `:! cargo check`
- `:! npm run build`
- `:! make tests`

We recommend running code (`npm start`, `cargo run`, etc...) in a separate terminal pane/tab or split, especially for TUI or GUI apps.

#### Save file without formatting

`:w --no-format`. Alternatively, you can temporarily toggle off formatting with `:toggle auto-format`, then toggle it back on again

#### Are LSP extensions supported?

Helix aims to support only the official parts of the LSP specification in its codebase.

Some language servers extend the LSP specification to add custom methods and
notifications. For example `rust-analyzer` adds a custom `rust-analyzer/expandMacro`
request to provide its macro expansion feature: https://rust-analyzer.github.io/manual.html#expand-macro-recursively.

Extensions to the LSP spec should be implemented in language-specific plugins once the plugin system is available.

## Installation

### Error when building tree-sitter language grammars in Fedora

Ensure that you have a C compiler installed:

```shell
sudo dnf group install "C Development Tools and Libraries"
```
