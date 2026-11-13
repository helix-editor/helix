# Configuration

To override global configuration parameters, create a `config.toml` file located in your config directory:

- Linux and Mac: `~/.config/helix/config.toml`
- Windows: `%AppData%\helix\config.toml`

> 💡 You can easily open the config file by typing `:config-open` within Helix normal mode.

Example config:

```toml
theme = "onedark"

[editor]
line-number = "relative"
mouse = false

[editor.cursor-shape]
insert = "bar"
normal = "block"
select = "underline"

[editor.file-picker]
hidden = false
```

You can use a custom configuration file by specifying it with the `-c` or
`--config` command line argument, for example `hx -c path/to/custom-config.toml`.
You can reload the config file by issuing the `:config-reload` command. Alternatively, on Unix operating systems, you can reload it by sending the USR1
signal to the Helix process, such as by using the command `pkill -USR1 hx`.

Finally, you can have a `config.toml` and a `languages.toml` local to a project by putting it under a `.helix` directory in your repository.
Its settings will be merged with the configuration directory and the built-in configuration.

## Modular configuration

`config.toml` can be split into modules. After loading the main `config.toml`,
Helix scans `config.d/*.toml` next to it (e.g. `~/.config/helix/config.d/*.toml`
or, for `--config path/to/config.toml`, `path/to/config.d/*.toml`) and merges
each file alphabetically on top of the main file. Tables are deep-merged (so
nested `[keys.normal.space]`, `[editor.statusline.mode]`, etc. keep their other
entries); arrays are appended in file order and scalar values are replaced. This lets you keep
`config.toml` small and enable/disable features by adding or removing a file:

```
~/.config/helix/
  config.toml
  config.d/
    10-editor.toml      # [editor] tweaks
    20-vim.toml         # Vim muscle-memory (see contrib/config.d/vim.toml)
    30-obsidian.toml    # Obsidian pickers
```

`20-vim.toml` in this repository is a direct copy of
[`LGUG2Z/helix-vim`](https://github.com/LGUG2Z/helix-vim) (`contrib/config.d/vim.toml`
and `contrib/rx-helix/config.d/vim.toml`). Enable it with:

```sh
mkdir -p ~/.config/helix/config.d
cp contrib/config.d/vim.toml ~/.config/helix/config.d/vim.toml
# reload inside Helix:
:config-reload
```

Files are merged in sorted order, so prefix with `10-`, `20-` to control
precedence. Invalid TOML in any module is reported as `Bad config` on startup.

## Keymap descriptions

Nested keymap tables can set `desc` to label the prefix in the Space/which-key
popup:

```toml
[keys.normal.space.o]
desc = "Obsidian"
o = ":obsidian-quick-switch"
g = ":obsidian-search"
```

Without `desc`, the prefix still works but its menu row has no description.

