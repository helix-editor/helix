# 23.05 (2023-05-18)

23.05 is a smaller release focusing on fixes. There were 88 contributors in this release. Thank you all!

Features:

- Add a config option to exclude declaration from LSP references request ([#6886](https://github.com/helix-editor/helix/pull/6886))
- Enable injecting languages based on their file extension and shebang ([#3970](https://github.com/helix-editor/helix/pull/3970))
- Sort the buffer picker by most recent access ([#2980](https://github.com/helix-editor/helix/pull/2980))
- Perform syntax highlighting in the picker asynchronously ([#7028](https://github.com/helix-editor/helix/pull/7028))

Commands:

- `:update` is now aliased as `:u` ([#6835](https://github.com/helix-editor/helix/pull/6835))
- Add `extend_to_first_nonwhitespace` which acts the same as `goto_first_nonwhitespace` but always extends ([#6837](https://github.com/helix-editor/helix/pull/6837))
- Add `:clear-register` for clearing the given register or all registers ([#5695](https://github.com/helix-editor/helix/pull/5695))
- Add `:write-buffer-close` and `:write-buffer-close!` ([#6947](https://github.com/helix-editor/helix/pull/6947))

Fixes:

- Normalize LSP workspace paths ([#6517](https://github.com/helix-editor/helix/pull/6517))
- Robustly handle invalid LSP ranges ([#6512](https://github.com/helix-editor/helix/pull/6512))
- Fix line number display for LSP goto pickers ([#6559](https://github.com/helix-editor/helix/pull/6559))
- Fix toggling of `soft-wrap.enable` option ([#6656](https://github.com/helix-editor/helix/pull/6656), [58e457a](https://github.com/helix-editor/helix/commit/58e457a), [#6742](https://github.com/helix-editor/helix/pull/6742))
- Handle `workspace/configuration` requests from stopped language servers ([#6693](https://github.com/helix-editor/helix/pull/6693))
- Fix possible crash from opening the jumplist picker ([#6672](https://github.com/helix-editor/helix/pull/6672))
- Fix theme preview returning to current theme on line and word deletions ([#6694](https://github.com/helix-editor/helix/pull/6694))
- Re-run crate build scripts on changes to revision and grammar repositories ([#6743](https://github.com/helix-editor/helix/pull/6743))
- Fix crash on opening from suspended state ([#6764](https://github.com/helix-editor/helix/pull/6764))
- Fix unwrap bug in DAP ([#6786](https://github.com/helix-editor/helix/pull/6786))
- Always build tree-sitter parsers with C++14 and C11 ([#6792](https://github.com/helix-editor/helix/pull/6792), [#6834](https://github.com/helix-editor/helix/pull/6834), [#6845](https://github.com/helix-editor/helix/pull/6845))
- Exit with a non-zero statuscode when tree-sitter parser builds fail ([#6795](https://github.com/helix-editor/helix/pull/6795))
- Flip symbol range in LSP goto commands ([#6794](https://github.com/helix-editor/helix/pull/6794))
- Fix runtime toggling of the `mouse` option ([#6675](https://github.com/helix-editor/helix/pull/6675))
- Fix panic in inlay hint computation when view anchor is out of bounds ([#6883](https://github.com/helix-editor/helix/pull/6883))
- Significantly improve performance of git discovery on slow file systems ([#6890](https://github.com/helix-editor/helix/pull/6890))
- Downgrade gix log level to info ([#6915](https://github.com/helix-editor/helix/pull/6915))
- Conserve BOM and properly support saving UTF16 files ([#6497](https://github.com/helix-editor/helix/pull/6497))
- Correctly handle completion re-request ([#6594](https://github.com/helix-editor/helix/pull/6594))
- Fix offset encoding in LSP `didChange` notifications ([#6921](https://github.com/helix-editor/helix/pull/6921))
- Change `gix` logging level to info ([#6915](https://github.com/helix-editor/helix/pull/6915))
- Improve error message when writes fail because parent directories do not exist ([#7014](https://github.com/helix-editor/helix/pull/7014))
- Replace DAP variables popup instead of pushing more popups ([#7034](https://github.com/helix-editor/helix/pull/7034))
- Disable tree-sitter for files after parsing for 500ms ([#7028](https://github.com/helix-editor/helix/pull/7028))
- Fix crash when deleting with multiple cursors ([#6024](https://github.com/helix-editor/helix/pull/6024))
- Fix selection sliding when deleting forwards in append mode ([#6024](https://github.com/helix-editor/helix/pull/6024))
- Fix completion on paths containing spaces ([#6779](https://github.com/helix-editor/helix/pull/6779))

Themes:

- Style inlay hints in `dracula` theme ([#6515](https://github.com/helix-editor/helix/pull/6515))
- Style inlay hints in `onedark` theme ([#6503](https://github.com/helix-editor/helix/pull/6503))
- Style inlay hints and the soft-wrap indicator in `varua` ([#6568](https://github.com/helix-editor/helix/pull/6568), [#6589](https://github.com/helix-editor/helix/pull/6589))
- Style inlay hints in `emacs` theme ([#6569](https://github.com/helix-editor/helix/pull/6569))
- Update `base16_transparent` and `dark_high_contrast` themes ([#6577](https://github.com/helix-editor/helix/pull/6577))
- Style inlay hints for `mellow` and `rasmus` themes ([#6583](https://github.com/helix-editor/helix/pull/6583))
- Dim pane divider for `base16_transparent` theme ([#6534](https://github.com/helix-editor/helix/pull/6534))
- Style inlay hints in `zenburn` theme ([#6593](https://github.com/helix-editor/helix/pull/6593))
- Style inlay hints in `boo_berry` theme ([#6625](https://github.com/helix-editor/helix/pull/6625))
- Add `ferra` theme ([#6619](https://github.com/helix-editor/helix/pull/6619), [#6776](https://github.com/helix-editor/helix/pull/6776))
- Style inlay hints in `nightfox` theme ([#6655](https://github.com/helix-editor/helix/pull/6655))
- Fix `ayu` theme family markup code block background ([#6538](https://github.com/helix-editor/helix/pull/6538))
- Improve whitespace and search match colors in `rose_pine` theme ([#6679](https://github.com/helix-editor/helix/pull/6679))
- Highlight selected items in `base16_transparent` theme ([#6716](https://github.com/helix-editor/helix/pull/6716))
- Adjust everforest to resemble original more closely ([#5866](https://github.com/helix-editor/helix/pull/5866))
- Refactor `dracula` theme ([#6552](https://github.com/helix-editor/helix/pull/6552), [#6767](https://github.com/helix-editor/helix/pull/6767), [#6855](https://github.com/helix-editor/helix/pull/6855), [#6987](https://github.com/helix-editor/helix/pull/6987))
- Style inlay hints in `darcula` theme ([#6732](https://github.com/helix-editor/helix/pull/6732))
- Style inlay hints in `kanagawa` theme ([#6773](https://github.com/helix-editor/helix/pull/6773))
- Improve `ayu_dark` theme ([#6622](https://github.com/helix-editor/helix/pull/6622))
- Refactor `noctis` theme multiple cursor highlighting ([96720e7](https://github.com/helix-editor/helix/commit/96720e7))
- Refactor `noctis` theme whitespace rendering and indent guides ([f2ccc03](https://github.com/helix-editor/helix/commit/f2ccc03))
- Add `amberwood` theme ([#6924](https://github.com/helix-editor/helix/pull/6924))
- Update `nightfox` theme ([#7061](https://github.com/helix-editor/helix/pull/7061))

Language support:

- R language server: use the `--no-echo` flag to silence output ([#6570](https://github.com/helix-editor/helix/pull/6570))
- Recognize CUDA files as C++ ([#6521](https://github.com/helix-editor/helix/pull/6521))
- Add support for Hurl ([#6450](https://github.com/helix-editor/helix/pull/6450))
- Add textobject queries for Julia ([#6588](https://github.com/helix-editor/helix/pull/6588))
- Update Ruby highlight queries ([#6587](https://github.com/helix-editor/helix/pull/6587))
- Add xsd to XML file-types ([#6631](https://github.com/helix-editor/helix/pull/6631))
- Support Robot Framework ([#6611](https://github.com/helix-editor/helix/pull/6611))
- Update Gleam tree-sitter parser ([#6641](https://github.com/helix-editor/helix/pull/6641))
- Update git-commit tree-sitter parser ([#6692](https://github.com/helix-editor/helix/pull/6692))
- Update Haskell tree-sitter parser ([#6317](https://github.com/helix-editor/helix/pull/6317))
- Add injection queries for Haskell quasiquotes ([#6474](https://github.com/helix-editor/helix/pull/6474))
- Highlight C/C++ escape sequences ([#6724](https://github.com/helix-editor/helix/pull/6724))
- Support Markdoc ([#6432](https://github.com/helix-editor/helix/pull/6432))
- Support OpenCL ([#6473](https://github.com/helix-editor/helix/pull/6473))
- Support DTD ([#6644](https://github.com/helix-editor/helix/pull/6644))
- Fix constant highlighting in Python queries ([#6751](https://github.com/helix-editor/helix/pull/6751))
- Support Just ([#6453](https://github.com/helix-editor/helix/pull/6453))
- Fix Go locals query for `var_spec` identifiers ([#6763](https://github.com/helix-editor/helix/pull/6763))
- Update Markdown tree-sitter parser ([#6785](https://github.com/helix-editor/helix/pull/6785))
- Fix Haskell workspace root for cabal projects ([#6828](https://github.com/helix-editor/helix/pull/6828))
- Avoid extra indentation in Go switches ([#6817](https://github.com/helix-editor/helix/pull/6817))
- Fix Go workspace roots ([#6884](https://github.com/helix-editor/helix/pull/6884))
- Set PerlNavigator as the default Perl language server ([#6860](https://github.com/helix-editor/helix/pull/6860))
- Highlight more sqlx macros in Rust ([#6793](https://github.com/helix-editor/helix/pull/6793))
- Switch Odin tree-sitter grammar ([#6766](https://github.com/helix-editor/helix/pull/6766))
- Recognize `poetry.lock` as TOML ([#6928](https://github.com/helix-editor/helix/pull/6928))
- Recognize Jupyter notebooks as JSON ([#6927](https://github.com/helix-editor/helix/pull/6927))
- Add language server configuration for Crystal ([#6948](https://github.com/helix-editor/helix/pull/6948))
- Add `build.gradle.kts` to Java and Scala roots ([#6970](https://github.com/helix-editor/helix/pull/6970))
- Recognize `sty` and `cls` files as latex ([#6986](https://github.com/helix-editor/helix/pull/6986))
- Update Dockerfile tree-sitter grammar ([#6895](https://github.com/helix-editor/helix/pull/6895))
- Add comment injections for Odin ([#7027](https://github.com/helix-editor/helix/pull/7027))
- Recognize `gml` as XML ([#7055](https://github.com/helix-editor/helix/pull/7055))
- Recognize `geojson` as JSON ([#7054](https://github.com/helix-editor/helix/pull/7054))

Packaging:

- Update the Nix flake dependencies, remove a deprecated option ([#6546](https://github.com/helix-editor/helix/pull/6546))
- Fix and re-enable aarch64-macos release binary builds ([#6504](https://github.com/helix-editor/helix/pull/6504))
- The git dependency on `tree-sitter` has been replaced with a regular crates.io dependency ([#6608](https://github.com/helix-editor/helix/pull/6608))

# 23.03 (2023-03-31)

23.03 brings some long-awaited and exciting features. Thank you to everyone involved! This release saw changes from 102 contributors.

For the full log, check out the [git log](https://github.com/helix-editor/helix/compare/22.12..23.03).
Also check out the [release notes](https://helix-editor.com/news/release-23-03-highlights/) for more commentary on larger features.

Breaking changes:

- Select diagnostic range in `goto_*_diag` commands ([#4713](https://github.com/helix-editor/helix/pull/4713), [#5164](https://github.com/helix-editor/helix/pull/5164), [#6193](https://github.com/helix-editor/helix/pull/6193))
- Remove jump behavior from `increment`/`decrement` ([#4123](https://github.com/helix-editor/helix/pull/4123), [#5929](https://github.com/helix-editor/helix/pull/5929))
- Select change range in `goto_*_change` commands ([#5206](https://github.com/helix-editor/helix/pull/5206))
- Split file modification indicator from filename statusline elements ([#4731](https://github.com/helix-editor/helix/pull/4731), [#6036](https://github.com/helix-editor/helix/pull/6036))
- Jump to symbol ranges in LSP goto commands ([#5986](https://github.com/helix-editor/helix/pull/5986))
- Workspace detection now stops at the first `.helix/` directory (merging multiple `.helix/languages.toml` configurations is no longer supported) ([#5748](https://github.com/helix-editor/helix/pull/5748))

Features:

- Dynamic workspace symbol picker ([#5055](https://github.com/helix-editor/helix/pull/5055))
- Soft-wrap ([#5420](https://github.com/helix-editor/helix/pull/5420), [#5786](https://github.com/helix-editor/helix/pull/5786), [#5893](https://github.com/helix-editor/helix/pull/5893), [#6142](https://github.com/helix-editor/helix/pull/6142), [#6440](https://github.com/helix-editor/helix/pull/6440))
- Initial support for LSP snippet completions ([#5864](https://github.com/helix-editor/helix/pull/5864), [b1f7528](https://github.com/helix-editor/helix/commit/b1f7528), [#6263](https://github.com/helix-editor/helix/pull/6263), [bbf4800](https://github.com/helix-editor/helix/commit/bbf4800), [90348b8](https://github.com/helix-editor/helix/commit/90348b8), [f87299f](https://github.com/helix-editor/helix/commit/f87299f), [#6371](https://github.com/helix-editor/helix/pull/6371), [9fe3adc](https://github.com/helix-editor/helix/commit/9fe3adc))
- Add a statusline element for showing the current version control HEAD ([#5682](https://github.com/helix-editor/helix/pull/5682))
- Display LSP type hints ([#5420](https://github.com/helix-editor/helix/pull/5420), [#5934](https://github.com/helix-editor/helix/pull/5934), [#6312](https://github.com/helix-editor/helix/pull/6312))
- Enable the Kitty keyboard protocol on terminals with support ([#4939](https://github.com/helix-editor/helix/pull/4939), [#6170](https://github.com/helix-editor/helix/pull/6170), [#6194](https://github.com/helix-editor/helix/pull/6194), [#6438](https://github.com/helix-editor/helix/pull/6438))
- Add a statusline element for the basename of the current file ([#5318](https://github.com/helix-editor/helix/pull/5318))
- Add substring matching syntax for the picker ([#5658](https://github.com/helix-editor/helix/pull/5658))
- Support LSP `textDocument/prepareRename` ([#6103](https://github.com/helix-editor/helix/pull/6103))
- Allow multiple runtime directories with priorities ([#5411](https://github.com/helix-editor/helix/pull/5411))
- Allow configuring whether to insert or replace completions ([#5728](https://github.com/helix-editor/helix/pull/5728))
- Allow per-workspace config file `.helix/config.toml` ([#5748](https://github.com/helix-editor/helix/pull/5748))
- Add `workspace-lsp-roots` config option to support multiple LSP roots for use with monorepos ([#5748](https://github.com/helix-editor/helix/pull/5748))

Commands:

- `:pipe-to` which pipes selections into a shell command and ignores output ([#4931](https://github.com/helix-editor/helix/pull/4931))
- `merge_consecutive_selections` (`A-_`) combines all consecutive selections ([#5047](https://github.com/helix-editor/helix/pull/5047))
- `rotate_view_reverse` which focuses the previous view ([#5356](https://github.com/helix-editor/helix/pull/5356))
- `goto_declaration` (`gD`, requires LSP) which jumps to a symbol's declaration ([#5646](https://github.com/helix-editor/helix/pull/5646))
- `file_picker_in_current_buffer_directory` ([#4666](https://github.com/helix-editor/helix/pull/4666))
- `:character-info` which shows information about the character under the cursor ([#4000](https://github.com/helix-editor/helix/pull/4000))
- `:toggle-option` for toggling config options at runtime ([#4085](https://github.com/helix-editor/helix/pull/4085))
- `dap_restart` for restarting a debug session in DAP ([#5651](https://github.com/helix-editor/helix/pull/5651))
- `:lsp-stop` to stop the language server of the current buffer ([#5964](https://github.com/helix-editor/helix/pull/5964))
- `:reset-diff-change` for resetting a diff hunk to its original text ([#4974](https://github.com/helix-editor/helix/pull/4974))
- `:config-open-workspace` for opening the config file local to the current workspace ([#5748](https://github.com/helix-editor/helix/pull/5748))

Usability improvements:

- Remove empty detail section in completion menu when LSP doesn't send details ([#4902](https://github.com/helix-editor/helix/pull/4902))
- Pass client information on LSP initialization ([#4904](https://github.com/helix-editor/helix/pull/4904))
- Allow specifying environment variables for language servers in language config ([#4004](https://github.com/helix-editor/helix/pull/4004))
- Allow detached git worktrees to be recognized as root paths ([#5097](https://github.com/helix-editor/helix/pull/5097))
- Improve error message handling for theme loading failures ([#5073](https://github.com/helix-editor/helix/pull/5073))
- Print the names of binaries required for LSP/DAP in health-check ([#5195](https://github.com/helix-editor/helix/pull/5195))
- Improve sorting in the picker in cases of ties ([#5169](https://github.com/helix-editor/helix/pull/5169))
- Add theming for prompt suggestions ([#5104](https://github.com/helix-editor/helix/pull/5104))
- Open a file picker when using `:open` on directories ([#2707](https://github.com/helix-editor/helix/pull/2707), [#5278](https://github.com/helix-editor/helix/pull/5278))
- Reload language config with `:config-reload` ([#5239](https://github.com/helix-editor/helix/pull/5239), [#5381](https://github.com/helix-editor/helix/pull/5381), [#5431](https://github.com/helix-editor/helix/pull/5431))
- Improve indent queries for python when the tree is errored ([#5332](https://github.com/helix-editor/helix/pull/5332))
- Picker: Open files without closing the picker with `A-ret` ([#4435](https://github.com/helix-editor/helix/pull/4435))
- Allow theming cursors by primary/secondary and by mode ([#5130](https://github.com/helix-editor/helix/pull/5130))
- Allow configuration of the minimum width for the line-numbers gutter ([#4724](https://github.com/helix-editor/helix/pull/4724), [#5696](https://github.com/helix-editor/helix/pull/5696))
- Use filename completer for `:run-shell-command` command ([#5729](https://github.com/helix-editor/helix/pull/5729))
- Surround with line-endings with `ms<ret>` ([#4571](https://github.com/helix-editor/helix/pull/4571))
- Hide duplicate symlinks in file pickers ([#5658](https://github.com/helix-editor/helix/pull/5658))
- Tabulate buffer picker contents ([#5777](https://github.com/helix-editor/helix/pull/5777))
- Add an option to disable LSP ([#4425](https://github.com/helix-editor/helix/pull/4425))
- Short-circuit tree-sitter and word object motions ([#5851](https://github.com/helix-editor/helix/pull/5851))
- Add exit code to failed command message ([#5898](https://github.com/helix-editor/helix/pull/5898))
- Make `m` textobject look for pairs enclosing selections ([#3344](https://github.com/helix-editor/helix/pull/3344))
- Negotiate LSP position encoding ([#5894](https://github.com/helix-editor/helix/pull/5894), [a48d1a4](https://github.com/helix-editor/helix/commit/a48d1a4))
- Display deprecated LSP completions with strikethrough ([#5932](https://github.com/helix-editor/helix/pull/5932))
- Add JSONRPC request ID to failed LSP/DAP request log messages ([#6010](https://github.com/helix-editor/helix/pull/6010), [#6018](https://github.com/helix-editor/helix/pull/6018))
- Ignore case when filtering LSP completions ([#6008](https://github.com/helix-editor/helix/pull/6008))
- Show current language when no arguments are passed to `:set-language` ([#5895](https://github.com/helix-editor/helix/pull/5895))
- Refactor and rewrite all book documentation ([#5534](https://github.com/helix-editor/helix/pull/5534))
- Separate diagnostic picker message and code ([#6095](https://github.com/helix-editor/helix/pull/6095))
- Add a config option to bypass undercurl detection ([#6253](https://github.com/helix-editor/helix/pull/6253))
- Only complete appropriate arguments for typed commands ([#5966](https://github.com/helix-editor/helix/pull/5966))
- Discard outdated LSP diagnostics ([3c9d5d0](https://github.com/helix-editor/helix/commit/3c9d5d0))
- Discard outdated LSP workspace edits ([b6a4927](https://github.com/helix-editor/helix/commit/b6a4927))
- Run shell commands asynchronously ([#6373](https://github.com/helix-editor/helix/pull/6373))
- Show diagnostic codes in LSP diagnostic messages ([#6378](https://github.com/helix-editor/helix/pull/6378))
- Highlight the current line in a DAP debug session ([#5957](https://github.com/helix-editor/helix/pull/5957))
- Hide signature help if it overlaps with the completion menu ([#5523](https://github.com/helix-editor/helix/pull/5523), [7a69c40](https://github.com/helix-editor/helix/commit/7a69c40))

Fixes:

- Fix behavior of `auto-completion` flag for completion-on-trigger ([#5042](https://github.com/helix-editor/helix/pull/5042))
- Reset editor mode when changing buffers ([#5072](https://github.com/helix-editor/helix/pull/5072))
- Respect scrolloff settings in mouse movements ([#5255](https://github.com/helix-editor/helix/pull/5255))
- Avoid trailing `s` when only one file is opened ([#5189](https://github.com/helix-editor/helix/pull/5189))
- Fix erroneous indent between closers of auto-pairs ([#5330](https://github.com/helix-editor/helix/pull/5330))
- Expand `~` when parsing file paths in `:open` ([#5329](https://github.com/helix-editor/helix/pull/5329))
- Fix theme inheritance for default themes ([#5218](https://github.com/helix-editor/helix/pull/5218))
- Fix `extend_line` with a count when the current line(s) are selected ([#5288](https://github.com/helix-editor/helix/pull/5288))
- Prompt: Fix autocompletion for paths containing periods ([#5175](https://github.com/helix-editor/helix/pull/5175))
- Skip serializing JSONRPC params if params is null ([#5471](https://github.com/helix-editor/helix/pull/5471))
- Fix interaction with the `xclip` clipboard provider ([#5426](https://github.com/helix-editor/helix/pull/5426))
- Fix undo/redo execution from the command palette ([#5294](https://github.com/helix-editor/helix/pull/5294))
- Fix highlighting of non-block cursors ([#5575](https://github.com/helix-editor/helix/pull/5575))
- Fix panic when nooping in `join_selections` and `join_selections_space` ([#5423](https://github.com/helix-editor/helix/pull/5423))
- Fix selecting a changed file in global search ([#5639](https://github.com/helix-editor/helix/pull/5639))
- Fix initial syntax highlight layer sort order ([#5196](https://github.com/helix-editor/helix/pull/5196))
- Fix UTF-8 length handling for shellwords ([#5738](https://github.com/helix-editor/helix/pull/5738))
- Remove C-j and C-k bindings from the completion menu ([#5070](https://github.com/helix-editor/helix/pull/5070))
- Always commit to history when pasting ([#5790](https://github.com/helix-editor/helix/pull/5790))
- Properly handle LSP position encoding ([#5711](https://github.com/helix-editor/helix/pull/5711))
- Fix infinite loop in `copy_selection_on_prev_line` ([#5888](https://github.com/helix-editor/helix/pull/5888))
- Fix completion popup positioning ([#5842](https://github.com/helix-editor/helix/pull/5842))
- Fix a panic when uncommenting a line with only a comment token ([#5933](https://github.com/helix-editor/helix/pull/5933))
- Fix panic in `goto_window_center` at EOF ([#5987](https://github.com/helix-editor/helix/pull/5987))
- Ignore invalid file URIs sent by a language server ([#6000](https://github.com/helix-editor/helix/pull/6000))
- Decode LSP URIs for the workspace diagnostics picker ([#6016](https://github.com/helix-editor/helix/pull/6016))
- Fix incorrect usages of `tab_width` with `indent_width` ([#5918](https://github.com/helix-editor/helix/pull/5918))
- DAP: Send Disconnect if the Terminated event is received ([#5532](https://github.com/helix-editor/helix/pull/5532))
- DAP: Validate key and index exist when requesting variables ([#5628](https://github.com/helix-editor/helix/pull/5628))
- Check LSP renaming support before prompting for rename text ([#6257](https://github.com/helix-editor/helix/pull/6257))
- Fix indent guide rendering ([#6136](https://github.com/helix-editor/helix/pull/6136))
- Fix division by zero panic ([#6155](https://github.com/helix-editor/helix/pull/6155))
- Fix lacking space panic ([#6109](https://github.com/helix-editor/helix/pull/6109))
- Send error replies for malformed and unhandled LSP requests ([#6058](https://github.com/helix-editor/helix/pull/6058))
- Fix table column calculations for dynamic pickers ([#5920](https://github.com/helix-editor/helix/pull/5920))
- Skip adding jumplist entries for `:<n>` line number previews ([#5751](https://github.com/helix-editor/helix/pull/5751))
- Fix completion race conditions ([#6173](https://github.com/helix-editor/helix/pull/6173))
- Fix `shrink_selection` with multiple cursors ([#6093](https://github.com/helix-editor/helix/pull/6093))
- Fix indentation calculation for lines with mixed tabs/spaces ([#6278](https://github.com/helix-editor/helix/pull/6278))
- No-op `client/registerCapability` LSP requests ([#6258](https://github.com/helix-editor/helix/pull/6258))
- Send the STOP signal to all processes in the process group ([#3546](https://github.com/helix-editor/helix/pull/3546))
- Fix workspace edit client capabilities declaration ([7bf168d](https://github.com/helix-editor/helix/commit/7bf168d))
- Fix highlighting in picker results with multiple columns ([#6333](https://github.com/helix-editor/helix/pull/6333))
- Canonicalize paths before stripping the current dir as a prefix ([#6290](https://github.com/helix-editor/helix/pull/6290))
- Fix truncation behavior for long path names in the file picker ([#6410](https://github.com/helix-editor/helix/pull/6410), [67783dd](https://github.com/helix-editor/helix/commit/67783dd))
- Fix theme reloading behavior in `:config-reload` ([ab819d8](https://github.com/helix-editor/helix/commit/ab819d8))

Themes:

- Update `serika` ([#5038](https://github.com/helix-editor/helix/pull/5038), [#6344](https://github.com/helix-editor/helix/pull/6344))
- Update `flatwhite` ([#5036](https://github.com/helix-editor/helix/pull/5036), [#6323](https://github.com/helix-editor/helix/pull/6323))
- Update `autumn` ([#5051](https://github.com/helix-editor/helix/pull/5051), [#5397](https://github.com/helix-editor/helix/pull/5397), [#6280](https://github.com/helix-editor/helix/pull/6280), [#6316](https://github.com/helix-editor/helix/pull/6316))
- Update `acme` ([#5019](https://github.com/helix-editor/helix/pull/5019), [#5486](https://github.com/helix-editor/helix/pull/5486), [#5488](https://github.com/helix-editor/helix/pull/5488))
- Update `gruvbox` themes ([#5066](https://github.com/helix-editor/helix/pull/5066), [#5333](https://github.com/helix-editor/helix/pull/5333), [#5540](https://github.com/helix-editor/helix/pull/5540), [#6285](https://github.com/helix-editor/helix/pull/6285), [#6295](https://github.com/helix-editor/helix/pull/6295))
- Update `base16_transparent` ([#5105](https://github.com/helix-editor/helix/pull/5105))
- Update `dark_high_contrast` ([#5105](https://github.com/helix-editor/helix/pull/5105))
- Update `dracula` ([#5236](https://github.com/helix-editor/helix/pull/5236), [#5627](https://github.com/helix-editor/helix/pull/5627), [#6414](https://github.com/helix-editor/helix/pull/6414))
- Update `monokai_pro_spectrum` ([#5250](https://github.com/helix-editor/helix/pull/5250), [#5602](https://github.com/helix-editor/helix/pull/5602))
- Update `rose_pine` ([#5267](https://github.com/helix-editor/helix/pull/5267), [#5489](https://github.com/helix-editor/helix/pull/5489), [#6384](https://github.com/helix-editor/helix/pull/6384))
- Update `kanagawa` ([#5273](https://github.com/helix-editor/helix/pull/5273), [#5571](https://github.com/helix-editor/helix/pull/5571), [#6085](https://github.com/helix-editor/helix/pull/6085))
- Update `emacs` ([#5334](https://github.com/helix-editor/helix/pull/5334))
- Add `github` themes ([#5353](https://github.com/helix-editor/helix/pull/5353), [efeec12](https://github.com/helix-editor/helix/commit/efeec12))
    - Dark themes: `github_dark`, `github_dark_colorblind`, `github_dark_dimmed`, `github_dark_high_contrast`, `github_dark_tritanopia`
    - Light themes: `github_light`, `github_light_colorblind`, `github_light_dimmed`, `github_light_high_contrast`, `github_light_tritanopia`
- Update `solarized` variants ([#5445](https://github.com/helix-editor/helix/pull/5445), [#6327](https://github.com/helix-editor/helix/pull/6327))
- Update `catppuccin` variants ([#5404](https://github.com/helix-editor/helix/pull/5404), [#6107](https://github.com/helix-editor/helix/pull/6107), [#6269](https://github.com/helix-editor/helix/pull/6269), [#6464](https://github.com/helix-editor/helix/pull/6464))
- Use curly underlines in built-in themes ([#5419](https://github.com/helix-editor/helix/pull/5419))
- Update `zenburn` ([#5573](https://github.com/helix-editor/helix/pull/5573))
- Rewrite `snazzy` ([#3971](https://github.com/helix-editor/helix/pull/3971))
- Add `monokai_aqua` ([#5578](https://github.com/helix-editor/helix/pull/5578))
- Add `markup.strikethrough` to existing themes ([#5619](https://github.com/helix-editor/helix/pull/5619))
- Update `sonokai` ([#5440](https://github.com/helix-editor/helix/pull/5440))
- Update `onedark` ([#5755](https://github.com/helix-editor/helix/pull/5755))
- Add `ayu_evolve` ([#5638](https://github.com/helix-editor/helix/pull/5638), [#6028](https://github.com/helix-editor/helix/pull/6028), [#6225](https://github.com/helix-editor/helix/pull/6225))
- Add `jellybeans` ([#5719](https://github.com/helix-editor/helix/pull/5719))
- Update `fleet_dark` ([#5605](https://github.com/helix-editor/helix/pull/5605), [#6266](https://github.com/helix-editor/helix/pull/6266), [#6324](https://github.com/helix-editor/helix/pull/6324), [#6375](https://github.com/helix-editor/helix/pull/6375))
- Add `darcula-solid` ([#5778](https://github.com/helix-editor/helix/pull/5778))
- Remove text background from monokai themes ([#6009](https://github.com/helix-editor/helix/pull/6009))
- Update `pop_dark` ([#5992](https://github.com/helix-editor/helix/pull/5992), [#6208](https://github.com/helix-editor/helix/pull/6208), [#6227](https://github.com/helix-editor/helix/pull/6227), [#6292](https://github.com/helix-editor/helix/pull/6292))
- Add `everblush` ([#6086](https://github.com/helix-editor/helix/pull/6086))
- Add `adwaita-dark` ([#6042](https://github.com/helix-editor/helix/pull/6042), [#6342](https://github.com/helix-editor/helix/pull/6342))
- Update `papercolor` ([#6162](https://github.com/helix-editor/helix/pull/6162))
- Update `onelight` ([#6192](https://github.com/helix-editor/helix/pull/6192), [#6276](https://github.com/helix-editor/helix/pull/6276))
- Add `molokai` ([#6260](https://github.com/helix-editor/helix/pull/6260))
- Update `ayu` variants ([#6329](https://github.com/helix-editor/helix/pull/6329))
- Update `tokyonight` variants ([#6349](https://github.com/helix-editor/helix/pull/6349))
- Update `nord` variants ([#6376](https://github.com/helix-editor/helix/pull/6376))

New languages:

- BibTeX ([#5064](https://github.com/helix-editor/helix/pull/5064))
- Mermaid.js ([#5147](https://github.com/helix-editor/helix/pull/5147))
- Crystal ([#4993](https://github.com/helix-editor/helix/pull/4993), [#5205](https://github.com/helix-editor/helix/pull/5205))
- MATLAB/Octave ([#5192](https://github.com/helix-editor/helix/pull/5192))
- `tfvars` (uses HCL) ([#5396](https://github.com/helix-editor/helix/pull/5396))
- Ponylang ([#5416](https://github.com/helix-editor/helix/pull/5416))
- DHall ([1f6809c](https://github.com/helix-editor/helix/commit/1f6809c))
- Sagemath ([#5649](https://github.com/helix-editor/helix/pull/5649))
- MSBuild ([#5793](https://github.com/helix-editor/helix/pull/5793))
- pem ([#5797](https://github.com/helix-editor/helix/pull/5797))
- passwd ([#4959](https://github.com/helix-editor/helix/pull/4959))
- hosts ([#4950](https://github.com/helix-editor/helix/pull/4950), [#5914](https://github.com/helix-editor/helix/pull/5914))
- uxntal ([#6047](https://github.com/helix-editor/helix/pull/6047))
- Yuck ([#6064](https://github.com/helix-editor/helix/pull/6064), [#6242](https://github.com/helix-editor/helix/pull/6242))
- GNU gettext PO ([#5996](https://github.com/helix-editor/helix/pull/5996))
- Sway ([#6023](https://github.com/helix-editor/helix/pull/6023))
- NASM ([#6068](https://github.com/helix-editor/helix/pull/6068))
- PRQL ([#6126](https://github.com/helix-editor/helix/pull/6126))
- reStructuredText ([#6180](https://github.com/helix-editor/helix/pull/6180))
- Smithy ([#6370](https://github.com/helix-editor/helix/pull/6370))
- VHDL ([#5826](https://github.com/helix-editor/helix/pull/5826))
- Rego (OpenPolicy Agent) ([#6415](https://github.com/helix-editor/helix/pull/6415))
- Nim ([#6123](https://github.com/helix-editor/helix/pull/6123))

Updated languages and queries:

- Use diff syntax for patch files ([#5085](https://github.com/helix-editor/helix/pull/5085))
- Add Haskell textobjects ([#5061](https://github.com/helix-editor/helix/pull/5061))
- Fix commonlisp configuration ([#5091](https://github.com/helix-editor/helix/pull/5091))
- Update Scheme ([bae890d](https://github.com/helix-editor/helix/commit/bae890d))
- Add indent queries for Bash ([#5149](https://github.com/helix-editor/helix/pull/5149))
- Recognize `c++` as a C++ extension ([#5183](https://github.com/helix-editor/helix/pull/5183))
- Enable HTTP server in `metals` (Scala) config ([#5551](https://github.com/helix-editor/helix/pull/5551))
- Change V-lang language server to `v ls` from `vls` ([#5677](https://github.com/helix-editor/helix/pull/5677))
- Inject comment grammar into Nix ([#5208](https://github.com/helix-editor/helix/pull/5208))
- Update Rust highlights ([#5238](https://github.com/helix-editor/helix/pull/5238), [#5349](https://github.com/helix-editor/helix/pull/5349))
- Fix HTML injection within Markdown ([#5265](https://github.com/helix-editor/helix/pull/5265))
- Fix comment token for godot ([#5276](https://github.com/helix-editor/helix/pull/5276))
- Expand injections for Vue ([#5268](https://github.com/helix-editor/helix/pull/5268))
- Add `.bash_aliases` as a Bash file-type ([#5347](https://github.com/helix-editor/helix/pull/5347))
- Fix comment token for sshclientconfig ([#5351](https://github.com/helix-editor/helix/pull/5351))
- Update Prisma ([#5417](https://github.com/helix-editor/helix/pull/5417))
- Update C++ ([#5457](https://github.com/helix-editor/helix/pull/5457))
- Add more file-types for Python ([#5593](https://github.com/helix-editor/helix/pull/5593))
- Update tree-sitter-scala ([#5576](https://github.com/helix-editor/helix/pull/5576))
- Add an injection regex for Lua ([#5606](https://github.com/helix-editor/helix/pull/5606))
- Add `build.gradle` to java roots configuration ([#5641](https://github.com/helix-editor/helix/pull/5641))
- Add Hub PR files to markdown file-types ([#5634](https://github.com/helix-editor/helix/pull/5634))
- Add an external formatter configuration for Cue ([#5679](https://github.com/helix-editor/helix/pull/5679))
- Add injections for builders and writers to Nix ([#5629](https://github.com/helix-editor/helix/pull/5629))
- Update tree-sitter-xml to fix whitespace parsing ([#5685](https://github.com/helix-editor/helix/pull/5685))
- Add `Justfile` to the make file-types configuration ([#5687](https://github.com/helix-editor/helix/pull/5687))
- Update tree-sitter-sql and highlight queries ([#5683](https://github.com/helix-editor/helix/pull/5683), [#5772](https://github.com/helix-editor/helix/pull/5772))
- Use the bash grammar and queries for env language ([#5720](https://github.com/helix-editor/helix/pull/5720))
- Add podspec files to ruby file-types ([#5811](https://github.com/helix-editor/helix/pull/5811))
- Recognize `.C` and `.H` file-types as C++ ([#5808](https://github.com/helix-editor/helix/pull/5808))
- Recognize plist and mobileconfig files as XML ([#5863](https://github.com/helix-editor/helix/pull/5863))
- Fix `select` indentation in Go ([#5713](https://github.com/helix-editor/helix/pull/5713))
- Check for external file modifications when writing ([#5805](https://github.com/helix-editor/helix/pull/5805))
- Recognize containerfiles as dockerfile syntax ([#5873](https://github.com/helix-editor/helix/pull/5873))
- Update godot grammar and queries ([#5944](https://github.com/helix-editor/helix/pull/5944), [#6186](https://github.com/helix-editor/helix/pull/6186))
- Improve DHall highlights ([#5959](https://github.com/helix-editor/helix/pull/5959))
- Recognize `.env.dist` and `source.env` as env language ([#6003](https://github.com/helix-editor/helix/pull/6003))
- Update tree-sitter-git-rebase ([#6030](https://github.com/helix-editor/helix/pull/6030), [#6094](https://github.com/helix-editor/helix/pull/6094))
- Improve SQL highlights ([#6041](https://github.com/helix-editor/helix/pull/6041))
- Improve markdown highlights and inject LaTeX ([#6100](https://github.com/helix-editor/helix/pull/6100))
- Add textobject queries for Elm ([#6084](https://github.com/helix-editor/helix/pull/6084))
- Recognize graphql schema file type ([#6159](https://github.com/helix-editor/helix/pull/6159))
- Improve highlighting in comments ([#6143](https://github.com/helix-editor/helix/pull/6143))
- Improve highlighting for JavaScript/TypeScript/ECMAScript languages ([#6205](https://github.com/helix-editor/helix/pull/6205))
- Improve PHP highlights ([#6203](https://github.com/helix-editor/helix/pull/6203), [#6250](https://github.com/helix-editor/helix/pull/6250), [#6299](https://github.com/helix-editor/helix/pull/6299))
- Improve Go highlights ([#6204](https://github.com/helix-editor/helix/pull/6204))
- Highlight unchecked sqlx functions as SQL in Rust ([#6256](https://github.com/helix-editor/helix/pull/6256))
- Improve Erlang highlights ([cdd6c8d](https://github.com/helix-editor/helix/commit/cdd6c8d))
- Improve Nix highlights ([fb4d703](https://github.com/helix-editor/helix/commit/fb4d703))
- Improve gdscript highlights ([#6311](https://github.com/helix-editor/helix/pull/6311))
- Improve Vlang highlights ([#6279](https://github.com/helix-editor/helix/pull/6279))
- Improve Makefile highlights ([#6339](https://github.com/helix-editor/helix/pull/6339))
- Remove auto-pair for `'` in OCaml ([#6381](https://github.com/helix-editor/helix/pull/6381))
- Fix indents in switch statements in ECMA languages ([#6369](https://github.com/helix-editor/helix/pull/6369))
- Recognize xlb and storyboard file-types as XML ([#6407](https://github.com/helix-editor/helix/pull/6407))
- Recognize cts and mts file-types as TypeScript ([#6424](https://github.com/helix-editor/helix/pull/6424))
- Recognize SVG file-type as XML ([#6431](https://github.com/helix-editor/helix/pull/6431))
- Add theme scopes for (un)checked list item markup scopes ([#6434](https://github.com/helix-editor/helix/pull/6434))
- Update git commit grammar and add the comment textobject ([#6439](https://github.com/helix-editor/helix/pull/6439), [#6493](https://github.com/helix-editor/helix/pull/6493))
- Recognize ARB file-type as JSON ([#6452](https://github.com/helix-editor/helix/pull/6452))
- Inject markdown into markdown strings in Julia ([#6489](https://github.com/helix-editor/helix/pull/6489))

Packaging:

- Fix Nix flake devShell for darwin hosts ([#5368](https://github.com/helix-editor/helix/pull/5368))
- Add Appstream metadata file to `contrib/` ([#5643](https://github.com/helix-editor/helix/pull/5643))
- Increase the MSRV to 1.65 ([#5570](https://github.com/helix-editor/helix/pull/5570), [#6185](https://github.com/helix-editor/helix/pull/6185))
- Expose the Nix flake's `wrapper` ([#5994](https://github.com/helix-editor/helix/pull/5994))

# 22.12 (2022-12-06)

This is a great big release filled with changes from a 99 contributors. A big _thank you_ to you all!

As usual, the following is a summary of each of the changes since the last release.
For the full log, check out the [git log](https://github.com/helix-editor/helix/compare/22.08.1..22.12).

Breaking changes:

- Remove readline-like navigation bindings from the default insert mode keymap ([e12690e](https://github.com/helix-editor/helix/commit/e12690e), [#3811](https://github.com/helix-editor/helix/pull/3811), [#3827](https://github.com/helix-editor/helix/pull/3827), [#3915](https://github.com/helix-editor/helix/pull/3915), [#4088](https://github.com/helix-editor/helix/pull/4088))
- Rename `append_to_line` as `insert_at_line_end` and `prepend_to_line` as `insert_at_line_start` ([#3753](https://github.com/helix-editor/helix/pull/3753))
- Swap diagnostic picker and debug mode bindings in the space keymap ([#4229](https://github.com/helix-editor/helix/pull/4229))
- Select newly inserted text on paste or from shell commands ([#4458](https://github.com/helix-editor/helix/pull/4458), [#4608](https://github.com/helix-editor/helix/pull/4608), [#4619](https://github.com/helix-editor/helix/pull/4619), [#4824](https://github.com/helix-editor/helix/pull/4824))
- Select newly inserted surrounding characters on `ms<char>` ([#4752](https://github.com/helix-editor/helix/pull/4752))
- Exit select-mode after executing `replace_*` commands ([#4554](https://github.com/helix-editor/helix/pull/4554))
- Exit select-mode after executing surround commands ([#4858](https://github.com/helix-editor/helix/pull/4858))
- Change tree-sitter text-object keys ([#3782](https://github.com/helix-editor/helix/pull/3782))
- Rename `fleetish` theme to `fleet_dark` ([#4997](https://github.com/helix-editor/helix/pull/4997))

Features:

- Bufferline ([#2759](https://github.com/helix-editor/helix/pull/2759))
- Support underline styles and colors ([#4061](https://github.com/helix-editor/helix/pull/4061), [98c121c](https://github.com/helix-editor/helix/commit/98c121c))
- Inheritance for themes ([#3067](https://github.com/helix-editor/helix/pull/3067), [#4096](https://github.com/helix-editor/helix/pull/4096))
- Cursorcolumn ([#4084](https://github.com/helix-editor/helix/pull/4084))
- Overhauled system for writing files and quiting ([#2267](https://github.com/helix-editor/helix/pull/2267), [#4397](https://github.com/helix-editor/helix/pull/4397))
- Autosave when terminal loses focus ([#3178](https://github.com/helix-editor/helix/pull/3178))
- Use OSC52 as a fallback for the system clipboard ([#3220](https://github.com/helix-editor/helix/pull/3220))
- Show git diffs in the gutter ([#3890](https://github.com/helix-editor/helix/pull/3890), [#5012](https://github.com/helix-editor/helix/pull/5012), [#4995](https://github.com/helix-editor/helix/pull/4995))
- Add a logo ([dc1ec56](https://github.com/helix-editor/helix/commit/dc1ec56))
- Multi-cursor completion ([#4496](https://github.com/helix-editor/helix/pull/4496))

Commands:

- `file_picker_in_current_directory` (`<space>F`) ([#3701](https://github.com/helix-editor/helix/pull/3701))
- `:lsp-restart` to restart the current document's language server ([#3435](https://github.com/helix-editor/helix/pull/3435), [#3972](https://github.com/helix-editor/helix/pull/3972))
- `join_selections_space` (`A-j`) which joins selections and selects the joining whitespace ([#3549](https://github.com/helix-editor/helix/pull/3549))
- `:update` to write the current file if it is modified ([#4426](https://github.com/helix-editor/helix/pull/4426))
- `:lsp-workspace-command` for picking LSP commands to execute ([#3140](https://github.com/helix-editor/helix/pull/3140))
- `extend_prev_word_end` - the extend variant for `move_prev_word_end` ([7468fa2](https://github.com/helix-editor/helix/commit/7468fa2))
- `make_search_word_bounded` which adds regex word boundaries to the current search register value ([#4322](https://github.com/helix-editor/helix/pull/4322))
- `:reload-all` - `:reload` for all open buffers ([#4663](https://github.com/helix-editor/helix/pull/4663), [#4901](https://github.com/helix-editor/helix/pull/4901))
- `goto_next_change` (`]g`), `goto_prev_change` (`[g`), `goto_first_change` (`[G`), `goto_last_change` (`]G`) textobjects for jumping between VCS changes ([#4650](https://github.com/helix-editor/helix/pull/4650))

Usability improvements and fixes:

- Don't log 'LSP not defined' errors in the logfile ([1caba2d](https://github.com/helix-editor/helix/commit/1caba2d))
- Look for the external formatter program before invoking it ([#3670](https://github.com/helix-editor/helix/pull/3670))
- Don't send LSP didOpen events for documents without URLs ([44b4479](https://github.com/helix-editor/helix/commit/44b4479))
- Fix off-by-one in `extend_line_above` command ([#3689](https://github.com/helix-editor/helix/pull/3689))
- Use the original scroll offset when opening a split ([1acdfaa](https://github.com/helix-editor/helix/commit/1acdfaa))
- Handle auto-formatting failures and save the file anyway ([#3684](https://github.com/helix-editor/helix/pull/3684))
- Ensure the cursor is in view after `:reflow` ([#3733](https://github.com/helix-editor/helix/pull/3733))
- Add default rulers and reflow config for git commit messages ([#3738](https://github.com/helix-editor/helix/pull/3738))
- Improve grammar fetching and building output ([#3773](https://github.com/helix-editor/helix/pull/3773))
- Add a `text` language to language completion ([cc47d3f](https://github.com/helix-editor/helix/commit/cc47d3f))
- Improve error handling for `:set-language` ([e8add6f](https://github.com/helix-editor/helix/commit/e8add6f))
- Improve error handling for `:config-reload` ([#3668](https://github.com/helix-editor/helix/pull/3668))
- Improve error handling when passing improper ranges to syntax highlighting ([#3826](https://github.com/helix-editor/helix/pull/3826))
- Render `<code>` tags as raw markup in markdown ([#3425](https://github.com/helix-editor/helix/pull/3425))
- Remove border around the LSP code-actions popup ([#3444](https://github.com/helix-editor/helix/pull/3444))
- Canonicalize the path to the runtime directory ([#3794](https://github.com/helix-editor/helix/pull/3794))
- Add a `themelint` xtask for linting themes ([#3234](https://github.com/helix-editor/helix/pull/3234))
- Re-sort LSP diagnostics after applying transactions ([#3895](https://github.com/helix-editor/helix/pull/3895), [#4319](https://github.com/helix-editor/helix/pull/4319))
- Add a command-line flag to specify the log file ([#3807](https://github.com/helix-editor/helix/pull/3807))
- Track source and tag information in LSP diagnostics ([#3898](https://github.com/helix-editor/helix/pull/3898), [1df32c9](https://github.com/helix-editor/helix/commit/1df32c9))
- Fix theme returning to normal when exiting the `:theme` completion ([#3644](https://github.com/helix-editor/helix/pull/3644))
- Improve error messages for invalid commands in the keymap ([#3931](https://github.com/helix-editor/helix/pull/3931))
- Deduplicate regexs in `search_selection` command ([#3941](https://github.com/helix-editor/helix/pull/3941))
- Split the finding of LSP root and config roots ([#3929](https://github.com/helix-editor/helix/pull/3929))
- Ensure that the cursor is within view after auto-formatting ([#4047](https://github.com/helix-editor/helix/pull/4047))
- Add pseudo-pending to commands with on-next-key callbacks ([#4062](https://github.com/helix-editor/helix/pull/4062), [#4077](https://github.com/helix-editor/helix/pull/4077))
- Add live preview to `:goto` ([#2982](https://github.com/helix-editor/helix/pull/2982))
- Show regex compilation failure in a popup ([#3049](https://github.com/helix-editor/helix/pull/3049))
- Add 'cycled to end' and 'no more matches' for search ([#3176](https://github.com/helix-editor/helix/pull/3176), [#4101](https://github.com/helix-editor/helix/pull/4101))
- Add extending behavior to tree-sitter textobjects ([#3266](https://github.com/helix-editor/helix/pull/3266))
- Add `ui.gutter.selected` option for themes ([#3303](https://github.com/helix-editor/helix/pull/3303))
- Make statusline mode names configurable ([#3311](https://github.com/helix-editor/helix/pull/3311))
- Add a statusline element for total line count ([#3960](https://github.com/helix-editor/helix/pull/3960))
- Add extending behavior to `goto_window_*` commands ([#3985](https://github.com/helix-editor/helix/pull/3985))
- Fix a panic in signature help when the preview is too large ([#4030](https://github.com/helix-editor/helix/pull/4030))
- Add command names to the command palette ([#4071](https://github.com/helix-editor/helix/pull/4071), [#4223](https://github.com/helix-editor/helix/pull/4223), [#4495](https://github.com/helix-editor/helix/pull/4495))
- Find the LSP workspace root from the current document's path ([#3553](https://github.com/helix-editor/helix/pull/3553))
- Add an option to skip indent-guide levels ([#3819](https://github.com/helix-editor/helix/pull/3819), [2c36e33](https://github.com/helix-editor/helix/commit/2c36e33))
- Change focus to modified docs on quit ([#3872](https://github.com/helix-editor/helix/pull/3872))
- Respond to `USR1` signal by reloading config ([#3952](https://github.com/helix-editor/helix/pull/3952))
- Exit gracefully when the close operation fails ([#4081](https://github.com/helix-editor/helix/pull/4081))
- Fix goto/view center mismatch ([#4135](https://github.com/helix-editor/helix/pull/4135))
- Highlight the current file picker document on idle-timeout ([#3172](https://github.com/helix-editor/helix/pull/3172), [a85e386](https://github.com/helix-editor/helix/commit/a85e386))
- Apply transactions to jumplist selections ([#4186](https://github.com/helix-editor/helix/pull/4186), [#4227](https://github.com/helix-editor/helix/pull/4227), [#4733](https://github.com/helix-editor/helix/pull/4733), [#4865](https://github.com/helix-editor/helix/pull/4865), [#4912](https://github.com/helix-editor/helix/pull/4912), [#4965](https://github.com/helix-editor/helix/pull/4965), [#4981](https://github.com/helix-editor/helix/pull/4981))
- Use space as a separator for fuzzy matcher ([#3969](https://github.com/helix-editor/helix/pull/3969))
- Overlay all diagnostics with highest severity on top ([#4113](https://github.com/helix-editor/helix/pull/4113))
- Avoid re-parsing unmodified tree-sitter injections ([#4146](https://github.com/helix-editor/helix/pull/4146))
- Add extending captures for indentation, re-enable python indentation ([#3382](https://github.com/helix-editor/helix/pull/3382), [3e84434](https://github.com/helix-editor/helix/commit/3e84434))
- Only allow either `--vsplit` or `--hsplit` CLI flags at once ([#4202](https://github.com/helix-editor/helix/pull/4202))
- Fix append cursor location when selection anchor is at the end of the document ([#4147](https://github.com/helix-editor/helix/pull/4147))
- Improve selection yanking message ([#4275](https://github.com/helix-editor/helix/pull/4275))
- Log failures to load tree-sitter grammars as errors ([#4315](https://github.com/helix-editor/helix/pull/4315))
- Fix rendering of lines longer than 65,536 columns ([#4172](https://github.com/helix-editor/helix/pull/4172))
- Skip searching `.git` in `global_search` ([#4334](https://github.com/helix-editor/helix/pull/4334))
- Display tree-sitter scopes in a popup ([#4337](https://github.com/helix-editor/helix/pull/4337))
- Fix deleting a word from the end of the buffer ([#4328](https://github.com/helix-editor/helix/pull/4328))
- Pretty print the syntax tree in `:tree-sitter-subtree` ([#4295](https://github.com/helix-editor/helix/pull/4295), [#4606](https://github.com/helix-editor/helix/pull/4606))
- Allow specifying suffixes for file-type detection ([#2455](https://github.com/helix-editor/helix/pull/2455), [#4414](https://github.com/helix-editor/helix/pull/4414))
- Fix multi-byte auto-pairs ([#4024](https://github.com/helix-editor/helix/pull/4024))
- Improve sort scoring for LSP code-actions and completions ([#4134](https://github.com/helix-editor/helix/pull/4134))
- Fix the handling of quotes within shellwords ([#4098](https://github.com/helix-editor/helix/pull/4098))
- Fix `delete_word_backward` and `delete_word_forward` on newlines ([#4392](https://github.com/helix-editor/helix/pull/4392))
- Fix 'no entry found for key' crash on `:write-all` ([#4384](https://github.com/helix-editor/helix/pull/4384))
- Remove lowercase requirement for tree-sitter grammars ([#4346](https://github.com/helix-editor/helix/pull/4346))
- Resolve LSP completion items on idle-timeout ([#4406](https://github.com/helix-editor/helix/pull/4406), [#4797](https://github.com/helix-editor/helix/pull/4797))
- Render diagnostics in the file picker preview ([#4324](https://github.com/helix-editor/helix/pull/4324))
- Fix terminal freezing on `shell_insert_output` ([#4156](https://github.com/helix-editor/helix/pull/4156))
- Allow use of the count in the repeat operator (`.`) ([#4450](https://github.com/helix-editor/helix/pull/4450))
- Show the current theme name on `:theme` with no arguments ([#3740](https://github.com/helix-editor/helix/pull/3740))
- Fix rendering in very large terminals ([#4318](https://github.com/helix-editor/helix/pull/4318))
- Sort LSP preselected items to the top of the completion menu ([#4480](https://github.com/helix-editor/helix/pull/4480))
- Trim braces and quotes from paths in goto-file ([#4370](https://github.com/helix-editor/helix/pull/4370))
- Prevent automatic signature help outside of insert mode ([#4456](https://github.com/helix-editor/helix/pull/4456))
- Fix freezes with external programs that process stdin and stdout concurrently ([#4180](https://github.com/helix-editor/helix/pull/4180))
- Make `scroll` aware of tabs and wide characters ([#4519](https://github.com/helix-editor/helix/pull/4519))
- Correctly handle escaping in `command_mode` completion ([#4316](https://github.com/helix-editor/helix/pull/4316), [#4587](https://github.com/helix-editor/helix/pull/4587), [#4632](https://github.com/helix-editor/helix/pull/4632))
- Fix `delete_char_backward` for paired characters ([#4558](https://github.com/helix-editor/helix/pull/4558))
- Fix crash from two windows editing the same document ([#4570](https://github.com/helix-editor/helix/pull/4570))
- Fix pasting from the blackhole register ([#4497](https://github.com/helix-editor/helix/pull/4497))
- Support LSP insertReplace completion items ([1312682](https://github.com/helix-editor/helix/commit/1312682))
- Dynamically resize the line number gutter width ([#3469](https://github.com/helix-editor/helix/pull/3469))
- Fix crash for unknown completion item kinds ([#4658](https://github.com/helix-editor/helix/pull/4658))
- Re-enable `format_selections` for single selection ranges ([d4f5cab](https://github.com/helix-editor/helix/commit/d4f5cab))
- Limit the number of in-progress tree-sitter query matches ([#4707](https://github.com/helix-editor/helix/pull/4707), [#4830](https://github.com/helix-editor/helix/pull/4830))
- Use the special `#` register with `increment`/`decrement` to change by range number ([#4418](https://github.com/helix-editor/helix/pull/4418))
- Add a statusline element to show number of selected chars ([#4682](https://github.com/helix-editor/helix/pull/4682))
- Add a statusline element showing global LSP diagnostic warning and error counts ([#4569](https://github.com/helix-editor/helix/pull/4569))
- Add a scrollbar to popups ([#4449](https://github.com/helix-editor/helix/pull/4449))
- Prefer shorter matches in fuzzy matcher scoring ([#4698](https://github.com/helix-editor/helix/pull/4698))
- Use key-sequence format for command palette keybinds ([#4712](https://github.com/helix-editor/helix/pull/4712))
- Remove prefix filtering from autocompletion menu ([#4578](https://github.com/helix-editor/helix/pull/4578))
- Focus on the parent buffer when closing a split ([#4766](https://github.com/helix-editor/helix/pull/4766))
- Handle language server termination ([#4797](https://github.com/helix-editor/helix/pull/4797), [#4852](https://github.com/helix-editor/helix/pull/4852))
- Allow `r`/`t`/`f` to work on tab characters ([#4817](https://github.com/helix-editor/helix/pull/4817))
- Show a preview for scratch buffers in the buffer picker ([#3454](https://github.com/helix-editor/helix/pull/3454))
- Set a limit of entries in the jumplist ([#4750](https://github.com/helix-editor/helix/pull/4750))
- Re-use shell outputs when inserting or appending shell output ([#3465](https://github.com/helix-editor/helix/pull/3465))
- Check LSP server provider capabilities ([#3554](https://github.com/helix-editor/helix/pull/3554))
- Improve tree-sitter parsing performance on files with many language layers ([#4716](https://github.com/helix-editor/helix/pull/4716))
- Move indentation to the next line when using `<ret>` on a line with only whitespace ([#4854](https://github.com/helix-editor/helix/pull/4854))
- Remove selections for closed views from all documents ([#4888](https://github.com/helix-editor/helix/pull/4888))
- Improve performance of the `:reload` command ([#4457](https://github.com/helix-editor/helix/pull/4457))
- Properly handle media keys ([#4887](https://github.com/helix-editor/helix/pull/4887))
- Support LSP diagnostic data field ([#4935](https://github.com/helix-editor/helix/pull/4935))
- Handle C-i keycode as tab ([#4961](https://github.com/helix-editor/helix/pull/4961))
- Fix view alignment for jumplist picker jumps ([#3743](https://github.com/helix-editor/helix/pull/3743))
- Use OSC52 for tmux clipboard provider ([#5027](https://github.com/helix-editor/helix/pull/5027))

Themes:

- Add `varua` ([#3610](https://github.com/helix-editor/helix/pull/3610), [#4964](https://github.com/helix-editor/helix/pull/4964))
- Update `boo_berry` ([#3653](https://github.com/helix-editor/helix/pull/3653))
- Add `rasmus` ([#3728](https://github.com/helix-editor/helix/pull/3728))
- Add `papercolor_dark` ([#3742](https://github.com/helix-editor/helix/pull/3742))
- Update `monokai_pro_spectrum` ([#3814](https://github.com/helix-editor/helix/pull/3814))
- Update `nord` ([#3792](https://github.com/helix-editor/helix/pull/3792))
- Update `fleetish` ([#3844](https://github.com/helix-editor/helix/pull/3844), [#4487](https://github.com/helix-editor/helix/pull/4487), [#4813](https://github.com/helix-editor/helix/pull/4813))
- Update `flatwhite` ([#3843](https://github.com/helix-editor/helix/pull/3843))
- Add `darcula` ([#3739](https://github.com/helix-editor/helix/pull/3739))
- Update `papercolor` ([#3938](https://github.com/helix-editor/helix/pull/3938), [#4317](https://github.com/helix-editor/helix/pull/4317))
- Add bufferline colors to multiple themes ([#3881](https://github.com/helix-editor/helix/pull/3881))
- Add `gruvbox_dark_hard` ([#3948](https://github.com/helix-editor/helix/pull/3948))
- Add `onedarker` ([#3980](https://github.com/helix-editor/helix/pull/3980), [#4060](https://github.com/helix-editor/helix/pull/4060))
- Add `dark_high_contrast` ([#3312](https://github.com/helix-editor/helix/pull/3312))
- Update `bogster` ([#4121](https://github.com/helix-editor/helix/pull/4121), [#4264](https://github.com/helix-editor/helix/pull/4264))
- Update `sonokai` ([#4089](https://github.com/helix-editor/helix/pull/4089))
- Update `ayu_*` themes ([#4140](https://github.com/helix-editor/helix/pull/4140), [#4109](https://github.com/helix-editor/helix/pull/4109), [#4662](https://github.com/helix-editor/helix/pull/4662), [#4764](https://github.com/helix-editor/helix/pull/4764))
- Update `everforest` ([#3998](https://github.com/helix-editor/helix/pull/3998))
- Update `monokai_pro_octagon` ([#4247](https://github.com/helix-editor/helix/pull/4247))
- Add `heisenberg` ([#4209](https://github.com/helix-editor/helix/pull/4209))
- Add `bogster_light` ([#4265](https://github.com/helix-editor/helix/pull/4265))
- Update `pop-dark` ([#4323](https://github.com/helix-editor/helix/pull/4323))
- Update `rose_pine` ([#4221](https://github.com/helix-editor/helix/pull/4221))
- Add `kanagawa` ([#4300](https://github.com/helix-editor/helix/pull/4300))
- Add `hex_steel`, `hex_toxic` and `hex_lavendar` ([#4367](https://github.com/helix-editor/helix/pull/4367), [#4990](https://github.com/helix-editor/helix/pull/4990))
- Update `tokyonight` and `tokyonight_storm` ([#4415](https://github.com/helix-editor/helix/pull/4415))
- Update `gruvbox` ([#4626](https://github.com/helix-editor/helix/pull/4626))
- Update `dark_plus` ([#4661](https://github.com/helix-editor/helix/pull/4661), [#4678](https://github.com/helix-editor/helix/pull/4678))
- Add `zenburn` ([#4613](https://github.com/helix-editor/helix/pull/4613), [#4977](https://github.com/helix-editor/helix/pull/4977))
- Update `monokai_pro` ([#4789](https://github.com/helix-editor/helix/pull/4789))
- Add `mellow` ([#4770](https://github.com/helix-editor/helix/pull/4770))
- Add `nightfox` ([#4769](https://github.com/helix-editor/helix/pull/4769), [#4966](https://github.com/helix-editor/helix/pull/4966))
- Update `doom_acario_dark` ([#4979](https://github.com/helix-editor/helix/pull/4979))
- Update `autumn` ([#4996](https://github.com/helix-editor/helix/pull/4996))
- Update `acme` ([#4999](https://github.com/helix-editor/helix/pull/4999))
- Update `nord_light` ([#4999](https://github.com/helix-editor/helix/pull/4999))
- Update `serika_*` ([#5015](https://github.com/helix-editor/helix/pull/5015))

LSP configurations:

- Switch to `openscad-lsp` for OpenScad ([#3750](https://github.com/helix-editor/helix/pull/3750))
- Support Jsonnet ([#3748](https://github.com/helix-editor/helix/pull/3748))
- Support Markdown ([#3499](https://github.com/helix-editor/helix/pull/3499))
- Support Bass ([#3771](https://github.com/helix-editor/helix/pull/3771))
- Set roots configuration for Elixir and HEEx ([#3917](https://github.com/helix-editor/helix/pull/3917), [#3959](https://github.com/helix-editor/helix/pull/3959))
- Support Purescript ([#4242](https://github.com/helix-editor/helix/pull/4242))
- Set roots configuration for Julia ([#4361](https://github.com/helix-editor/helix/pull/4361))
- Support D ([#4372](https://github.com/helix-editor/helix/pull/4372))
- Increase default language server timeout for Julia ([#4575](https://github.com/helix-editor/helix/pull/4575))
- Use ElixirLS for HEEx ([#4679](https://github.com/helix-editor/helix/pull/4679))
- Support Bicep ([#4403](https://github.com/helix-editor/helix/pull/4403))
- Switch to `nil` for Nix ([433ccef](https://github.com/helix-editor/helix/commit/433ccef))
- Support QML ([#4842](https://github.com/helix-editor/helix/pull/4842))
- Enable auto-format for CSS ([#4987](https://github.com/helix-editor/helix/pull/4987))
- Support CommonLisp ([4176769](https://github.com/helix-editor/helix/commit/4176769))

New languages:

- SML ([#3692](https://github.com/helix-editor/helix/pull/3692))
- Jsonnet ([#3714](https://github.com/helix-editor/helix/pull/3714))
- Godot resource ([#3759](https://github.com/helix-editor/helix/pull/3759))
- Astro ([#3829](https://github.com/helix-editor/helix/pull/3829))
- SSH config ([#2455](https://github.com/helix-editor/helix/pull/2455), [#4538](https://github.com/helix-editor/helix/pull/4538))
- Bass ([#3771](https://github.com/helix-editor/helix/pull/3771))
- WAT (WebAssembly text format) ([#4040](https://github.com/helix-editor/helix/pull/4040), [#4542](https://github.com/helix-editor/helix/pull/4542))
- Purescript ([#4242](https://github.com/helix-editor/helix/pull/4242))
- D ([#4372](https://github.com/helix-editor/helix/pull/4372), [#4562](https://github.com/helix-editor/helix/pull/4562))
- VHS ([#4486](https://github.com/helix-editor/helix/pull/4486))
- KDL ([#4481](https://github.com/helix-editor/helix/pull/4481))
- XML ([#4518](https://github.com/helix-editor/helix/pull/4518))
- WIT ([#4525](https://github.com/helix-editor/helix/pull/4525))
- ENV ([#4536](https://github.com/helix-editor/helix/pull/4536))
- INI ([#4538](https://github.com/helix-editor/helix/pull/4538))
- Bicep ([#4403](https://github.com/helix-editor/helix/pull/4403), [#4751](https://github.com/helix-editor/helix/pull/4751))
- QML ([#4842](https://github.com/helix-editor/helix/pull/4842))
- CommonLisp ([4176769](https://github.com/helix-editor/helix/commit/4176769))

Updated languages and queries:

- Zig ([#3621](https://github.com/helix-editor/helix/pull/3621), [#4745](https://github.com/helix-editor/helix/pull/4745))
- Rust ([#3647](https://github.com/helix-editor/helix/pull/3647), [#3729](https://github.com/helix-editor/helix/pull/3729), [#3927](https://github.com/helix-editor/helix/pull/3927), [#4073](https://github.com/helix-editor/helix/pull/4073), [#4510](https://github.com/helix-editor/helix/pull/4510), [#4659](https://github.com/helix-editor/helix/pull/4659), [#4717](https://github.com/helix-editor/helix/pull/4717))
- Solidity ([20ed8c2](https://github.com/helix-editor/helix/commit/20ed8c2))
- Fish ([#3704](https://github.com/helix-editor/helix/pull/3704))
- Elixir ([#3645](https://github.com/helix-editor/helix/pull/3645), [#4333](https://github.com/helix-editor/helix/pull/4333), [#4821](https://github.com/helix-editor/helix/pull/4821))
- Diff ([#3708](https://github.com/helix-editor/helix/pull/3708))
- Nix ([665e27f](https://github.com/helix-editor/helix/commit/665e27f), [1fe3273](https://github.com/helix-editor/helix/commit/1fe3273))
- Markdown ([#3749](https://github.com/helix-editor/helix/pull/3749), [#4078](https://github.com/helix-editor/helix/pull/4078), [#4483](https://github.com/helix-editor/helix/pull/4483), [#4478](https://github.com/helix-editor/helix/pull/4478))
- GDScript ([#3760](https://github.com/helix-editor/helix/pull/3760))
- JSX and TSX ([#3853](https://github.com/helix-editor/helix/pull/3853), [#3973](https://github.com/helix-editor/helix/pull/3973))
- Ruby ([#3976](https://github.com/helix-editor/helix/pull/3976), [#4601](https://github.com/helix-editor/helix/pull/4601))
- R ([#4031](https://github.com/helix-editor/helix/pull/4031))
- WGSL ([#3996](https://github.com/helix-editor/helix/pull/3996), [#4079](https://github.com/helix-editor/helix/pull/4079))
- C# ([#4118](https://github.com/helix-editor/helix/pull/4118), [#4281](https://github.com/helix-editor/helix/pull/4281), [#4213](https://github.com/helix-editor/helix/pull/4213))
- Twig ([#4176](https://github.com/helix-editor/helix/pull/4176))
- Lua ([#3552](https://github.com/helix-editor/helix/pull/3552))
- C/C++ ([#4079](https://github.com/helix-editor/helix/pull/4079), [#4278](https://github.com/helix-editor/helix/pull/4278), [#4282](https://github.com/helix-editor/helix/pull/4282))
- Cairo ([17488f1](https://github.com/helix-editor/helix/commit/17488f1), [431f9c1](https://github.com/helix-editor/helix/commit/431f9c1), [09a6df1](https://github.com/helix-editor/helix/commit/09a6df1))
- Rescript ([#4356](https://github.com/helix-editor/helix/pull/4356))
- Zig ([#4409](https://github.com/helix-editor/helix/pull/4409))
- Scala ([#4353](https://github.com/helix-editor/helix/pull/4353), [#4697](https://github.com/helix-editor/helix/pull/4697), [#4701](https://github.com/helix-editor/helix/pull/4701))
- LaTeX ([#4528](https://github.com/helix-editor/helix/pull/4528), [#4922](https://github.com/helix-editor/helix/pull/4922))
- SQL ([#4529](https://github.com/helix-editor/helix/pull/4529))
- Python ([#4560](https://github.com/helix-editor/helix/pull/4560))
- Bash/Zsh ([#4582](https://github.com/helix-editor/helix/pull/4582))
- Nu ([#4583](https://github.com/helix-editor/helix/pull/4583))
- Julia ([#4588](https://github.com/helix-editor/helix/pull/4588))
- Typescript ([#4703](https://github.com/helix-editor/helix/pull/4703))
- Meson ([#4572](https://github.com/helix-editor/helix/pull/4572))
- Haskell ([#4800](https://github.com/helix-editor/helix/pull/4800))
- CMake ([#4809](https://github.com/helix-editor/helix/pull/4809))
- HTML ([#4829](https://github.com/helix-editor/helix/pull/4829), [#4881](https://github.com/helix-editor/helix/pull/4881))
- Java ([#4886](https://github.com/helix-editor/helix/pull/4886))
- Go ([#4906](https://github.com/helix-editor/helix/pull/4906), [#4969](https://github.com/helix-editor/helix/pull/4969), [#5010](https://github.com/helix-editor/helix/pull/5010))
- CSS ([#4882](https://github.com/helix-editor/helix/pull/4882))
- Racket ([#4915](https://github.com/helix-editor/helix/pull/4915))
- SCSS ([#5003](https://github.com/helix-editor/helix/pull/5003))

Packaging:

- Filter relevant source files in the Nix flake ([#3657](https://github.com/helix-editor/helix/pull/3657))
- Build a binary for `aarch64-linux` in the release CI ([038a91d](https://github.com/helix-editor/helix/commit/038a91d))
- Build an AppImage for `aarch64-linux` in the release CI ([b738031](https://github.com/helix-editor/helix/commit/b738031))
- Enable CI builds for `riscv64-linux` ([#3685](https://github.com/helix-editor/helix/pull/3685))
- Support preview releases in CI ([0090a2d](https://github.com/helix-editor/helix/commit/0090a2d))
- Strip binaries built in CI ([#3780](https://github.com/helix-editor/helix/pull/3780))
- Fix the development shell for the Nix Flake on `aarch64-darwin` ([#3810](https://github.com/helix-editor/helix/pull/3810))
- Raise the MSRV and create an MSRV policy ([#3896](https://github.com/helix-editor/helix/pull/3896), [#3913](https://github.com/helix-editor/helix/pull/3913), [#3961](https://github.com/helix-editor/helix/pull/3961))
- Fix Fish completions for `--config` and `--log` flags ([#3912](https://github.com/helix-editor/helix/pull/3912))
- Use builtin filenames option in Bash completion ([#4648](https://github.com/helix-editor/helix/pull/4648))

# 22.08.1 (2022-09-01)

This is a patch release that fixes a panic caused by closing splits or buffers. ([#3633](https://github.com/helix-editor/helix/pull/3633))

# 22.08 (2022-08-31)

A big _thank you_ to our contributors! This release had 87 contributors.

As usual, the following is a summary of each of the changes since the last release.
For the full log, check out the [git log](https://github.com/helix-editor/helix/compare/22.05..22.08).

Breaking changes:

- Special keymap names for `+`, `;` and `%` have been replaced with those literal characters ([#2677](https://github.com/helix-editor/helix/pull/2677), [#3556](https://github.com/helix-editor/helix/pull/3556))
- `A-Left` and `A-Right` have become `C-Left` and `C-Right` for word-wise motion ([#2500](https://github.com/helix-editor/helix/pull/2500))
- The `catppuccin` theme's name has been corrected from `catpuccin` ([#2713](https://github.com/helix-editor/helix/pull/2713))
- `catppuccin` has been replaced by its variants, `catppuccin_frappe`, `catppuccin_latte`, `catppuccin_macchiato`, `catppuccin_mocha` ([#3281](https://github.com/helix-editor/helix/pull/3281))
- `C-n` and `C-p` have been removed from the default insert mode keymap ([#3340](https://github.com/helix-editor/helix/pull/3340))
- The `extend_line` command has been replaced with `extend_line_below` and a new `extend_line` command now exists ([#3046](https://github.com/helix-editor/helix/pull/3046))

Features:

- Add an integration testing harness ([#2359](https://github.com/helix-editor/helix/pull/2359))
- Indent guides ([#1796](https://github.com/helix-editor/helix/pull/1796), [906259c](https://github.com/helix-editor/helix/commit/906259c))
- Cursorline ([#2170](https://github.com/helix-editor/helix/pull/2170), [fde9e03](https://github.com/helix-editor/helix/commit/fde9e03))
- Select all instances of the symbol under the cursor (`<space>h`) ([#2738](https://github.com/helix-editor/helix/pull/2738))
- A picker for document and workspace LSP diagnostics (`<space>g`/`<space>G`) ([#2013](https://github.com/helix-editor/helix/pull/2013), [#2984](https://github.com/helix-editor/helix/pull/2984))
- Allow styling the mode indicator per-mode ([#2676](https://github.com/helix-editor/helix/pull/2676))
- Live preview for the theme picker ([#1798](https://github.com/helix-editor/helix/pull/1798))
- Configurable statusline ([#2434](https://github.com/helix-editor/helix/pull/2434))
- LSP SignatureHelp ([#1755](https://github.com/helix-editor/helix/pull/1755), [a8b123f](https://github.com/helix-editor/helix/commit/a8b123f))
- A picker for the jumplist ([#3033](https://github.com/helix-editor/helix/pull/3033))
- Configurable external formatter binaries ([#2942](https://github.com/helix-editor/helix/pull/2942))
- Bracketed paste support ([#3233](https://github.com/helix-editor/helix/pull/3233), [12ddd03](https://github.com/helix-editor/helix/commit/12ddd03))

Commands:

- `:insert-output` and `:append-output` which insert/append output from a shell command ([#2589](https://github.com/helix-editor/helix/pull/2589))
- The `t` textobject (`]t`/`[t`/`mit`/`mat`) for navigating tests ([#2807](https://github.com/helix-editor/helix/pull/2807))
- `C-Backspace` and `C-Delete` for word-wise deletion in prompts and pickers ([#2500](https://github.com/helix-editor/helix/pull/2500))
- `A-Delete` for forward word-wise deletion in insert mode ([#2500](https://github.com/helix-editor/helix/pull/2500))
- `C-t` for toggling the preview pane in pickers ([#3021](https://github.com/helix-editor/helix/pull/3021))
- `extend_line` now extends in the direction of the cursor ([#3046](https://github.com/helix-editor/helix/pull/3046))

Usability improvements and fixes:

- Fix tree-sitter parser builds on illumos ([#2602](https://github.com/helix-editor/helix/pull/2602))
- Remove empty stratch buffer from jumplists when removing ([5ed6223](https://github.com/helix-editor/helix/commit/5ed6223))
- Fix panic on undo after `shell_append_output` ([#2625](https://github.com/helix-editor/helix/pull/2625))
- Sort LSP edits by start range ([3d91c99](https://github.com/helix-editor/helix/commit/3d91c99))
- Be more defensive about LSP URI conversions ([6de6a3e](https://github.com/helix-editor/helix/commit/6de6a3e), [378f438](https://github.com/helix-editor/helix/commit/378f438))
- Ignore SendErrors when grammar builds fail ([#2641](https://github.com/helix-editor/helix/pull/2641))
- Append `set_line_ending` to document history ([#2649](https://github.com/helix-editor/helix/pull/2649))
- Use last prompt entry when empty ([b14c258](https://github.com/helix-editor/helix/commit/b14c258), [#2870](https://github.com/helix-editor/helix/pull/2870))
- Do not add extra line breaks in markdown lists ([#2689](https://github.com/helix-editor/helix/pull/2689))
- Disable dialyzer by default for ElixirLS ([#2710](https://github.com/helix-editor/helix/pull/2710))
- Refactor textobject node capture ([#2741](https://github.com/helix-editor/helix/pull/2741))
- Prevent re-selecting the same range with `expand_selection` ([#2760](https://github.com/helix-editor/helix/pull/2760))
- Introduce `keyword.storage` highlight scope ([#2731](https://github.com/helix-editor/helix/pull/2731))
- Handle symlinks more consistently ([#2718](https://github.com/helix-editor/helix/pull/2718))
- Improve markdown list rendering ([#2687](https://github.com/helix-editor/helix/pull/2687))
- Update auto-pairs and idle-timout settings when the config is reloaded ([#2736](https://github.com/helix-editor/helix/pull/2736))
- Fix panic on closing last buffer ([#2658](https://github.com/helix-editor/helix/pull/2658))
- Prevent modifying jumplist until jumping to a reference ([#2670](https://github.com/helix-editor/helix/pull/2670))
- Ensure `:quit` and `:quit!` take no arguments ([#2654](https://github.com/helix-editor/helix/pull/2654))
- Fix crash due to cycles when replaying macros ([#2647](https://github.com/helix-editor/helix/pull/2647))
- Pass LSP FormattingOptions ([#2635](https://github.com/helix-editor/helix/pull/2635))
- Prevent showing colors when the health-check is piped ([#2836](https://github.com/helix-editor/helix/pull/2836))
- Use character indexing for mouse selection ([#2839](https://github.com/helix-editor/helix/pull/2839))
- Display the highest severity diagnostic for a line in the gutter ([#2835](https://github.com/helix-editor/helix/pull/2835))
- Default the ruler color to red background ([#2669](https://github.com/helix-editor/helix/pull/2669))
- Make `move_vertically` aware of tabs and wide characters ([#2620](https://github.com/helix-editor/helix/pull/2620))
- Enable shellwords for Windows ([#2767](https://github.com/helix-editor/helix/pull/2767))
- Add history suggestions to global search ([#2717](https://github.com/helix-editor/helix/pull/2717))
- Fix the scrollbar's length proportional to total menu items ([#2860](https://github.com/helix-editor/helix/pull/2860))
- Reset terminal modifiers for diagnostic text ([#2861](https://github.com/helix-editor/helix/pull/2861), [#2900](https://github.com/helix-editor/helix/pull/2900))
- Redetect indents and line-endings after a Language Server replaces the document ([#2778](https://github.com/helix-editor/helix/pull/2778))
- Check selection's visible width when copying on mouse click ([#2711](https://github.com/helix-editor/helix/pull/2711))
- Fix edge-case in tree-sitter `expand_selection` command ([#2877](https://github.com/helix-editor/helix/pull/2877))
- Add a single-width left margin for the completion popup ([#2728](https://github.com/helix-editor/helix/pull/2728))
- Right-align the scrollbar in the completion popup ([#2754](https://github.com/helix-editor/helix/pull/2754))
- Fix recursive macro crash and empty macro lockout ([#2902](https://github.com/helix-editor/helix/pull/2902))
- Fix backwards character deletion on other whitespaces ([#2855](https://github.com/helix-editor/helix/pull/2855))
- Add search and space/backspace bindings to view modes ([#2803](https://github.com/helix-editor/helix/pull/2803))
- Add `--vsplit` and `--hsplit` CLI arguments for opening in splits ([#2773](https://github.com/helix-editor/helix/pull/2773), [#3073](https://github.com/helix-editor/helix/pull/3073))
- Sort themes, languages and files inputs by score and name ([#2675](https://github.com/helix-editor/helix/pull/2675))
- Highlight entire rows in ([#2939](https://github.com/helix-editor/helix/pull/2939))
- Fix backwards selection duplication widening bug ([#2945](https://github.com/helix-editor/helix/pull/2945), [#3024](https://github.com/helix-editor/helix/pull/3024))
- Skip serializing Option type DAP fields ([44f5963](https://github.com/helix-editor/helix/commit/44f5963))
- Fix required `cwd` field in DAP `RunTerminalArguments` type ([85411be](https://github.com/helix-editor/helix/commit/85411be), [#3240](https://github.com/helix-editor/helix/pull/3240))
- Add LSP `workspace/applyEdit` to client capabilities ([#3012](https://github.com/helix-editor/helix/pull/3012))
- Respect count for repeating motion ([#3057](https://github.com/helix-editor/helix/pull/3057))
- Respect count for selecting next/previous match ([#3056](https://github.com/helix-editor/helix/pull/3056))
- Respect count for tree-sitter motions ([#3058](https://github.com/helix-editor/helix/pull/3058))
- Make gutters padding optional ([#2996](https://github.com/helix-editor/helix/pull/2996))
- Support pre-filling prompts ([#2459](https://github.com/helix-editor/helix/pull/2459), [#3259](https://github.com/helix-editor/helix/pull/3259))
- Add statusline element to display file line-endings ([#3113](https://github.com/helix-editor/helix/pull/3113))
- Keep jump and file history when using `:split` ([#3031](https://github.com/helix-editor/helix/pull/3031), [#3160](https://github.com/helix-editor/helix/pull/3160))
- Make tree-sitter query `; inherits <language>` feature imperative ([#2470](https://github.com/helix-editor/helix/pull/2470))
- Indent with tabs by default ([#3095](https://github.com/helix-editor/helix/pull/3095))
- Fix non-msvc grammar compilation on Windows ([#3190](https://github.com/helix-editor/helix/pull/3190))
- Add spacer element to the statusline ([#3165](https://github.com/helix-editor/helix/pull/3165), [255c173](https://github.com/helix-editor/helix/commit/255c173))
- Make gutters padding automatic ([#3163](https://github.com/helix-editor/helix/pull/3163))
- Add `code` for LSP `Diagnostic` type ([#3096](https://github.com/helix-editor/helix/pull/3096))
- Add position percentage to the statusline ([#3168](https://github.com/helix-editor/helix/pull/3168))
- Add a configurable and themable statusline separator string ([#3175](https://github.com/helix-editor/helix/pull/3175))
- Use OR of all selections when `search_selection` acts on multiple selections ([#3138](https://github.com/helix-editor/helix/pull/3138))
- Add clipboard information to logs and the healthcheck ([#3271](https://github.com/helix-editor/helix/pull/3271))
- Fix align selection behavior on tabs ([#3276](https://github.com/helix-editor/helix/pull/3276))
- Fix terminal cursor shape reset ([#3289](https://github.com/helix-editor/helix/pull/3289))
- Add an `injection.include-unnamed-children` predicate to injections queries ([#3129](https://github.com/helix-editor/helix/pull/3129))
- Add a `-c`/`--config` CLI flag for specifying config file location ([#2666](https://github.com/helix-editor/helix/pull/2666))
- Detect indent-style in `:set-language` command ([#3330](https://github.com/helix-editor/helix/pull/3330))
- Fix non-deterministic highlighting ([#3275](https://github.com/helix-editor/helix/pull/3275))
- Avoid setting the stdin handle when not necessary ([#3248](https://github.com/helix-editor/helix/pull/3248), [#3379](https://github.com/helix-editor/helix/pull/3379))
- Fix indent guide styling ([#3324](https://github.com/helix-editor/helix/pull/3324))
- Fix tab highlight when tab is partially visible ([#3313](https://github.com/helix-editor/helix/pull/3313))
- Add completion for nested settings ([#3183](https://github.com/helix-editor/helix/pull/3183))
- Advertise WorkspaceSymbolClientCapabilities LSP client capability ([#3361](https://github.com/helix-editor/helix/pull/3361))
- Remove duplicate entries from the theme picker ([#3439](https://github.com/helix-editor/helix/pull/3439))
- Shorted output for grammar fetching and building ([#3396](https://github.com/helix-editor/helix/pull/3396))
- Add a `tabpad` option for visible tab padding whitespace characters ([#3458](https://github.com/helix-editor/helix/pull/3458))
- Make DAP external terminal provider configurable ([cb7615e](https://github.com/helix-editor/helix/commit/cb7615e))
- Use health checkmark character with shorter width ([#3505](https://github.com/helix-editor/helix/pull/3505))
- Reset document mode to normal on view focus loss ([e4c9d40](https://github.com/helix-editor/helix/commit/e4c9d40))
- Render indented code-blocks in markdown ([#3503](https://github.com/helix-editor/helix/pull/3503))
- Add WezTerm to DAP terminal provider defaults ([#3588](https://github.com/helix-editor/helix/pull/3588))
- Derive `Document` language name from `languages.toml` `name` key ([#3338](https://github.com/helix-editor/helix/pull/3338))
- Fix process spawning error handling ([#3349](https://github.com/helix-editor/helix/pull/3349))
- Don't resolve links for `:o` completion ([8a4fbf6](https://github.com/helix-editor/helix/commit/8a4fbf6))
- Recalculate completion after pasting into prompt ([e77b7d1](https://github.com/helix-editor/helix/commit/e77b7d1))
- Fix extra selections with regex anchors ([#3598](https://github.com/helix-editor/helix/pull/3598))
- Move mode transition logic to `handle_keymap_event` ([#2634](https://github.com/helix-editor/helix/pull/2634))
- Add documents to view history when using the jumplist ([#3593](https://github.com/helix-editor/helix/pull/3593))
- Prevent panic when loading tree-sitter queries ([fa1dc7e](https://github.com/helix-editor/helix/commit/fa1dc7e))
- Discard LSP publishDiagnostic when LS is not initialized ([#3403](https://github.com/helix-editor/helix/pull/3403))
- Refactor tree-sitter textobject motions as repeatable motions ([#3264](https://github.com/helix-editor/helix/pull/3264))
- Avoid command execution hooks on closed docs ([#3613](https://github.com/helix-editor/helix/pull/3613))
- Share `restore_term` code between panic and normal exits ([#2612](https://github.com/helix-editor/helix/pull/2612))
- Show clipboard info in `--health` output ([#2947](https://github.com/helix-editor/helix/pull/2947))
- Recalculate completion when going through prompt history ([#3193](https://github.com/helix-editor/helix/pull/3193))

Themes:

- Update `tokyonight` and `tokyonight_storm` themes ([#2606](https://github.com/helix-editor/helix/pull/2606))
- Update `solarized_light` themes ([#2626](https://github.com/helix-editor/helix/pull/2626))
- Fix `catpuccin` `ui.popup` theme ([#2644](https://github.com/helix-editor/helix/pull/2644))
- Update selection style of `night_owl` ([#2668](https://github.com/helix-editor/helix/pull/2668))
- Fix spelling of `catppuccin` theme ([#2713](https://github.com/helix-editor/helix/pull/2713))
- Update `base16_default`'s `ui.menu` ([#2794](https://github.com/helix-editor/helix/pull/2794))
- Add `noctis_bordo` ([#2830](https://github.com/helix-editor/helix/pull/2830))
- Add `acme` ([#2876](https://github.com/helix-editor/helix/pull/2876))
- Add `meliora` ([#2884](https://github.com/helix-editor/helix/pull/2884), [#2890](https://github.com/helix-editor/helix/pull/2890))
- Add cursorline scopes to various themes ([33d287a](https://github.com/helix-editor/helix/commit/33d287a), [#2892](https://github.com/helix-editor/helix/pull/2892), [#2915](https://github.com/helix-editor/helix/pull/2915), [#2916](https://github.com/helix-editor/helix/pull/2916), [#2918](https://github.com/helix-editor/helix/pull/2918), [#2927](https://github.com/helix-editor/helix/pull/2927), [#2925](https://github.com/helix-editor/helix/pull/2925), [#2938](https://github.com/helix-editor/helix/pull/2938), [#2962](https://github.com/helix-editor/helix/pull/2962), [#3054](https://github.com/helix-editor/helix/pull/3054))
- Add mode colors to various themes ([#2926](https://github.com/helix-editor/helix/pull/2926), [#2933](https://github.com/helix-editor/helix/pull/2933), [#2929](https://github.com/helix-editor/helix/pull/2929), [#3098](https://github.com/helix-editor/helix/pull/3098), [#3104](https://github.com/helix-editor/helix/pull/3104), [#3128](https://github.com/helix-editor/helix/pull/3128), [#3135](https://github.com/helix-editor/helix/pull/3135), [#3200](https://github.com/helix-editor/helix/pull/3200))
- Add `nord_light` ([#2908](https://github.com/helix-editor/helix/pull/2908))
- Update `night_owl` ([#2929](https://github.com/helix-editor/helix/pull/2929))
- Update `autumn` ([2e70985](https://github.com/helix-editor/helix/commit/2e70985), [936ed3a](https://github.com/helix-editor/helix/commit/936ed3a))
- Update `one_dark` ([#3011](https://github.com/helix-editor/helix/pull/3011))
- Add `noctis` ([#3043](https://github.com/helix-editor/helix/pull/3043), [#3128](https://github.com/helix-editor/helix/pull/3128))
- Update `boo_berry` ([#3191](https://github.com/helix-editor/helix/pull/3191))
- Update `monokai` ([#3131](https://github.com/helix-editor/helix/pull/3131))
- Add `ayu_dark`, `ayu_light`, `ayu_mirage` ([#3184](https://github.com/helix-editor/helix/pull/3184))
- Update `onelight` ([#3226](https://github.com/helix-editor/helix/pull/3226))
- Add `base16_transparent` ([#3216](https://github.com/helix-editor/helix/pull/3216), [b565fff](https://github.com/helix-editor/helix/commit/b565fff))
- Add `flatwhite` ([#3236](https://github.com/helix-editor/helix/pull/3236))
- Update `dark_plus` ([#3302](https://github.com/helix-editor/helix/pull/3302))
- Add `doom_acario_dark` ([#3308](https://github.com/helix-editor/helix/pull/3308), [#3539](https://github.com/helix-editor/helix/pull/3539))
- Add `rose_pine_moon` ([#3229](https://github.com/helix-editor/helix/pull/3229))
- Update `spacebones_light` ([#3342](https://github.com/helix-editor/helix/pull/3342))
- Fix typos in themes ([8deaebd](https://github.com/helix-editor/helix/commit/8deaebd), [#3412](https://github.com/helix-editor/helix/pull/3412))
- Add `emacs` ([#3410](https://github.com/helix-editor/helix/pull/3410))
- Add `papercolor-light` ([#3426](https://github.com/helix-editor/helix/pull/3426), [#3470](https://github.com/helix-editor/helix/pull/3470), [#3585](https://github.com/helix-editor/helix/pull/3585))
- Add `penumbra+` ([#3398](https://github.com/helix-editor/helix/pull/3398))
- Add `fleetish` ([#3591](https://github.com/helix-editor/helix/pull/3591), [#3607](https://github.com/helix-editor/helix/pull/3607))
- Add `sonokai` ([#3595](https://github.com/helix-editor/helix/pull/3595))
- Update all themes for theme lints ([#3587](https://github.com/helix-editor/helix/pull/3587))

LSP:

- V ([#2526](https://github.com/helix-editor/helix/pull/2526))
- Prisma ([#2703](https://github.com/helix-editor/helix/pull/2703))
- Clojure ([#2780](https://github.com/helix-editor/helix/pull/2780))
- WGSL ([#2872](https://github.com/helix-editor/helix/pull/2872))
- Elvish ([#2948](https://github.com/helix-editor/helix/pull/2948))
- Idris ([#2971](https://github.com/helix-editor/helix/pull/2971))
- Fortran ([#3025](https://github.com/helix-editor/helix/pull/3025))
- Gleam ([#3139](https://github.com/helix-editor/helix/pull/3139))
- Odin ([#3214](https://github.com/helix-editor/helix/pull/3214))

New languages:

- V ([#2526](https://github.com/helix-editor/helix/pull/2526))
- EDoc ([#2640](https://github.com/helix-editor/helix/pull/2640))
- JSDoc ([#2650](https://github.com/helix-editor/helix/pull/2650))
- OpenSCAD ([#2680](https://github.com/helix-editor/helix/pull/2680))
- Prisma ([#2703](https://github.com/helix-editor/helix/pull/2703))
- Clojure ([#2780](https://github.com/helix-editor/helix/pull/2780))
- Starlark ([#2903](https://github.com/helix-editor/helix/pull/2903))
- Elvish ([#2948](https://github.com/helix-editor/helix/pull/2948))
- Fortran ([#3025](https://github.com/helix-editor/helix/pull/3025))
- Ungrammar ([#3048](https://github.com/helix-editor/helix/pull/3048))
- SCSS ([#3074](https://github.com/helix-editor/helix/pull/3074))
- Go Template ([#3091](https://github.com/helix-editor/helix/pull/3091))
- Graphviz dot ([#3241](https://github.com/helix-editor/helix/pull/3241))
- Cue ([#3262](https://github.com/helix-editor/helix/pull/3262))
- Slint ([#3355](https://github.com/helix-editor/helix/pull/3355))
- Beancount ([#3297](https://github.com/helix-editor/helix/pull/3297))
- Taskwarrior ([#3468](https://github.com/helix-editor/helix/pull/3468))
- xit ([#3521](https://github.com/helix-editor/helix/pull/3521))
- ESDL ([#3526](https://github.com/helix-editor/helix/pull/3526))
- Awk ([#3528](https://github.com/helix-editor/helix/pull/3528), [#3535](https://github.com/helix-editor/helix/pull/3535))
- Pascal ([#3542](https://github.com/helix-editor/helix/pull/3542))

Updated languages and queries:

- Nix ([#2472](https://github.com/helix-editor/helix/pull/2472))
- Elixir ([#2619](https://github.com/helix-editor/helix/pull/2619))
- CPON ([#2643](https://github.com/helix-editor/helix/pull/2643))
- Textobjects queries for Erlang, Elixir, Gleam ([#2661](https://github.com/helix-editor/helix/pull/2661))
- Capture rust closures as function textobjects ([4a27e2d](https://github.com/helix-editor/helix/commit/4a27e2d))
- Heex ([#2800](https://github.com/helix-editor/helix/pull/2800), [#3170](https://github.com/helix-editor/helix/pull/3170))
- Add `<<=` operator highlighting for Rust ([#2805](https://github.com/helix-editor/helix/pull/2805))
- Fix comment injection in JavaScript/TypeScript ([#2763](https://github.com/helix-editor/helix/pull/2763))
- Nickel ([#2859](https://github.com/helix-editor/helix/pull/2859))
- Add `Rakefile` and `Gemfile` to Ruby file-types ([#2875](https://github.com/helix-editor/helix/pull/2875))
- Erlang ([#2910](https://github.com/helix-editor/helix/pull/2910), [ac669ad](https://github.com/helix-editor/helix/commit/ac669ad))
- Markdown ([#2910](https://github.com/helix-editor/helix/pull/2910), [#3108](https://github.com/helix-editor/helix/pull/3108), [#3400](https://github.com/helix-editor/helix/pull/3400))
- Bash ([#2910](https://github.com/helix-editor/helix/pull/2910))
- Rust ([#2910](https://github.com/helix-editor/helix/pull/2910), [#3397](https://github.com/helix-editor/helix/pull/3397))
- Edoc ([#2910](https://github.com/helix-editor/helix/pull/2910))
- HTML ([#2910](https://github.com/helix-editor/helix/pull/2910))
- Make ([#2910](https://github.com/helix-editor/helix/pull/2910))
- TSQ ([#2910](https://github.com/helix-editor/helix/pull/2910), [#2960](https://github.com/helix-editor/helix/pull/2960))
- git-commit ([#2910](https://github.com/helix-editor/helix/pull/2910))
- Use default fallback for Python indents ([9ae70cc](https://github.com/helix-editor/helix/commit/9ae70cc))
- Add Haskell LSP roots ([#2954](https://github.com/helix-editor/helix/pull/2954))
- Ledger ([#2936](https://github.com/helix-editor/helix/pull/2936), [#2988](https://github.com/helix-editor/helix/pull/2988))
- Nickel ([#2987](https://github.com/helix-editor/helix/pull/2987))
- JavaScript/TypeScript ([#2961](https://github.com/helix-editor/helix/pull/2961), [#3219](https://github.com/helix-editor/helix/pull/3219), [#3213](https://github.com/helix-editor/helix/pull/3213), [#3280](https://github.com/helix-editor/helix/pull/3280), [#3301](https://github.com/helix-editor/helix/pull/3301))
- GLSL ([#3051](https://github.com/helix-editor/helix/pull/3051))
- Fix locals tracking in Rust ([#3027](https://github.com/helix-editor/helix/pull/3027), [#3212](https://github.com/helix-editor/helix/pull/3212), [#3345](https://github.com/helix-editor/helix/pull/3345))
- Verilog ([#3158](https://github.com/helix-editor/helix/pull/3158))
- Ruby ([#3173](https://github.com/helix-editor/helix/pull/3173), [#3527](https://github.com/helix-editor/helix/pull/3527))
- Svelte ([#3147](https://github.com/helix-editor/helix/pull/3147))
- Add Elixir and HEEx comment textobjects ([#3179](https://github.com/helix-editor/helix/pull/3179))
- Python ([#3103](https://github.com/helix-editor/helix/pull/3103), [#3201](https://github.com/helix-editor/helix/pull/3201), [#3284](https://github.com/helix-editor/helix/pull/3284))
- PHP ([#3317](https://github.com/helix-editor/helix/pull/3317))
- Latex ([#3370](https://github.com/helix-editor/helix/pull/3370))
- Clojure ([#3387](https://github.com/helix-editor/helix/pull/3387))
- Swift ([#3461](https://github.com/helix-editor/helix/pull/3461))
- C# ([#3480](https://github.com/helix-editor/helix/pull/3480), [#3494](https://github.com/helix-editor/helix/pull/3494))
- Org ([#3489](https://github.com/helix-editor/helix/pull/3489))
- Elm ([#3497](https://github.com/helix-editor/helix/pull/3497))
- Dart ([#3419](https://github.com/helix-editor/helix/pull/3419))
- Julia ([#3507](https://github.com/helix-editor/helix/pull/3507))
- Fix Rust textobjects ([#3590](https://github.com/helix-editor/helix/pull/3590))
- C ([00d88e5](https://github.com/helix-editor/helix/commit/00d88e5))
- Update Rust ([0ef0ef9](https://github.com/helix-editor/helix/commit/0ef0ef9))

Packaging:

- Add `rust-analyzer` to Nix flake devShell ([#2739](https://github.com/helix-editor/helix/pull/2739))
- Add cachix information to the Nix flake ([#2999](https://github.com/helix-editor/helix/pull/2999))
- Pass makeWrapperArgs to wrapProgram in the Nix flake ([#3003](https://github.com/helix-editor/helix/pull/3003))
- Add a way to override which grammars are built by Nix ([#3141](https://github.com/helix-editor/helix/pull/3141))
- Add a GitHub actions release for `aarch64-macos` ([#3137](https://github.com/helix-editor/helix/pull/3137))
- Add shell auto-completions for Elvish ([#3331](https://github.com/helix-editor/helix/pull/3331))

# 22.05 (2022-05-28)

An even bigger shout out than usual to all the contributors - we had a whopping
110 contributors in this release! That's more than double the number of
contributors as last release!

Check out some of the highlights in the [news section](https://helix-editor.com/news/release-22-05-highlights/).

As usual, the following is a summary of each of the changes since the last release.
For the full log, check out the [git log](https://github.com/helix-editor/helix/compare/22.03..22.05).

Breaking Changes:

- Removed `C-j`, `C-k` bindings from file picker ([#1792](https://github.com/helix-editor/helix/pull/1792))
- Replaced `C-f` with `C-d` and `C-b` with `C-u` bindings in file picker ([#1792](https://github.com/helix-editor/helix/pull/1792))
- `A-hjkl` bindings have been moved to `A-pion` ([#2205](https://github.com/helix-editor/helix/pull/2205))
- `A-Left`/`A-Right` have been moved to `C-Left`/`C-Right` ([#2193](https://github.com/helix-editor/helix/pull/2193))

Features:

- The indentation mechanism has been reworked ([#1562](https://github.com/helix-editor/helix/pull/1562), [#1908](https://github.com/helix-editor/helix/pull/1908))
- Configurable gutters ([#1967](https://github.com/helix-editor/helix/pull/1967))
- Support for local language configuration ([#1249](https://github.com/helix-editor/helix/pull/1249))
- Configurable themed rulers ([#2060](https://github.com/helix-editor/helix/pull/2060))
- Render visible whitespace ([e6b865e](https://github.com/helix-editor/helix/commit/e6b865e), [#2322](https://github.com/helix-editor/helix/pull/2322), [#2331](https://github.com/helix-editor/helix/pull/2331))

Commands:

- Paragraph motion and textobject (`]p`, `[p`) ([#1627](https://github.com/helix-editor/helix/pull/1627), [#1956](https://github.com/helix-editor/helix/pull/1956), [#1969](https://github.com/helix-editor/helix/pull/1969), [#1992](https://github.com/helix-editor/helix/pull/1992), [#2226](https://github.com/helix-editor/helix/pull/2226))
- `:buffer-next`, `:buffer-previous` ([#1940](https://github.com/helix-editor/helix/pull/1940))
- `:set-language` to set the buffers language ([#1866](https://github.com/helix-editor/helix/pull/1866), [#1996](https://github.com/helix-editor/helix/pull/1996))
- Command for picking files from the current working directory (`Space-F`) ([#1600](https://github.com/helix-editor/helix/pull/1600), [#2308](https://github.com/helix-editor/helix/pull/2308))
- `:write!` which creates non-existent subdirectories ([#1839](https://github.com/helix-editor/helix/pull/1839))
- Add `m` textobject that selects closest surrounding pair ([de15d70](https://github.com/helix-editor/helix/commit/de15d70), [76175db](https://github.com/helix-editor/helix/commit/76175db))
- `:pipe` typable command for piping selections ([#1972](https://github.com/helix-editor/helix/pull/1972))
- `extend_line_above` which extends to previous lines ([#2117](https://github.com/helix-editor/helix/pull/2117))
- `set_line_ending` which replaces line endings ([#1871](https://github.com/helix-editor/helix/pull/1871))
- `:get-option` for getting the current value of an option (`:get`) ([#2231](https://github.com/helix-editor/helix/pull/2231))
- `:run-shell-command` which does not interact with selections ([#1682](https://github.com/helix-editor/helix/pull/1682))
- `:reflow` which hard-wraps selected text ([#2128](https://github.com/helix-editor/helix/pull/2128))
- `commit_undo_checkpoint` which adds an undo checkpoint ([#2115](https://github.com/helix-editor/helix/pull/2115))
- `:log-open` which opens the log file ([#2422](https://github.com/helix-editor/helix/pull/2422))
- `transpose_view` which transposes window splits ([#2461](https://github.com/helix-editor/helix/pull/2461))
- View-swapping: `swap_view_right`, `swap_view_left`, `swap_view_up`, `swap_view_down` ([#2445](https://github.com/helix-editor/helix/pull/2445))
- `shrink_to_line_bounds` which shrinks selections to line-bounds ([#2450](https://github.com/helix-editor/helix/pull/2450))

Usability improvements and fixes:

- Handle broken pipes when piping `hx --health` through `head` ([#1876](https://github.com/helix-editor/helix/pull/1876))
- Fix for `copy_selection` on newlines ([ab7885e](https://github.com/helix-editor/helix/commit/ab7885e), [236c6b7](https://github.com/helix-editor/helix/commit/236c6b7))
- Use `win32yank` clipboard provider on WSL2 ([#1912](https://github.com/helix-editor/helix/pull/1912))
- Jump to the next number on the line before incrementing ([#1778](https://github.com/helix-editor/helix/pull/1778))
- Fix start position of next search ([#1904](https://github.com/helix-editor/helix/pull/1904))
- Use check and X marks for health check output ([#1918](https://github.com/helix-editor/helix/pull/1918))
- Clear terminal after switching to alternate screens ([#1944](https://github.com/helix-editor/helix/pull/1944))
- Fix `toggle_comments` command on multiple selections ([#1882](https://github.com/helix-editor/helix/pull/1882))
- Apply `ui.gutter` theming to empty gutter spans ([#2032](https://github.com/helix-editor/helix/pull/2032))
- Use checkboxes in `hx --health` output ([#1947](https://github.com/helix-editor/helix/pull/1947))
- Pass unmapped keys through prompt regardless of modifiers ([764adbd](https://github.com/helix-editor/helix/commit/764adbd))
- LSP: pull formatting options from config ([c18de0e](https://github.com/helix-editor/helix/commit/c18de0e))
- LSP: provide `rootPath` ([84e799f](https://github.com/helix-editor/helix/commit/84e799f))
- LSP: implement `workspace_folders` ([8adf0c1](https://github.com/helix-editor/helix/commit/8adf0c1))
- LSP: fix auto-import ([#2088](https://github.com/helix-editor/helix/pull/2088))
- Send active diagnostic to LSP when requesting code actions ([#2005](https://github.com/helix-editor/helix/pull/2005))
- Prevent panic when parsing malformed LSP `PublishDiagnostic` ([#2160](https://github.com/helix-editor/helix/pull/2160))
- Restore document state on completion cancel ([#2096](https://github.com/helix-editor/helix/pull/2096))
- Only merge top-level array when merging `languages.toml` ([#2145](https://github.com/helix-editor/helix/pull/2145), [#2215](https://github.com/helix-editor/helix/pull/2215))
- Fix open on multiline selection ([#2161](https://github.com/helix-editor/helix/pull/2161))
- Allow re-binding `0` if it is not used in a count ([#2174](https://github.com/helix-editor/helix/pull/2174))
- Fix `ctrl-u` behavior in insert mode ([#1957](https://github.com/helix-editor/helix/pull/1957))
- Check LSP rename capabilities before sending rename action ([#2203](https://github.com/helix-editor/helix/pull/2203))
- Register the `publish_diagnostics` LSP capability ([#2241](https://github.com/helix-editor/helix/pull/2241))
- Fix paste direction for typed paste commands ([#2288](https://github.com/helix-editor/helix/pull/2288))
- Improve handling of buffer-close ([#1397](https://github.com/helix-editor/helix/pull/1397))
- Extend the tutor file ([#2133](https://github.com/helix-editor/helix/pull/2133))
- Treat slashes as word separators in prompts ([#2315](https://github.com/helix-editor/helix/pull/2315))
- Auto-complete directory members ([#1682](https://github.com/helix-editor/helix/pull/1682))
- Allow disabling format-on-save as a global editor setting ([#2321](https://github.com/helix-editor/helix/pull/2321))
- Wrap command palette in overlay ([#2378](https://github.com/helix-editor/helix/pull/2378))
- Prevent selections from collapsing when inserting newlines ([#2414](https://github.com/helix-editor/helix/pull/2414))
- Allow configuration of LSP request timeout ([#2405](https://github.com/helix-editor/helix/pull/2405))
- Use debug console on Windows for DAP terminal ([#2294](https://github.com/helix-editor/helix/pull/2294))
- Exclude cursor when deleting with `C-w` in insert mode ([#2431](https://github.com/helix-editor/helix/pull/2431))
- Prevent panics from LSP parsing errors ([7ae6cad](https://github.com/helix-editor/helix/commit/7ae6cad))
- Prevent panics from LSP responses without requests ([#2475](https://github.com/helix-editor/helix/pull/2475))
- Fix scroll rate for documentation popups ([#2497](https://github.com/helix-editor/helix/pull/2497))
- Support inserting into prompts from registers ([#2458](https://github.com/helix-editor/helix/pull/2458))
- Separate theme scopes for diagnostic types ([#2437](https://github.com/helix-editor/helix/pull/2437))
- Use `ui.menu` instead of `ui.statusline` for command completion menu theming ([82fb217](https://github.com/helix-editor/helix/commit/82fb217))
- Panic when reloading a shrunk file ([#2506](https://github.com/helix-editor/helix/pull/2506))
- Add theme key for picker separator ([#2523](https://github.com/helix-editor/helix/pull/2523))

Themes:

- Remove `ui.text` background from dark_plus ([#1950](https://github.com/helix-editor/helix/pull/1950))
- Add `boo_berry` ([#1962](https://github.com/helix-editor/helix/pull/1962))
- Update `dark_plus` markup colors ([#1989](https://github.com/helix-editor/helix/pull/1989))
- Update `dark_plus` `tag` and `ui.menu.selected` colors ([#2014](https://github.com/helix-editor/helix/pull/2014))
- Add `dracula_at_night` ([#2008](https://github.com/helix-editor/helix/pull/2008))
- Improve `dracula` selection theming ([#2077](https://github.com/helix-editor/helix/pull/2077))
- Remove dim attribute on `onedark` line-number gutter ([#2155](https://github.com/helix-editor/helix/pull/2155))
- Add `tokyonight` ([#2162](https://github.com/helix-editor/helix/pull/2162))
- Use border colors from the original `dark_plus` theme ([#2186](https://github.com/helix-editor/helix/pull/2186))
- Add `autumn` ([#2212](https://github.com/helix-editor/helix/pull/2212), [#2270](https://github.com/helix-editor/helix/pull/2270), [#2531](https://github.com/helix-editor/helix/pull/2531))
- Add `tokyonight_storm` ([#2240](https://github.com/helix-editor/helix/pull/2240))
- Add `pop-dark` ([#2189](https://github.com/helix-editor/helix/pull/2189))
- Fix `base16_terminal` theme using incorrect ansi-color ([#2279](https://github.com/helix-editor/helix/pull/2279))
- Add `onelight` ([#2287](https://github.com/helix-editor/helix/pull/2287), [#2323](https://github.com/helix-editor/helix/pull/2323))
- Add `ui.virtual` scopes to `onedark` theme ([3626e38](https://github.com/helix-editor/helix/commit/3626e38))
- Add `night_owl` ([#2330](https://github.com/helix-editor/helix/pull/2330))
- Use yellow foreground and red background for `monokai_pro_spectrum` ([#2433](https://github.com/helix-editor/helix/pull/2433))
- Add `snazzy` ([#2473](https://github.com/helix-editor/helix/pull/2473))
- Update `dark_plus` constructor color ([8e8d4ba](https://github.com/helix-editor/helix/commit/8e8d4ba))
- Add `ui.menu` to the default theme ([e7e13dc](https://github.com/helix-editor/helix/commit/e7e13dc))
- Add `ui.menu` to any themes missing the key ([9be810f](https://github.com/helix-editor/helix/commit/9be810f))
- Add `catppuccin` ([#2546](https://github.com/helix-editor/helix/pull/2546), [7160e74](https://github.com/helix-editor/helix/commit/7160e74))

LSP:

- Use texlab for latex ([#1922](https://github.com/helix-editor/helix/pull/1922))
- HTML ([#2018](https://github.com/helix-editor/helix/pull/2018))
- JSON ([#2024](https://github.com/helix-editor/helix/pull/2024))
- CSS ([#2025](https://github.com/helix-editor/helix/pull/2025))
- PHP ([#2031](https://github.com/helix-editor/helix/pull/2031))
- Swift ([#2033](https://github.com/helix-editor/helix/pull/2033))
- OCaml ([#2035](https://github.com/helix-editor/helix/pull/2035))
- Vue ([#2043](https://github.com/helix-editor/helix/pull/2043))
- Yaml ([#2234](https://github.com/helix-editor/helix/pull/2234))
- Vala ([#2243](https://github.com/helix-editor/helix/pull/2243))
- TOML ([#2302](https://github.com/helix-editor/helix/pull/2302))
- Java ([#2511](https://github.com/helix-editor/helix/pull/2511))
- Lua ([#2560](https://github.com/helix-editor/helix/pull/2560))
- Verilog ([#2552](https://github.com/helix-editor/helix/pull/2552))

New Languages:

- JSX ([#1906](https://github.com/helix-editor/helix/pull/1906), [a24fb17](https://github.com/helix-editor/helix/commit/a24fb17), [855e438](https://github.com/helix-editor/helix/commit/855e438), [#1921](https://github.com/helix-editor/helix/pull/1921))
- Rust Object Notation (RON) ([#1925](https://github.com/helix-editor/helix/pull/1925))
- R and R Markdown ([#1998](https://github.com/helix-editor/helix/pull/1998))
- Swift ([#2033](https://github.com/helix-editor/helix/pull/2033))
- EJS and ERB ([#2055](https://github.com/helix-editor/helix/pull/2055))
- EEx ([9d095e0](https://github.com/helix-editor/helix/commit/9d095e0))
- HEEx ([4836bb3](https://github.com/helix-editor/helix/commit/4836bb3), [#2149](https://github.com/helix-editor/helix/pull/2149))
- SQL ([#2097](https://github.com/helix-editor/helix/pull/2097))
- GDScript ([#1985](https://github.com/helix-editor/helix/pull/1985))
- Nickel ([#2173](https://github.com/helix-editor/helix/pull/2173), [#2320](https://github.com/helix-editor/helix/pull/2320))
- `go.mod` and `go.work` ([#2197](https://github.com/helix-editor/helix/pull/2197))
- Nushell ([#2225](https://github.com/helix-editor/helix/pull/2225))
- Vala ([#2243](https://github.com/helix-editor/helix/pull/2243))
- Hare ([#2289](https://github.com/helix-editor/helix/pull/2289), [#2480](https://github.com/helix-editor/helix/pull/2480))
- DeviceTree ([#2329](https://github.com/helix-editor/helix/pull/2329))
- Cairo ([7387905](https://github.com/helix-editor/helix/commit/7387905))
- CPON ([#2355](https://github.com/helix-editor/helix/pull/2355), [#2424](https://github.com/helix-editor/helix/pull/2424))
- git-ignore ([#2397](https://github.com/helix-editor/helix/pull/2397))
- git-attributes ([#2397](https://github.com/helix-editor/helix/pull/2397))
- Odin ([#2399](https://github.com/helix-editor/helix/pull/2399), [#2464](https://github.com/helix-editor/helix/pull/2464))
- Meson ([#2314](https://github.com/helix-editor/helix/pull/2314))
- SSH Client Config ([#2498](https://github.com/helix-editor/helix/pull/2498))
- Scheme ([d25bae8](https://github.com/helix-editor/helix/commit/d25bae8))
- Verilog ([#2552](https://github.com/helix-editor/helix/pull/2552))

Updated Languages and Queries:

- Erlang ([e2a5071](https://github.com/helix-editor/helix/commit/e2a5071), [#2149](https://github.com/helix-editor/helix/pull/2149), [82da9bd](https://github.com/helix-editor/helix/commit/82da9bd))
- Elixir ([1819478](https://github.com/helix-editor/helix/commit/1819478), [8c3c901](https://github.com/helix-editor/helix/commit/8c3c901), [4ac94a5](https://github.com/helix-editor/helix/commit/4ac94a5))
- Gleam ([7cd6050](https://github.com/helix-editor/helix/commit/7cd6050), [45dd540](https://github.com/helix-editor/helix/commit/45dd540))
- Bash ([#1917](https://github.com/helix-editor/helix/pull/1917))
- JavaScript ([#2140](https://github.com/helix-editor/helix/pull/2140))
- Ruby textobject queries ([#2143](https://github.com/helix-editor/helix/pull/2143))
- Fix Golang textobject queries ([#2153](https://github.com/helix-editor/helix/pull/2153))
- Add more bash and HCL file extensions ([#2201](https://github.com/helix-editor/helix/pull/2201))
- Divide HCL and tfvars into separate languages ([#2244](https://github.com/helix-editor/helix/pull/2244))
- Use JavaScript for `cjs` files ([#2387](https://github.com/helix-editor/helix/pull/2387))
- Use Perl for `t` files ([#2395](https://github.com/helix-editor/helix/pull/2395))
- Use `markup.list` scopes for lists ([#2401](https://github.com/helix-editor/helix/pull/2401))
- Use PHP for `inc` files ([#2440](https://github.com/helix-editor/helix/pull/2440))
- Improve Rust textobjects ([#2494](https://github.com/helix-editor/helix/pull/2494), [10463fe](https://github.com/helix-editor/helix/commit/10463fe))
- Python ([#2451](https://github.com/helix-editor/helix/pull/2451))

Packaging:

- Use `builtins.fromTOML` in Nix Flake on Nix 2.6+ ([#1892](https://github.com/helix-editor/helix/pull/1892))
- Shell auto-completion files are now available ([#2022](https://github.com/helix-editor/helix/pull/2022))
- Create an AppImage on release ([#2089](https://github.com/helix-editor/helix/pull/2089))

# 22.03 (2022-03-28)

A big shout out to all the contributors! We had 51 contributors in this release.

This release is particularly large and featureful. Check out some of the
highlights in the [news section](https://helix-editor.com/news/release-22-03-highlights/).

As usual, the following is a summary of each of the changes since the last release.
For the full log, check out the [git log](https://github.com/helix-editor/helix/compare/v0.6.0..22.03).

Breaking changes:

- LSP config now lives under `editor.lsp` ([#1868](https://github.com/helix-editor/helix/pull/1868))
- Expand-selection was moved from `]o` to `Alt-h` ([#1495](https://github.com/helix-editor/helix/pull/1495))

Features:

- Experimental Debug Adapter Protocol (DAP) support ([#574](https://github.com/helix-editor/helix/pull/574))
- Primary cursor shape may now be customized per mode ([#1154](https://github.com/helix-editor/helix/pull/1154))
- Overhaul incremental highlights and enable combined injections ([`6728344..4080341`](https://github.com/helix-editor/helix/compare/6728344..4080341))
- Allow specifying file start position ([#445](https://github.com/helix-editor/helix/pull/445), [#1676](https://github.com/helix-editor/helix/pull/1676))
- Dynamic line numbers ([#1522](https://github.com/helix-editor/helix/pull/1522))
- Show an info box with the contents of registers ([#980](https://github.com/helix-editor/helix/pull/980))
- Wrap-around behavior during search is now configurable ([#1516](https://github.com/helix-editor/helix/pull/1516))
- Tree-sitter textobjects motions for classes, functions, and parameters ([#1619](https://github.com/helix-editor/helix/pull/1619), [#1708](https://github.com/helix-editor/helix/pull/1708), [#1805](https://github.com/helix-editor/helix/pull/1805))
- Command palette: a picker for available commands ([#1400](https://github.com/helix-editor/helix/pull/1400))
- LSP `workspace/configuration` and `workspace/didChangeConfiguration` support ([#1684](https://github.com/helix-editor/helix/pull/1684))
- `hx --health [LANG]` command ([#1669](https://github.com/helix-editor/helix/pull/1669))
- Refactor of the tree-sitter grammar system ([#1659](https://github.com/helix-editor/helix/pull/1659))
  - All submodules have been removed
  - New `hx --grammar {fetch|build}` flags for fetching and building tree-sitter grammars
  - A custom grammar selection may now be declared with the `use-grammars` key in `languages.toml`

Commands:

- `:cquit!` - quit forcefully with a non-zero exit-code ([#1414](https://github.com/helix-editor/helix/pull/1414))
- `shrink_selection` - shrink the selection to a child tree-sitter node (`Alt-j`, [#1340](https://github.com/helix-editor/helix/pull/1340))
- `:tree-sitter-subtree` - show the tree-sitter subtree under the primary selection ([#1453](https://github.com/helix-editor/helix/pull/1453), [#1524](https://github.com/helix-editor/helix/pull/1524))
- Add `Alt-Backspace`, `Alt-<`, `Alt->`, and `Ctrl-j` to insert mode ([#1441](https://github.com/helix-editor/helix/pull/1441))
- `select_next_sibling`, `select_prev_sibling` - select next and previous tree-sitter nodes (`Alt-l` and `Alt-h`, [#1495](https://github.com/helix-editor/helix/pull/1495))
- `:buffer-close-all`, `:buffer-close-all!`, `:buffer-close-others`, and `:buffer-close-others!` ([#1677](https://github.com/helix-editor/helix/pull/1677))
- `:vsplit-new` and `:hsplit-new` - open vertical and horizontal splits with new scratch buffers ([#1763](https://github.com/helix-editor/helix/pull/1763))
- `:open-config` to open the config file and `:refresh-config` to refresh config after changes ([#1771](https://github.com/helix-editor/helix/pull/1771), [#1803](https://github.com/helix-editor/helix/pull/1803))

Usability improvements and fixes:

- Prevent `:cquit` from ignoring unsaved changes ([#1414](https://github.com/helix-editor/helix/pull/1414))
- Scrolling view keeps selections ([#1420](https://github.com/helix-editor/helix/pull/1420))
- Only use shellwords parsing on unix platforms ([`7767703`](https://github.com/helix-editor/helix/commit/7767703))
- Fix slash in search selector status message ([#1449](https://github.com/helix-editor/helix/pull/1449))
- Use `std::path::MAIN_SEPARATOR` to determine completion ([`3e4f815`](https://github.com/helix-editor/helix/commit/3e4f815))
- Expand to current node with `expand_selection` when the node has no children ([#1454](https://github.com/helix-editor/helix/pull/1454))
- Add vertical and horizontal splits to the buffer picker ([#1502](https://github.com/helix-editor/helix/pull/1502))
- Use the correct language ID for JavaScript & TypeScript LSP ([#1466](https://github.com/helix-editor/helix/pull/1466))
- Run format command for all buffers being written ([#1444](https://github.com/helix-editor/helix/pull/1444))
- Fix panics during resizing ([#1408](https://github.com/helix-editor/helix/pull/1408))
- Fix auto-pairs with CRLF ([#1470](https://github.com/helix-editor/helix/pull/1470))
- Fix picker scrolling when the bottom is reached ([#1567](https://github.com/helix-editor/helix/pull/1567))
- Use markup themes for the markdown component ([#1363](https://github.com/helix-editor/helix/pull/1363))
- Automatically commit changes to history if not in insert mode ([`2a7ae96`](https://github.com/helix-editor/helix/commit/2a7ae96))
- Render code-actions as a menu and add padding to popup ([`094a0aa`](https://github.com/helix-editor/helix/commit/094a0aa))
- Only render menu scrollbar if the menu doesn't fit ([`f10a06f`](https://github.com/helix-editor/helix/commit/f10a06f), [`36b975c`](https://github.com/helix-editor/helix/commit/36b975c))
- Parse git revision instead of tag for version ([`d3221b0`](https://github.com/helix-editor/helix/commit/d3221b0), [#1674](https://github.com/helix-editor/helix/pull/1674))
- Fix incorrect last modified buffer ([#1621](https://github.com/helix-editor/helix/pull/1621))
- Add `PageUp`, `PageDown`, `Ctrl-u`, `Ctrl-d`, `Home`, `End` bindings to the file picker ([#1612](https://github.com/helix-editor/helix/pull/1612))
- Display buffer IDs in the buffer picker ([#1134](https://github.com/helix-editor/helix/pull/1134))
- Allow multi-line prompt documentation ([`2af0432`](https://github.com/helix-editor/helix/commit/2af0432))
- Ignore the `.git` directory from the file picker ([#1604](https://github.com/helix-editor/helix/pull/1604))
- Allow separate styling for markup heading levels ([#1618](https://github.com/helix-editor/helix/pull/1618))
- Automatically close popups ([#1285](https://github.com/helix-editor/helix/pull/1285))
- Allow auto-pairs tokens to be configured ([#1624](https://github.com/helix-editor/helix/pull/1624))
- Don't indent empty lines in `indent` command ([#1653](https://github.com/helix-editor/helix/pull/1653))
- Ignore `Enter` keypress when a menu has no selection ([#1704](https://github.com/helix-editor/helix/pull/1704))
- Show errors when surround deletions and replacements fail ([#1709](https://github.com/helix-editor/helix/pull/1709))
- Show infobox hints for `mi` and `ma` ([#1686](https://github.com/helix-editor/helix/pull/1686))
- Highlight matching text in file picker suggestions ([#1635](https://github.com/helix-editor/helix/pull/1635))
- Allow capturing multiple nodes in textobject queries ([#1611](https://github.com/helix-editor/helix/pull/1611))
- Make repeat operator work with completion edits ([#1640](https://github.com/helix-editor/helix/pull/1640))
- Save to the jumplist when searching ([#1718](https://github.com/helix-editor/helix/pull/1718))
- Fix bug with auto-replacement of components in compositor ([#1711](https://github.com/helix-editor/helix/pull/1711))
- Use Kakoune logic for `align_selection` ([#1675](https://github.com/helix-editor/helix/pull/1675))
- Fix `follows` for `nixpkgs` in `flake.nix` ([#1729](https://github.com/helix-editor/helix/pull/1729))
- Performance improvements for the picker ([`78fba86`](https://github.com/helix-editor/helix/commit/78fba86))
- Rename infobox theme scopes ([#1741](https://github.com/helix-editor/helix/pull/1741))
- Fallback to broader scopes if a theme scope is not found ([#1714](https://github.com/helix-editor/helix/pull/1714))
- Add arrow-keys bindings for tree-sitter sibling selection commands ([#1724](https://github.com/helix-editor/helix/pull/1724))
- Fix a bug in LSP when creating a file in a folder that does not exist ([#1775](https://github.com/helix-editor/helix/pull/1775))
- Use `^` and `$` regex location assertions for search ([#1793](https://github.com/helix-editor/helix/pull/1793))
- Fix register names in `insert_register` command ([#1751](https://github.com/helix-editor/helix/pull/1751))
- Perform extend line for all selections ([#1804](https://github.com/helix-editor/helix/pull/1804))
- Prevent panic when moving in an empty picker ([#1786](https://github.com/helix-editor/helix/pull/1786))
- Fix line number calculations for non CR/CRLF line breaks ([`b4a282f`](https://github.com/helix-editor/helix/commit/b4a282f), [`0b96201`](https://github.com/helix-editor/helix/commit/0b96201))
- Deploy documentation for `master` builds separately from release docs ([#1783](https://github.com/helix-editor/helix/pull/1783))

Themes:

- Add everforest_light ([#1412](https://github.com/helix-editor/helix/pull/1412))
- Add gruvbox_light ([#1509](https://github.com/helix-editor/helix/pull/1509))
- Add modified background to dracula popup ([#1434](https://github.com/helix-editor/helix/pull/1434))
- Markup support for monokai pro themes ([#1553](https://github.com/helix-editor/helix/pull/1553))
- Markup support for dracula theme ([#1554](https://github.com/helix-editor/helix/pull/1554))
- Add `tag` to gruvbox theme ([#1555](https://github.com/helix-editor/helix/pull/1555))
- Markup support for remaining themes ([#1525](https://github.com/helix-editor/helix/pull/1525))
- Serika light and dark ([#1566](https://github.com/helix-editor/helix/pull/1566))
- Fix rose_pine and rose_pine_dawn popup background color ([#1606](https://github.com/helix-editor/helix/pull/1606))
- Fix hover menu item text color in base16 themes ([#1668](https://github.com/helix-editor/helix/pull/1668))
- Update markup heading styles for everforest ([#1687](https://github.com/helix-editor/helix/pull/1687))
- Update markup heading styles for rose_pine themes ([#1706](https://github.com/helix-editor/helix/pull/1706))
- Style bogster cursors ([`6a6a9ab`](https://github.com/helix-editor/helix/commit/6a6a9ab))
- Fix `ui.selection` in rose_pine themes ([#1716](https://github.com/helix-editor/helix/pull/1716))
- Use distinct colors for cursor and matched pair in gruvbox ([#1791](https://github.com/helix-editor/helix/pull/1791))
- Improve colors for `ui.cursor.match` capture in some themes ([#1862](https://github.com/helix-editor/helix/pull/1862))

LSP:

- Add default language server for JavaScript ([#1457](https://github.com/helix-editor/helix/pull/1457))
- Add `pom.xml` as maven root directory marker ([#1496](https://github.com/helix-editor/helix/pull/1496))
- Haskell LSP ([#1556](https://github.com/helix-editor/helix/pull/1556))
- C-sharp LSP support ([#1788](https://github.com/helix-editor/helix/pull/1788))
- Clean up Julia LSP config ([#1811](https://github.com/helix-editor/helix/pull/1811))

New Languages:

- llvm-mir ([#1398](https://github.com/helix-editor/helix/pull/1398))
- regex ([#1362](https://github.com/helix-editor/helix/pull/1362))
- Make ([#1433](https://github.com/helix-editor/helix/pull/1433), [#1661](https://github.com/helix-editor/helix/pull/1661))
- git-config ([#1426](https://github.com/helix-editor/helix/pull/1426))
- Lean ([#1422](https://github.com/helix-editor/helix/pull/1422))
- Elm ([#1514](https://github.com/helix-editor/helix/pull/1514))
- GraphQL ([#1515](https://github.com/helix-editor/helix/pull/1515))
- Twig ([#1602](https://github.com/helix-editor/helix/pull/1602))
- Rescript ([#1616](https://github.com/helix-editor/helix/pull/1616), [#1863](https://github.com/helix-editor/helix/pull/1863))
- Erlang ([#1657](https://github.com/helix-editor/helix/pull/1657))
- Kotlin ([#1689](https://github.com/helix-editor/helix/pull/1689))
- HCL ([#1705](https://github.com/helix-editor/helix/pull/1705), [#1726](https://github.com/helix-editor/helix/pull/1726))
- Org ([#1845](https://github.com/helix-editor/helix/pull/1845))
- Solidity ([#1848](https://github.com/helix-editor/helix/pull/1848), [#1854](https://github.com/helix-editor/helix/pull/1854))

Updated Languages and Queries:

- Textobject and indent queries for c and cpp ([#1293](https://github.com/helix-editor/helix/pull/1293))
- Fix null and boolean constant highlights for nix ([#1428](https://github.com/helix-editor/helix/pull/1428))
- Capture markdown link text as `markup.link.text` ([#1456](https://github.com/helix-editor/helix/pull/1456))
- Update and re-enable Haskell ([#1417](https://github.com/helix-editor/helix/pull/1417), [#1520](https://github.com/helix-editor/helix/pull/1520))
- Update Go with generics support ([`ddbf036`](https://github.com/helix-editor/helix/commit/ddbf036))
- Use `tree-sitter-css` for SCSS files ([#1507](https://github.com/helix-editor/helix/pull/1507))
- Update Zig ([#1501](https://github.com/helix-editor/helix/pull/1501))
- Update PHP ([#1521](https://github.com/helix-editor/helix/pull/1521))
- Expand language support for comment injections ([#1527](https://github.com/helix-editor/helix/pull/1527))
- Use tree-sitter-bash for `.zshrc` and `.bashrc` ([`7d51042`](https://github.com/helix-editor/helix/commit/7d51042))
- Use tree-sitter-bash for `.bash_profile` ([#1571](https://github.com/helix-editor/helix/pull/1571))
- Use tree-sitter-bash for `.zshenv` and ZSH files ([#1574](https://github.com/helix-editor/helix/pull/1574))
- IEx ([#1576](https://github.com/helix-editor/helix/pull/1576))
- Textobject queries for PHP ([#1601](https://github.com/helix-editor/helix/pull/1601))
- C-sharp highlight query improvements ([#1795](https://github.com/helix-editor/helix/pull/1795))
- Git commit performance has been improved on large verbose commits ([#1838](https://github.com/helix-editor/helix/pull/1838))

Packaging:

- The submodules system has been replaced with command-line flags for fetching and building tree-sitter grammars ([#1659](https://github.com/helix-editor/helix/pull/1659))
- Flake outputs are pushed to Cachix on each push to `master` ([#1721](https://github.com/helix-editor/helix/pull/1721))
- Update flake's `nix-cargo-integration` to depend on `dream2nix` ([#1758](https://github.com/helix-editor/helix/pull/1758))

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
- Align selections (`&`) ([#1101](https://github.com/helix-editor/helix/pull/1101))
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
- Make kill_to_line_end behave like Emacs ([#1235](https://github.com/helix-editor/helix/pull/1235))
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
- Fix a test failure occurring only on `test --release` ([`4f108ab1`](https://github.com/helix-editor/helix/commit/4f108ab1b2197809506bd7305ad903a3525eabfa))
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
- Registers can now be selected via " (for example, `"ay`)
- Support for Nix files was added
- Movement is now fully tested and matches Kakoune implementation
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
- Show matching bracket (Vim's matchbrackets)
- Themes now support style modifiers
- First user contributed theme
- Create a document if it doesn't exist yet on save
- Detect language on a new file on save

- Panic fixes, lots of them
