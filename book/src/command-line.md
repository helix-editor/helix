# Command line

- [Quoting](#quoting)
- [Flags](#flags)
- [Expansions](#expansions)
- [Exceptions](#exceptions)

The command line is used for executing [typable commands](./commands.md#typable-commands) like `:write` or `:quit`. Press `:` to activate the command line.

Typable commands optionally accept arguments. `:write` for example accepts an optional path to write the file contents. The command line also supports a quoting syntax for arguments, flags to modify command behaviors, and _expansions_ - a way to insert values from the editor. Most commands support these features but some have custom parsing rules (see the [exceptions](#exceptions) below).

## Quoting

By default, command arguments are split on tabs and space characters. `:open README.md CHANGELOG.md` for example should open two files, `README.md` and `CHANGELOG.md`. Arguments that contain spaces can be surrounded in single quotes (`'`) or backticks (`` ` ``) to prevent the space from separating the argument, like `:open 'a b.txt'`.

Double quotes may be used the same way, but double quotes _expand_ their inner content. `:echo "%{cursor_line}"` for example may print `1` because of the expansion for the `cursor_line` variable. `:echo '%{cursor_line}'` though prints `%{cursor_line}` literally: content within single quotes or backticks is interpreted as-is.

On Unix systems the backslash character may be used to escape certain characters depending on where it is used. Within an argument which isn't surround in quotes, the backslash can be used to escape the space or tab characters: `:open a\ b.txt` is equivalent to `:open 'a b.txt'`. The backslash may also be used to escape quote characters (`'`, `` ` ``, `"`) or the percent token (`%`) when used at the beginning of an argument. `:echo \%%sh{foo}` for example prints `%sh{foo}` instead of invoking a `foo` shell command and `:echo \"quote` prints `"quote`. The backslash character is treated literally in any other situation on Unix systems and always on Windows: `:echo \n` always prints `\n`.

## Flags

Command flags are optional switches that can be used to alter the behavior of a command. For example the `:sort` command accepts an optional `--reverse` (or `-r` for short) flag which causes the sort command to reverse the sorting direction. Typing the `-` character shows completions for the current command's flags, if any.

The `--` flag specifies the end of flags. All arguments after `--` are treated as positional arguments: `:open -- -a.txt` opens a file called `-a.txt`.

## Expansions

Expansions are patterns that Helix recognizes and replaces within the command line. Helix recognizes anything starting with a percent token (`%`) as an expansion, for example `%sh{echo hi!}`. Expansions are particularly useful when used in commands like `:echo` or `:noop` for executing simple scripts. For example:

```toml
[keys.normal]
# Print the current line's git blame information to the statusline.
space.B = ":echo %sh{git blame -L %{cursor_line},+1 %{buffer_name}}"
```

Expansions take the form `%[<kind>]<open><contents><close>`. In `%sh{echo hi!}`, for example, the kind is `sh` - the shell expansion - and the contents are "echo hi!", with `{` and `}` acting as opening and closing delimiters. The following open/close characters are recognized as expansion delimiter pairs: `(`/`)`, `[`/`]`, `{`/`}` and `<`/`>`. Plus the single characters `'`, `"` or `|` may be used instead: `%{cursor_line}` is equivalent to `%<cursor_line>`, `%[cursor_line]` or `%|cursor_line|`.

To escape a percent character instead of treating it as an expansion, use two percent characters consecutively. To execute a shell command like `date -u +'%Y-%m-%d'`, double the percent characters: `:echo %sh{date -u +'%%Y-%%m-%%d'}`.

When no `<kind>` is provided, Helix will expand a **variable**. For example `%{cursor_line}` can be used as in argument to insert the line number. `:echo %{cursor_line}` for instance may print `1` to the statusline.

The following variables are supported:

| Name | Description |
|---   |---          |
| `cursor_line` | The line number of the primary cursor in the currently focused document, starting at 1. |
| `cursor_column` | The column number of the primary cursor in the currently focused document, starting at 1. This is counted as the number of grapheme clusters from the start of the line rather than bytes or codepoints. |
| `buffer_name` | The relative path of the currently focused document. `[scratch]` is expanded instead for scratch buffers. |
| `line_ending` | A string containing the line ending of the currently focused document. For example on Unix systems this is usually a line-feed character (`\n`) but on Windows systems this may be a carriage-return plus a line-feed (`\r\n`). The line ending kind of the currently focused document can be inspected with the `:line-ending` command. |
| `language` | A string containing the language name of the currently focused document.|
| `selection` | A string containing the contents of the primary selection of the currently focused document. |
| `selection_line_start` | The line number of the start of the primary selection in the currently focused document, starting at 1. |
| `selection_line_end` | The line number of the end of the primary selection in the currently focused document, starting at 1. |

Aside from editor variables, the following expansions may be used:

* Unicode `%u{..}`. The contents may contain up to six hexadecimal numbers corresponding to a Unicode codepoint value. For example `:echo %u{25CF}` prints `●` to the statusline.
* Shell `%sh{..}`. The contents are passed to the configured shell command. For example `:echo %sh{echo "20 * 5" | bc}` may print `100` on the statusline on when using a shell with `echo` and the `bc` calculator installed. Shell expansions are evaluated recursively. `%sh{echo '%{buffer_name}:%{cursor_line}'}` for example executes a command like `echo 'README.md:1'`: the variables within the `%sh{..}` expansion are evaluated before executing the shell command.

As mentioned above, double quotes can be used to surround arguments containing spaces but also support expansions within the quoted content unlike singe quotes or backticks. For example `:echo "circle: %u{25CF}"` prints `circle: ●` to the statusline while `:echo 'circle: %u{25CF}'` prints `circle: %u{25CF}`.

Note that expansions are only evaluated once the Enter key is pressed in command mode.

## Exceptions

The following commands support expansions but otherwise pass the given argument directly to the shell program without interpreting quotes:

* `:insert-output`
* `:append-output`
* `:pipe`
* `:pipe-to`
* `:run-shell-command`

For example executing `:sh echo "%{buffer_name}:%{cursor_column}"` would pass text like `echo "README.md:1"` as an argument to the shell program: the expansions are evaluated but not the quotes. As mentioned above, percent characters can be used in shell commands by doubling the percent character. To insert the output of a command like `date -u +'%Y-%m-%d'` use `:insert-output date -u +'%%Y-%%m-%%d'`.

The `:set-option` and `:toggle-option` commands use regular parsing for the first argument - the config option name - and parse the rest depending on the config option's type. `:set-option` interprets the second argument as a string for string config options and parses everything else as JSON.

`:toggle-option`'s behavior depends on the JSON type of the config option supplied as the first argument:

* Booleans: only the config option name should be provided. For example `:toggle-option auto-format` will flip the `auto-format` option.
* Strings: the rest of the command line is parsed with regular quoting rules. For example `:toggle-option indent-heuristic hybrid tree-sitter simple` cycles through "hybrid", "tree-sitter" and "simple" values on each invocation of the command.
* Numbers, arrays and objects: the rest of the command line is parsed as a stream of JSON values. For example `:toggle-option rulers [81] [51, 73]` cycles through `[81]` and `[51, 73]`.

When providing multiple values to `:toggle-option` there should be no duplicates. `:toggle-option indent-heuristic hybrid simple tree-sitter simple` for example would only toggle between "hybrid" and "tree-sitter" values.

`:lsp-workspace-command` works similarly to `:toggle-option`. The first argument (if present) is parsed according to normal rules. The rest of the line is parsed as JSON values. Unlike `:toggle-option`, string arguments for a command must be quoted. For example `:lsp-workspace-command lsp.Command "foo" "bar"`.
