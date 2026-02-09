# Configuration

To override global configuration parameters, create a `config.toml` file located in your config directory:

- Linux and Mac: `~/.config/silicon/config.toml`
- Windows: `%AppData%\silicon\config.toml`

> 💡 You can easily open the config file by typing `:config-open` within Silicon normal mode.

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
`--config` command line argument, for example `si -c path/to/custom-config.toml`.
You can reload the config file by issuing the `:config-reload` command. Alternatively, on Unix operating systems, you can reload it by sending the USR1
signal to the Silicon process, such as by using the command `pkill -USR1 si`.

Finally, you can have a `config.toml` and a `languages.toml` local to a project by putting it under a `.silicon` directory in your repository.
Its settings will be merged with the configuration directory and the built-in configuration.

