
# 0.5.0 (2021-10-28)

A big shout out to all the contributors! We had 46 contributors in this release.

Helix has popped up in [Scoop, FreeBSD Ports and Gentu GURU](https://repology.org/project/helix/versions)!

The following is a quick rundown of the larger changes, there were many more
(check the git history for more details).

Breaking changes:

- A couple of keymaps moved to resolve a few conflicting keybinds.
  - Documentation popups were moved from `K` to `space+k`
  - `K` is now `keep_selections` which filters selections to only keeps ones matching the regex
  - `keep_primary_selection` moved from `space+space` to `,`
  - `Alt-,` is now `remove_primary_selection` which keeps all selections except the primary one
  - Opening files in a split moved from `C-h` to `C-s`
- Some configuration options moved from a `[terminal]` section to `[editor]`. [Consult the documentation for more information.](https://docs.helix-editor.com/configuration.html)

Features:

- LSP compatibility greatly improved for some implementations (Julia, Python, Typescript)
- Autocompletion! Completion now triggers automatically after a set idle timeout
- Completion documentation is now displayed next to the popup ([#691](https://github.com/helix-editor/helix/pull/691))
- Treesitter textobjects (select a function via `mf`, class via `mc`) ([#728](https://github.com/helix-editor/helix/pull/728)) 
- Global search across entire workspace `space+/` ([#651](https://github.com/helix-editor/helix/pull/651)) 
- Relative line number support ([#485](https://github.com/helix-editor/helix/pull/485))
- Prompts now store a history (72cf86e)
- `:vsplit` and `:hsplit` commands ([#639](https://github.com/helix-editor/helix/pull/639))
- `C-w h/j/k/l` can now be used to navigate between splits ([#860](https://github.com/helix-editor/helix/pull/860))
- `C-j` and `C-k` are now alternative keybindings to `C-n` and `C-p` in the UI ([#876](https://github.com/helix-editor/helix/pull/876))
- Shell commands (shell-pipe, pipe-to, shell-insert-output, shell-append-output, keep-pipe) ([#547](https://github.com/helix-editor/helix/pull/547))
- Searching now defaults to smart case search (case insensitive unless uppercase is used) ([#761](https://github.com/helix-editor/helix/pull/761))
- The preview pane was improved to highlight and center line ranges
- The user `languages.toml` is now merged into defaults, no longer need to copy the entire file (dc57f8dc)
- Show hidden files in completions ([#648](https://github.com/helix-editor/helix/pull/648))
- Grammar injections are now properly handled (dd0b15e)
- `v` in select mode now switches back to normal mode ([#660](https://github.com/helix-editor/helix/pull/660))
- View mode can now be triggered as a "sticky" mode ([#719](https://github.com/helix-editor/helix/pull/719))
- `f`/`t` and object selection motions can now be repeated via `Alt-.` ([#891](https://github.com/helix-editor/helix/pull/891))
- Statusline now displays total selection count and diagnostics counts for both errors and warnings ([#916](https://github.com/helix-editor/helix/pull/916))

New grammars:

- Ledger ([#572](https://github.com/helix-editor/helix/pull/572))
- Protobuf ([#614](https://github.com/helix-editor/helix/pull/614))
- Zig ([#631](https://github.com/helix-editor/helix/pull/631))
- YAML ([#667](https://github.com/helix-editor/helix/pull/667))
- Lua ([#665](https://github.com/helix-editor/helix/pull/665))
- OCaml ([#666](https://github.com/helix-editor/helix/pull/666))
- Svelte ([#733](https://github.com/helix-editor/helix/pull/733))
- Vue ([#787](https://github.com/helix-editor/helix/pull/787))
- Tree-sitter queries ([#845](https://github.com/helix-editor/helix/pull/845))
- CMake ([#888](https://github.com/helix-editor/helix/pull/888))
- Elixir (we switched over to the official grammar) (6c0786e)
- Language server definitions for Nix and Elixir ([#725](https://github.com/helix-editor/helix/pull/725))
- Python now uses `pylsp` instead of `pyls`
- Python now supports indentation

New themes:

- Monokai ([#628](https://github.com/helix-editor/helix/pull/628))
- Everforest Dark ([#760](https://github.com/helix-editor/helix/pull/760))
- Nord ([#799](https://github.com/helix-editor/helix/pull/799))
- Base16 Default Dark ([#833](https://github.com/helix-editor/helix/pull/833))
- Rose Pine ([#897](https://github.com/helix-editor/helix/pull/897))

Fixes:

- Fix crash on empty rust file ([#592](https://github.com/helix-editor/helix/pull/592))
- Exit select mode after toggle comment ([#598](https://github.com/helix-editor/helix/pull/598))
- Pin popups with no positioning to the initial position (12ea3888)
- xsel copy should not freeze the editor (6dd7dc4)
- `*` now only sets the search register and doesn't jump to the next occurrence (3426285)
- Goto line start/end commands extend when in select mode ([#739](https://github.com/helix-editor/helix/pull/739)) 
- Fix documentation popups sometimes not getting fully highlighted (066367c)
- Refactor apply_workspace_edit to remove assert (b02d872)
- Wrap around the top of the picker menu when scrolling (c7d6e44)
- Don't allow closing the last split if there's unsaved changes (3ff5b00)
- Indentation used different default on hx vs hx new_file.txt (c913bad)

# 0.4.1 (2021-08-14)

A minor release that includes:
- A fix for rendering glitches that would occur after editing with multiple selections.
- CI fix for grammars not being cross-compiled for aarch64 

# 0.4.0 (2021-08-13)

A big shout out to all the contributors! We had 28 contributors in this release.

Two months have passed, so this is another big release. A big thank you to all
the contributors and package maintainers!

Helix has popped up in [Arch, Manjaro, Nix, MacPorts and Parabola and Termux repositories](https://repology.org/project/helix/versions)!

A [large scale refactor](https://github.com/helix-editor/helix/pull/376) landed that allows us to support zero width (empty)
selections in the future as well as resolves many bugs and edge cases.

- Multi-key remapping! Key binds now support much more complex usecases ([#454](https://github.com/helix-editor/helix/pull/454))
- Pending keys are shown in the statusline ([#515](https://github.com/helix-editor/helix/pull/515))
- Object selection / textobjects. `mi(` to select text inside parentheses ([#385](https://github.com/helix-editor/helix/pull/385))
- Autoinfo: `whichkey`-like popups which show available sub-mode shortcuts ([#316](https://github.com/helix-editor/helix/pull/316))
- Added WORD movements (W/B/E) ([#390](https://github.com/helix-editor/helix/pull/390))
- Vertical selections (repeat selection above/below) ([#462](https://github.com/helix-editor/helix/pull/462))
- Selection rotation via `(` and `)` ([66a90130](https://github.com/helix-editor/helix/commit/66a90130a5f99d769e9f6034025297f78ecaa3ec))
- Selection contents rotation via `Alt-(` and `Alt-)` ([02cba2a](https://github.com/helix-editor/helix/commit/02cba2a7f403f48eccb18100fb751f7b42373dba))
- Completion behavior improvements ([f917b5a4](https://github.com/helix-editor/helix/commit/f917b5a441ff3ae582358b6939ffbf889f4aa530), [627b899](https://github.com/helix-editor/helix/commit/627b89931576f7af86166ae8d5cbc55537877473))
- Fixed a language server crash ([385a6b5a](https://github.com/helix-editor/helix/commit/385a6b5a1adddfc26e917982641530e1a7c7aa81))
- Case change commands (`` ` ``, `~`, ``<a-`>``) ([#441](https://github.com/helix-editor/helix/pull/441))
- File pickers (including goto) now provide a preview! ([#534](https://github.com/helix-editor/helix/pull/534))
- Injection query support. Rust macro calls and embedded languages are now properly highlighted ([#430](https://github.com/helix-editor/helix/pull/430))
- Formatting is now asynchronous, and the async job infrastructure has been improved ([#285](https://github.com/helix-editor/helix/pull/285))
- Grammars are now compiled as separate shared libraries and loaded on-demand at runtime ([#432](https://github.com/helix-editor/helix/pull/432))
- Code action support ([#478](https://github.com/helix-editor/helix/pull/478))
- Mouse support ([#509](https://github.com/helix-editor/helix/pull/509), [#548](https://github.com/helix-editor/helix/pull/548))
- Native Windows clipboard support ([#373](https://github.com/helix-editor/helix/pull/373))
- Themes can now use color palettes ([#393](https://github.com/helix-editor/helix/pull/393))
- `:reload` command ([#374](https://github.com/helix-editor/helix/pull/374))
- Ctrl-z to suspend ([#464](https://github.com/helix-editor/helix/pull/464))
- Language servers can now be configured with a custom JSON config ([#460](https://github.com/helix-editor/helix/pull/460))
- Comment toggling now uses a language specific comment token ([#463](https://github.com/helix-editor/helix/pull/463))
- Julia support ([#413](https://github.com/helix-editor/helix/pull/413))
- Java support ([#448](https://github.com/helix-editor/helix/pull/448))
- Prompts have an (in-memory) history ([63e54e30](https://github.com/helix-editor/helix/commit/63e54e30a74bb0d1d782877ddbbcf95f2817d061))

# 0.3.0 (2021-06-27)

A big shout out to all the contributors! We had 24 contributors in this release.

Another big release. 

Highlights:

- Indentation is now automatically detected from file heuristics. ([#245](https://github.com/helix-editor/helix/pull/245))
- Support for other line endings (CRLF). Significantly improved Windows support. ([#224](https://github.com/helix-editor/helix/pull/224))
- Encodings other than UTF-8 are now supported! ([#228](https://github.com/helix-editor/helix/pull/228))
- Key bindings can now be configured via a `config.toml` file ([#268](https://github.com/helix-editor/helix/pull/268))
- Theme can now be configured and changed at runtime ([please feel free to contribute more themes!](https://github.com/helix-editor/helix/tree/master/runtime/themes)) ([#267](https://github.com/helix-editor/helix/pull/267))
- System clipboard yank/paste is now supported! ([#310](https://github.com/helix-editor/helix/pull/310))
- Surround commands were implemented ([#320](https://github.com/helix-editor/helix/pull/320))

Features:

- File picker can now be repeatedly filtered ([#232](https://github.com/helix-editor/helix/pull/232))
- LSP progress is now received and rendered as a spinner ([#234](https://github.com/helix-editor/helix/pull/234))
- Current line number can now be themed ([#260](https://github.com/helix-editor/helix/pull/260))
- Arrow keys & home/end now work in insert mode ([#305](https://github.com/helix-editor/helix/pull/305))
- Cursors and selections can now be themed ([#325](https://github.com/helix-editor/helix/pull/325))
- Language servers are now gracefully shut down before `hx` exits ([#287](https://github.com/helix-editor/helix/pull/287))
- `:show-directory`/`:change-directory` ([#335](https://github.com/helix-editor/helix/pull/335))
- File picker is now sorted by access time (before filtering) ([#336](https://github.com/helix-editor/helix/pull/336))
- Code is being migrated from helix-term to helix-view (prerequisite for
  alternative frontends) ([#366](https://github.com/helix-editor/helix/pull/366))
- `x` and `X` merged
  ([f41688d9](https://github.com/helix-editor/helix/commit/f41688d960ef89c29c4a51c872b8406fb8f81a85))

Fixes:

- The IME popup is now correctly positioned ([#273](https://github.com/helix-editor/helix/pull/273))
- A bunch of bugs regarding `o`/`O` behavior ([#281](https://github.com/helix-editor/helix/pull/281))
- `~` expansion now works in file completion ([#284](https://github.com/helix-editor/helix/pull/284))
- Several UI related overflow crashes ([#318](https://github.com/helix-editor/helix/pull/318))
- Fix a test failure occuring only on `test --release` ([4f108ab1](https://github.com/helix-editor/helix/commit/4f108ab1b2197809506bd7305ad903a3525eabfa))
- Prompts now support unicode input ([#295](https://github.com/helix-editor/helix/pull/295))
- Completion documentation no longer overlaps the popup ([#322](https://github.com/helix-editor/helix/pull/322))
- Fix a crash when trying to select `^` ([9c534614](https://github.com/helix-editor/helix/commit/9c53461429a3e72e3b1fb87d7ca490e168d7dee2))
- Prompt completions are now paginated ([39dc09e6](https://github.com/helix-editor/helix/commit/39dc09e6c4172299bc79de4c1c52288d3f624bd7))
- Goto did not work on Windows ([503ca112](https://github.com/helix-editor/helix/commit/503ca112ae57ebdf3ea323baf8940346204b46d2))

# 0.2.1

Includes a fix where wq/wqa could exit before file saving completed.

# 0.2.0

A big shout out to all the contributors! We had 18 contributors in this release.

Enough has changed to bump the version. We're skipping 0.1.x because
previously the CLI would always report version as 0.1.0, and we'd like
to distinguish it in bug reports..

- The `runtime/` directory is now properly detected on binary releases and
  on cargo run. `~/.config/helix/runtime` can also be used.
- Registers can now be selected via " (for example `"ay`)
- Support for Nix files was added
- Movement is now fully tested and matches kakoune implementation
- A per-file LSP symbol picker was added to space+s
- Selection can be replaced with yanked text via R

- `1g` now correctly goes to line 1
- `ctrl-i` now correctly jumps backwards in history
- A small memory leak was fixed, where we tried to reuse tree-sitter
  query cursors, but always allocated a new one
- Auto-formatting is now only on for certain languages
- The root directory is now provided in LSP initialization, fixing
  certain language servers (typescript)
- LSP failing to start no longer panics
- Elixir language queries were fixed

# 0.0.10

Keymaps:
- Add mappings to jump to diagnostics
- Add gt/gm/gb mappings to jump to top/middle/bottom of screen
- ^ and $ are now gh, gl

- The runtime/ can now optionally be embedded in the binary
- Haskell syntax added
- Window mode (ctrl-w) added
- Show matching bracket (vim's matchbrackets)
- Themes now support style modifiers
- First user contributed theme
- Create a document if it doesn't exist yet on save
- Detect language on a new file on save

- Panic fixes, lots of them
