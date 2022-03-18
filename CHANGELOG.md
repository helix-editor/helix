
# 0.6.0 (2022-01-04)

Happy new year and a big shout out to all the contributors! We had 55 contributors in this release.

Helix has popped up in DPorts and Fedora Linux via COPR ([#1270](https://github.com/helix-editor/helix/pull/1270))

As usual the following is a brief summary, refer to the git history for a full log:

Breaking changes:

- fix: Normalize backtab into shift-tab

Features:

- Macros ([#1234](https://github.com/helix-editor/helix/pull/1234))
- Add reverse search functionality ([#958](https://github.com/helix-editor/helix/pull/958))
- Allow keys to be mapped to sequences of commands ([#589](https://github.com/helix-editor/helix/pull/589))
- Make it possible to keybind TypableCommands ([#1169](https://github.com/helix-editor/helix/pull/1169))
- Detect workspace root using language markers ([#1370](https://github.com/helix-editor/helix/pull/1370))
- Add WORD textobject ([#991](https://github.com/helix-editor/helix/pull/991))
- Add LSP rename_symbol (`space-r`) ([#1011](https://github.com/helix-editor/helix/pull/1011))
- Added workspace_symbol_picker ([#1041](https://github.com/helix-editor/helix/pull/1041))
- Detect filetype from shebang line ([#1001](https://github.com/helix-editor/helix/pull/1001))
- Allow piping from stdin into a buffer on startup ([#996](https://github.com/helix-editor/helix/pull/996))
- Add auto pairs for same-char pairs ([#1219](https://github.com/helix-editor/helix/pull/1219))
- Update settings at runtime ([#798](https://github.com/helix-editor/helix/pull/798))
- Enable thin LTO ([`cccc194`](https://github.com/helix-editor/helix/commit/cccc194))

Commands:

- `:wonly` -- window only ([#1057](https://github.com/helix-editor/helix/pull/1057))
- buffer-close (`:bc`, `:bclose`) ([#1035](https://github.com/helix-editor/helix/pull/1035))
- Add `:<line>` and `:goto <line>` commands ([#1128](https://github.com/helix-editor/helix/pull/1128))
- `:sort` command ([#1288](https://github.com/helix-editor/helix/pull/1288))
- Add m textobject for pair under cursor ([#961](https://github.com/helix-editor/helix/pull/961))
- Implement "Goto next buffer / Goto previous buffer" commands ([#950](https://github.com/helix-editor/helix/pull/950))
- Implement "Goto last modification" command ([#1067](https://github.com/helix-editor/helix/pull/1067))
- Add trim_selections command ([#1092](https://github.com/helix-editor/helix/pull/1092))
- Add movement shortcut for history ([#1088](https://github.com/helix-editor/helix/pull/1088))
- Add command to inc/dec number under cursor ([#1027](https://github.com/helix-editor/helix/pull/1027))
  - Add support for dates for increment/decrement
- Align selections (&) ([#1101](https://github.com/helix-editor/helix/pull/1101))
- Implement no-yank delete/change ([#1099](https://github.com/helix-editor/helix/pull/1099))
- Implement black hole register ([#1165](https://github.com/helix-editor/helix/pull/1165))
- `gf` as goto_file (`gf`) ([#1102](https://github.com/helix-editor/helix/pull/1102))
- Add last modified file (`gm`) ([#1093](https://github.com/helix-editor/helix/pull/1093))
- ensure_selections_forward ([#1393](https://github.com/helix-editor/helix/pull/1393))
- Readline style insert mode ([#1039](https://github.com/helix-editor/helix/pull/1039))

Usability improvements and fixes:

- Detect filetype on `:write` ([#1141](https://github.com/helix-editor/helix/pull/1141))
- Add single and double quotes to matching pairs ([#995](https://github.com/helix-editor/helix/pull/995))
- Launch with defaults upon invalid config/theme (rather than panicking) ([#982](https://github.com/helix-editor/helix/pull/982))
- If switching away from an empty scratch buffer, remove it ([#935](https://github.com/helix-editor/helix/pull/935))
- Truncate the starts of file paths instead of the ends in picker ([#951](https://github.com/helix-editor/helix/pull/951))
- Truncate the start of file paths in the StatusLine ([#1351](https://github.com/helix-editor/helix/pull/1351))
- Prevent picker from previewing binaries or large file ([#939](https://github.com/helix-editor/helix/pull/939))
- Inform when reaching undo/redo bounds ([#981](https://github.com/helix-editor/helix/pull/981))
- search_impl will only align cursor center when it isn't in view ([#959](https://github.com/helix-editor/helix/pull/959))
- Add `<C-h>`, `<C-u>`, `<C-d>`, Delete in prompt mode ([#1034](https://github.com/helix-editor/helix/pull/1034))
- Restore screen position when aborting search ([#1047](https://github.com/helix-editor/helix/pull/1047))
- Buffer picker: show is_modifier flag ([#1020](https://github.com/helix-editor/helix/pull/1020))
- Add commit hash to version info, if present ([#957](https://github.com/helix-editor/helix/pull/957))
- Implement indent-aware delete ([#1120](https://github.com/helix-editor/helix/pull/1120))
- Jump to end char of surrounding pair from any cursor pos ([#1121](https://github.com/helix-editor/helix/pull/1121))
- File picker configuration ([#988](https://github.com/helix-editor/helix/pull/988))
- Fix surround cursor position calculation ([#1183](https://github.com/helix-editor/helix/pull/1183))
- Accept count for goto_window ([#1033](https://github.com/helix-editor/helix/pull/1033))
- Make kill_to_line_end behave like emacs ([#1235](https://github.com/helix-editor/helix/pull/1235))
- Only use a single documentation popup ([#1241](https://github.com/helix-editor/helix/pull/1241))
- ui: popup: Don't allow scrolling past the end of content ([`3307f44c`](https://github.com/helix-editor/helix/commit/3307f44c))
- Open files with spaces in filename, allow opening multiple files ([#1231](https://github.com/helix-editor/helix/pull/1231))
- Allow paste commands to take a count ([#1261](https://github.com/helix-editor/helix/pull/1261))
- Auto pairs selection ([#1254](https://github.com/helix-editor/helix/pull/1254))
- Use a fuzzy matcher for commands ([#1386](https://github.com/helix-editor/helix/pull/1386))
- Add `<C-s>` to pick word under doc cursor to prompt line & search completion ([#831](https://github.com/helix-editor/helix/pull/831))
- Fix `:earlier`/`:later` missing changeset update ([#1069](https://github.com/helix-editor/helix/pull/1069))
- Support extend for multiple goto ([#909](https://github.com/helix-editor/helix/pull/909))
- Add arrow-key bindings for window switching ([#933](https://github.com/helix-editor/helix/pull/933))
- Implement key ordering for info box ([#952](https://github.com/helix-editor/helix/pull/952))

LSP:
- Implement MarkedString rendering ([`e128a8702`](https://github.com/helix-editor/helix/commit/e128a8702))
- Don't panic if init fails ([`d31bef7`](https://github.com/helix-editor/helix/commit/d31bef7))
- Configurable diagnostic severity ([#1325](https://github.com/helix-editor/helix/pull/1325))
- Resolve completion item ([#1315](https://github.com/helix-editor/helix/pull/1315))
- Code action command support ([#1304](https://github.com/helix-editor/helix/pull/1304))

Grammars:

- Adds mint language server ([#974](https://github.com/helix-editor/helix/pull/974))
- Perl ([#978](https://github.com/helix-editor/helix/pull/978)) ([#1280](https://github.com/helix-editor/helix/pull/1280))
- GLSL ([#993](https://github.com/helix-editor/helix/pull/993))
- Racket ([#1143](https://github.com/helix-editor/helix/pull/1143))
- WGSL ([#1166](https://github.com/helix-editor/helix/pull/1166))
- LLVM ([#1167](https://github.com/helix-editor/helix/pull/1167)) ([#1388](https://github.com/helix-editor/helix/pull/1388)) ([#1409](https://github.com/helix-editor/helix/pull/1409)) ([#1398](https://github.com/helix-editor/helix/pull/1398))
- Markdown ([`49e06787`](https://github.com/helix-editor/helix/commit/49e06787))
- Scala ([#1278](https://github.com/helix-editor/helix/pull/1278))
- Dart ([#1250](https://github.com/helix-editor/helix/pull/1250))
- Fish ([#1308](https://github.com/helix-editor/helix/pull/1308))
- Dockerfile ([#1303](https://github.com/helix-editor/helix/pull/1303))
- Git (commit, rebase, diff) ([#1338](https://github.com/helix-editor/helix/pull/1338)) ([#1402](https://github.com/helix-editor/helix/pull/1402)) ([#1373](https://github.com/helix-editor/helix/pull/1373))
- tree-sitter-comment ([#1300](https://github.com/helix-editor/helix/pull/1300))
- Highlight comments in c, cpp, cmake and llvm ([#1309](https://github.com/helix-editor/helix/pull/1309))
- Improve yaml syntax highlighting highlighting ([#1294](https://github.com/helix-editor/helix/pull/1294))
- Improve rust syntax highlighting ([#1295](https://github.com/helix-editor/helix/pull/1295))
- Add textobjects and indents to cmake ([#1307](https://github.com/helix-editor/helix/pull/1307))
- Add textobjects and indents to c and cpp ([#1293](https://github.com/helix-editor/helix/pull/1293))

New themes:

- Solarized dark ([#999](https://github.com/helix-editor/helix/pull/999))
- Solarized light ([#1010](https://github.com/helix-editor/helix/pull/1010))
- Spacebones light ([#1131](https://github.com/helix-editor/helix/pull/1131))
- Monokai Pro ([#1206](https://github.com/helix-editor/helix/pull/1206))
- Base16 Light and Terminal ([#1078](https://github.com/helix-editor/helix/pull/1078))
  - and a default 16 color theme, truecolor detection
- Dracula ([#1258](https://github.com/helix-editor/helix/pull/1258))

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
- Prompts now store a history ([`72cf86e`](https://github.com/helix-editor/helix/commit/72cf86e))
- `:vsplit` and `:hsplit` commands ([#639](https://github.com/helix-editor/helix/pull/639))
- `C-w h/j/k/l` can now be used to navigate between splits ([#860](https://github.com/helix-editor/helix/pull/860))
- `C-j` and `C-k` are now alternative keybindings to `C-n` and `C-p` in the UI ([#876](https://github.com/helix-editor/helix/pull/876))
- Shell commands (shell-pipe, pipe-to, shell-insert-output, shell-append-output, keep-pipe) ([#547](https://github.com/helix-editor/helix/pull/547))
- Searching now defaults to smart case search (case insensitive unless uppercase is used) ([#761](https://github.com/helix-editor/helix/pull/761))
- The preview pane was improved to highlight and center line ranges
- The user `languages.toml` is now merged into defaults, no longer need to copy the entire file ([`dc57f8dc`](https://github.com/helix-editor/helix/commit/dc57f8dc))
- Show hidden files in completions ([#648](https://github.com/helix-editor/helix/pull/648))
- Grammar injections are now properly handled ([`dd0b15e`](https://github.com/helix-editor/helix/commit/dd0b15e))
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
- Elixir (we switched over to the official grammar) ([`6c0786e`](https://github.com/helix-editor/helix/commit/6c0786e))
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
- Pin popups with no positioning to the initial position ([`12ea3888`](https://github.com/helix-editor/helix/commit/12ea3888))
- xsel copy should not freeze the editor ([`6dd7dc4`](https://github.com/helix-editor/helix/commit/6dd7dc4))
- `*` now only sets the search register and doesn't jump to the next occurrence ([`3426285`](https://github.com/helix-editor/helix/commit/3426285))
- Goto line start/end commands extend when in select mode ([#739](https://github.com/helix-editor/helix/pull/739)) 
- Fix documentation popups sometimes not getting fully highlighted ([`066367c`](https://github.com/helix-editor/helix/commit/066367c))
- Refactor apply_workspace_edit to remove assert ([`b02d872`](https://github.com/helix-editor/helix/commit/b02d872))
- Wrap around the top of the picker menu when scrolling ([`c7d6e44`](https://github.com/helix-editor/helix/commit/c7d6e44))
- Don't allow closing the last split if there's unsaved changes ([`3ff5b00`](https://github.com/helix-editor/helix/commit/3ff5b00))
- Indentation used different default on hx vs hx new_file.txt ([`c913bad`](https://github.com/helix-editor/helix/commit/c913bad))

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
- Selection rotation via `(` and `)` ([`66a90130`](https://github.com/helix-editor/helix/commit/66a90130a5f99d769e9f6034025297f78ecaa3ec))
- Selection contents rotation via `Alt-(` and `Alt-)` ([`02cba2a`](https://github.com/helix-editor/helix/commit/02cba2a7f403f48eccb18100fb751f7b42373dba))
- Completion behavior improvements ([`f917b5a4`](https://github.com/helix-editor/helix/commit/f917b5a441ff3ae582358b6939ffbf889f4aa530), [`627b899`](https://github.com/helix-editor/helix/commit/627b89931576f7af86166ae8d5cbc55537877473))
- Fixed a language server crash ([`385a6b5a`](https://github.com/helix-editor/helix/commit/385a6b5a1adddfc26e917982641530e1a7c7aa81))
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
- Prompts have an (in-memory) history ([`63e54e30`](https://github.com/helix-editor/helix/commit/63e54e30a74bb0d1d782877ddbbcf95f2817d061))

# 0.3.0 (2021-06-27)

A big shout out to all the contributors! We had 24 contributors in this release.

Another big release. 

Highlights:

- Indentation is now automatically detected from file heuristics. ([#245](https://github.com/helix-editor/helix/pull/245))
- Support for other line endings (CRLF). Significantly improved Windows support. ([#224](https://github.com/helix-editor/helix/pull/224))
- Encodings other than UTF-8 are now supported! ([#228](https://github.com/helix-editor/helix/pull/228))
- Key bindings can now be configured via a `config.toml` file ([#268](https://github.com/helix-editor/helix/pull/268))
- Theme can now be configured and changed at runtime. ([Please feel free to contribute more themes!](https://github.com/helix-editor/helix/tree/master/runtime/themes)) ([#267](https://github.com/helix-editor/helix/pull/267))
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
  ([`f41688d9`](https://github.com/helix-editor/helix/commit/f41688d960ef89c29c4a51c872b8406fb8f81a85))

Fixes:

- The IME popup is now correctly positioned ([#273](https://github.com/helix-editor/helix/pull/273))
- A bunch of bugs regarding `o`/`O` behavior ([#281](https://github.com/helix-editor/helix/pull/281))
- `~` expansion now works in file completion ([#284](https://github.com/helix-editor/helix/pull/284))
- Several UI related overflow crashes ([#318](https://github.com/helix-editor/helix/pull/318))
- Fix a test failure occuring only on `test --release` ([`4f108ab1`](https://github.com/helix-editor/helix/commit/4f108ab1b2197809506bd7305ad903a3525eabfa))
- Prompts now support unicode input ([#295](https://github.com/helix-editor/helix/pull/295))
- Completion documentation no longer overlaps the popup ([#322](https://github.com/helix-editor/helix/pull/322))
- Fix a crash when trying to select `^` ([`9c534614`](https://github.com/helix-editor/helix/commit/9c53461429a3e72e3b1fb87d7ca490e168d7dee2))
- Prompt completions are now paginated ([`39dc09e6`](https://github.com/helix-editor/helix/commit/39dc09e6c4172299bc79de4c1c52288d3f624bd7))
- Goto did not work on Windows ([`503ca112`](https://github.com/helix-editor/helix/commit/503ca112ae57ebdf3ea323baf8940346204b46d2))

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
