# Configuration

To override global configuration parameters, create a `config.toml` file located in your config directory:

* Linux and Mac: `~/.config/helix/config.toml`
* Windows: `%AppData%\helix\config.toml`

## Editor

`[editor]` section of the config.

| Key | Description | Default |
|--|--|---------|
| `scrolloff` | Number of lines of padding around the edge of the screen when scrolling. | `3` |
| `mouse` | Enable mouse mode. | `true` |
| `middle-click-paste` | Middle click paste support. | `true` |
| `scroll-lines` | Number of lines to scroll per scroll wheel step. | `3` |
| `shell` | Shell to use when running external commands. | Unix: `["sh", "-c"]`<br/>Windows: `["cmd", "/C"]` |
| `line-number` | Line number display (`absolute`, `relative`) | `absolute` |

## LSP

To display all language server messages in the status line add the following to your `config.toml`:
```toml
[lsp]
display-messages = true
```
