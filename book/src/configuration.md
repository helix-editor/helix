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
| `file-picker` | Sets options for file picker and global search. Multiple pairs in an inline table. Details below.  | `{hidden = true, parents = true, ignore = false, git-ignore = true, git-global = true, git-exclude = true } |
All the pairs listed in the default configuration above are IgnoreOptions: which types of files are ignored in the file picker and global search. 
`hidden` Directly enables ignoring hidden files.
`parents` Enables reading ignore files from parent directories. 
`ignore` Enables reading `.ignore` files.
`git-ignore` Enables reading `.gitignore` files.
`git-global` Enables reading global .gitignore, whose path is specified in git's config: `core.excludefile` option.
`git-exclude` Enables reading `.git/info/exclude` files.
`max-depth` can also be used as a key, and can be bound to an integer value for maximum depth to recurse. Defaults to `None`.

## LSP

To display all language server messages in the status line add the following to your `config.toml`:
```toml
[lsp]
display-messages = true
```
