<div align="center">

<h1>
Evil Helix
<!-- <picture> -->
<!--   <source media="(prefers-color-scheme: dark)" srcset="logo_dark.svg"> -->
<!--   <source media="(prefers-color-scheme: light)" srcset="logo_light.svg"> -->
<!--   <img alt="Helix" height="128" src="logo_light.svg"> -->
<!-- </picture> -->
</h1>

</div>

![Screenshot](./screenshot.png)

# Project Goals
- Implement VIM motions as closely as possible
- Reuse Helix's already implemented functions as much as possible
- Integrate [lazygit](https://github.com/jesseduffield/lazygit) into Helix somehow (very long term goal)
- Integrate an [oil.nvim](https://github.com/stevearc/oil.nvim) style file browser (very long term goal)

# What works
## V-motions
- `v`
    - `w/W`
    - `b/B`
    - `e/E`
- `vi` (select inside textobject) and `va` (select around textobject)
> NOTE: The pairs matching first looks for any surrounding pair and if not found, will search for the next one forward
    - `w/W`
    - `p`
    - treesitter objects 
        - `f` for function
        - `t` for type
        - `a` for argument
        - `c` for comment
        - `T` for test
    - pairs
        - `{`
        - `(`
        - `[`
        - etc
- `vt` and `vf`
    - i.e. `vt"` or `vT"` to select until `"` forward or backwards
    - i.e. `vf"` or `vF"` to select to `"` forward or backwards
    - using a count like `3vf"`
- `V` enters visual line mode

## D-motions
- `dd` deletes entire line
    - accepts counts like `3dd` to delete 3 lines
- `D` to delete from cursor to end of line
- `d`
    - `w/W`
    - `b/B`
    - `e/E`
- `di` (select inside textobject) and `da` (select around textobject)
> NOTE: The pairs matching first looks for any surrounding pair and if not found, will search for the next one forward
    - `w/W`
    - `p`
    - treesitter objects 
        - `f` for function
        - `t` for type
        - `a` for argument
        - `c` for comment
        - `T` for test
    - pairs
        - `{`
        - `(`
        - `[`
        - etc
- `dt` and `df`
    - i.e. `dt"` or `dT"` to delete until `"` forward or backwards
    - i.e. `df"` or `dF"` to delete to `"` forward or backwards
    - using a count like `3df"`

## C-motions
- `C` to change from cursor to end of line
- `c`
    - `w/W`
    - `b/B`
    - `e/E`
- `ci` (select inside textobject) and `ca` (select around textobject)
> NOTE: The pairs matching first looks for any surrounding pair and if not found, will search for the next one forward
    - `w/W`
    - `p`
    - treesitter objects 
        - `f` for function
        - `t` for type
        - `a` for argument
        - `c` for comment
        - `T` for test
    - pairs
        - `{`
        - `(`
        - `[`
        - etc
- `ct` and `cf`
    - i.e. `ct"` or `cT"` to change until `"` forward or backwards
    - i.e. `cf"` or `cF"` to change to `"` forward or backwards
    - using a count like `3cf"`

## Y-motions
- `yy` yanks entire line
    - accepts counts like `3yy` to yank 3 lines
- `y`
    - `w/W`
    - `b/B`
    - `e/E`
- `yi` (select inside textobject) and `ya` (select around textobject)
> NOTE: The pairs matching first looks for any surrounding pair and if not found, will search for the next one forward
    - `w/W`
    - `p`
    - treesitter objects 
        - `f` for function
        - `t` for type
        - `a` for argument
        - `c` for comment
        - `T` for test
    - pairs
        - `{`
        - `(`
        - `[`
        - etc
- `yt` and `yf`
    - i.e. `yt"` or `yT"` to yank until `"` forward or backwards
    - i.e. `yf"` or `yF"` to yank to `"` forward or backwards
    - using a count like `3yf"`

## Misc
- Normal and Insert modes no longer selects as you go (removes Helix default behavior)
- Helix shows available options for keys as you press them
- `w/W`, `e/E`, and `b/B` all go to the correct spot of word
- `t` and `f`
    - i.e. `t"` or `T"` to move until `"` forward or backwards
    - i.e. `f"` or `F"` to move to `"` forward or backwards
    - using a count like `3f"`
- `S` to change entire line
- `$` to go to end of line
- `^` to go to first non-whitespace of line
- `0` to go to beginning of line
- `%` to go to matching pair beneath cursor

# What doesn't work/TODO
- Enter Visual mode by pressing `vv` because I haven't figured out how to set a timer to default to Visual mode if nothing is pressed immediately after `v`
- Currently there is no Visual Block mode because I think Visual mode combined with multicursor does the same thing
- Helix seems to add an additional block that the cursor can be moved to at the end of every line
- When using `dd` or `yy` commands, the cursor position is not kept
- Motions like `cip` or `cif` do not search for next occurence of paragraph or function
- Motions with pairs like `ci{` do not work with a count
- Comments
    - Implement `gcc` to comment in Normal mode
    - Implement `gc` to comment in Visual mode
    - Implement `gb` to block comment in Visual mode
- Probably lots of motions with counts that don't work
- Refactor evil functions to match Helix architecture (i.e. `_impl` functions)
- Refactor tests for new motions and behavior (very long term goal)


# Installation

[Installation documentation](https://docs.helix-editor.com/install.html).

<!-- [![Packaging status](https://repology.org/badge/vertical-allrepos/helix-editor.svg?exclude_unsupported=1)](https://repology.org/project/helix-editor/versions) -->

# Contributing

Contributing guidelines can be found [here](./docs/CONTRIBUTING.md).

I reserve the right to reject any suggestions or PRs for this fork.
