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
| `smart-case` | Enable smart case regex searching (case insensitive unless pattern contains upper case characters) | `true` |
| `auto-pairs` | Enable automatic insertion of pairs to parenthese, brackets, etc. | `true` |
| `auto-completion` | Enable automatic pop up of auto-completion. | `true` |
| `idle-timeout` | Time in milliseconds since last keypress before idle timers trigger. Used for autocompletion, set to 0 for instant. | `400` |
| `completion-trigger-len` | The min-length of word under cursor to trigger autocompletion | `2` |
| `auto-info` | Whether to display infoboxes | `true` |
| `true-color` | Set to `true` to override automatic detection of terminal truecolor support in the event of a false negative. | `false` |

`[editor.filepicker]` section of the config. Sets options for file picker and global search. All but the last key listed in the default file-picker configuration below are IgnoreOptions: whether hidden files and files listed within ignore files are ignored by (not visible in) the helix file picker and global search. There is also one other key, `max-depth` available, which is not defined by default.

| Key | Description | Default |
|--|--|---------|
|`hidden` | Enables ignoring hidden files. | true
|`parents` | Enables reading ignore files from parent directories. | true
|`ignore` | Enables reading `.ignore` files. | true
|`git-ignore` | Enables reading `.gitignore` files. | true
|`git-global` | Enables reading global .gitignore, whose path is specified in git's config: `core.excludefile` option. | true
|`git-exclude` | Enables reading `.git/info/exclude` files. | true
|`max-depth` | Set with an integer value for maximum depth to recurse. | Defaults to `None`.

## LSP

To display all language server messages in the status line add the following to your `config.toml`:
```toml
[lsp]
display-messages = true
```
