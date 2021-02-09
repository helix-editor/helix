# Helix

| Crate        | Description                                            |
| -----------  | -----------                                            |
| helix-core   | Core editing primitives, functional.                   |
| helix-syntax | Tree-sitter grammars                                   |
| helix-view   | UI abstractions for use in backends, imperative shell. |
| helix-term   | Terminal UI                                            |

# Installation

```
git clone --depth 1 --recurse-submodules -j8 https://github.com/helix-editor/helix
cd helix
cargo install --path helix-term
```

This will install the `hx` binary to `$HOME/.cargo/bin`.

