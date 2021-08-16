# Configuration

To override global configuration parameters, create a `config.toml` file located in your config directory:

* Linux and Mac: `~/.config/helix/config.toml`
* Windows: `%AppData%\helix\config.toml`

## LSP

To display all language server messages in the status line add the following to your `config.toml`:
```toml
[lsp]
display-messages = true
```
