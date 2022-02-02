helix-syntax
============

Syntax highlighting for helix. This crate builds the tree-sitter
grammars fetched by the grammars script.

Differences from nvim-treesitter
--------------------------------

As the syntax are commonly ported from
<https://github.com/nvim-treesitter/nvim-treesitter>.

Note that we do not support the custom `#any-of` predicate which is
supported by neovim so one needs to change it to `#match` with regex.
