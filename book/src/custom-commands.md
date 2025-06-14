# Custom Commands

There are three kinds of commands that can be used in custom commands:

* Static commands: commands like `move_char_right` which are usually bound to
  keys and used for movement and editing. A list of static commands is
  available in the [Keymap](./keymap.html) documentation and in the source code
  in [`helix-term/src/commands.rs`](https://github.com/helix-editor/helix/blob/master/helix-term/src/commands.rs)
  at the invocation of `static_commands!` macro.
* Typable commands: commands that can be executed from command mode (`:`), for
  example `:write!`. See the [Commands](./commands.html) documentation for a
  list of available typeable commands or the `TypableCommandList` declaration in
  the source code at [`helix-term/src/commands/typed.rs`](https://github.com/helix-editor/helix/blob/master/helix-term/src/commands/typed.rs).
* Macros: sequences of keys that are executed in order. These keybindings
  start with `@` and then list any number of keys to be executed. For example
  `@miw` can be used to select the surrounding word. For now, macro keybindings
  are not allowed in sequences due to limitations in the way that
  command sequences are executed. Modifier keys (e.g. Alt+o) can be used
  like `"<A-o>"`, e.g. `"@miw<A-o>"`

To remap commands, create a `config.toml` file in your `helix` configuration
directory (default `~/.config/helix` on Linux systems) with a structure like
this:

```toml
[commands]
":wcb" = [":write", ":buffer-close"] # Maps `:wcb` to write the current buffer and then close it
":f" = ":format" # Maps `:f` to format the current buffer
":W" = ":write!" # Maps `:W` to forcefully save the current buffer
":Q" = ":quit!" # Maps `:Q` to forcefully quit helix
":hints" = ":toggle lsp.display-inlay-hints" # Maps `:hints` to toggle inlay hints
```

## Shadowing Built-in Commands

If you redefine a built-in command but still need access to the original, prefix the command with `^` when entering it.

Example:

```toml
[commands]
":w" = ":write!" # Force save
```

To invoke the original behavior:

```
:^w
```

This executes the original `:write` command instead of the remapped one.

## Visibility

By default, custom commands appear in the command list. If you prefer to keep them hidden, omit the `:` prefix:

```toml
[commands]
"0" = ":goto 1" # `:0` moves to the first line
```

Even though `:0` can still be used, it won't appear in the command list.

## Positional Arguments

To pass arguments to an underlying command, use `%arg`:

```toml
[commands]
":cc" = ":pipe xargs ccase --to %arg{0}"
```

Example usage:

```
:cc snake
```

This executes: `:pipe xargs ccase --to snake`.

- `%arg` uses zero-based indexing (`%arg{0}`, `%arg{1}`, etc.).
- Valid argument brace syntax follows the [Command Line](./command-line.html) conventions.

## Descriptions and Prompts

To provide descriptions for custom commands, use optional fields:

```toml
[commands.":wcd!"]
commands = [":write! %arg(0)", ":cd %sh{ %arg(0) | path dirname }"]
desc = "Force save buffer, then change directory"
accepts = "<path>"
```

## Command Completion

To enable autocompletion for a custom command, assign it an existing completer:

```toml
[commands.":touch"]
commands = [":noop %sh{ touch %arg{0} }"]
completer = ":write"
```

This allows `:touch` to inherit `:write`'s file path completion.
