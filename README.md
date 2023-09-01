<div align="center">

<h1>
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="logo_dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="logo_light.svg">
  <img alt="Helix" height="128" src="logo_light.svg">
</picture>
</h1>

[![Build status](https://github.com/helix-editor/helix/actions/workflows/build.yml/badge.svg)](https://github.com/helix-editor/helix/actions)
[![GitHub Release](https://img.shields.io/github/v/release/helix-editor/helix)](https://github.com/helix-editor/helix/releases/latest)
[![Documentation](https://shields.io/badge/-documentation-452859)](https://docs.helix-editor.com/)
[![GitHub contributors](https://img.shields.io/github/contributors/helix-editor/helix)](https://github.com/helix-editor/helix/graphs/contributors)
[![Matrix Space](https://img.shields.io/matrix/helix-community:matrix.org)](https://matrix.to/#/#helix-community:matrix.org)

</div>

![Screenshot](./screenshot.png)

<div align="center">
  <p>A <a href="https://kakoune.org/">Kakoune</a> and <a href="https://neovim.io/">Neovim</a> inspired editor, written in Rust.</p>
  
  <a href="https://docs.helix-editor.com/install.html"><img alt="Static Badge" src="https://img.shields.io/badge/Installation-997BC8?style=for-the-badge"></a>
  <a href="https://helix-editor.com"><img alt="Static Badge" src="https://img.shields.io/badge/Website-997BC8?style=for-the-badge"></a>
  <a href="https://docs.helix-editor.com"><img alt="Static Badge" src="https://img.shields.io/badge/Documentation-997BC8?style=for-the-badge"></a>
  <a href="https://github.com/helix-editor/helix/wiki/Troubleshooting"><img alt="Static Badge" src="https://img.shields.io/badge/Troubleshooting-997BC8?style=for-the-badge"></a>
  <a href="https://github.com/helix-editor/helix/blob/master/docs/CONTRIBUTING.md"><img alt="Static Badge" src="https://img.shields.io/badge/Contributing-997BC8?style=for-the-badge"></a>
  <br>
  <a href="https://matrix.to/#/#helix-community:matrix.org"><img alt="Static Badge" src="https://img.shields.io/badge/Matrix-Space-55C5E4?style=for-the-badge"></a>
  <a href="https://matrix.to/#/#helix-editor:matrix.org"><img alt="Static Badge" src="https://img.shields.io/badge/Matrix-Chat-55C5E4?style=for-the-badge"></a>
</div>

# Features

- Vim-like modal editing
- Multiple selections
- Built-in language server support
- Smart, incremental syntax highlighting and code editing via tree-sitter

It's a terminal-based editor first, but I'd like to explore a custom renderer
(similar to Emacs) in wgpu or skulpin.

> [!NOTE]  
> Only certain languages have indentation definitions at the moment. Please check `runtime/queries/<lang>/` for `indents.scm` file.

# Credits

Thanks to [@jakenvac](https://github.com/jakenvac) for designing the logo!
