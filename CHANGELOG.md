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
