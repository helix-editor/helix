## Languages

Language-specific settings and settings for language servers are configured
in `languages.toml` files.

## `languages.toml` files

There are three possible locations for a `languages.toml` file:

1. In the Helix source code, which lives in the
   [Helix repository](https://github.com/helix-editor/helix/blob/master/languages.toml).
   It provides the default configurations for languages and language servers.

2. In your [configuration directory](./configuration.md). This overrides values
   from the built-in language configuration. For example, to disable
   auto-formatting for Rust:

   ```toml
   # in <config_dir>/helix/languages.toml

   [language-server.mylang-lsp]
   command = "mylang-lsp"

   [[language]]
   name = "rust"
   auto-format = false
   ```

3. In a `.helix` folder in your project. Language configuration may also be
   overridden local to a project by creating a `languages.toml` file in a
   `.helix` folder. Its settings will be merged with the language configuration
   in the configuration directory and the built-in configuration.

## Language configuration

Each language is configured by adding a `[[language]]` section to a
`languages.toml` file. For example:

```toml
[[language]]
name = "mylang"
scope = "source.mylang"
injection-regex = "mylang"
file-types = ["mylang", "myl"]
comment-tokens = "#"
indent = { tab-width = 2, unit = "  " }
formatter = { command = "mylang-formatter" , args = ["--stdin"] }
language-servers = [ "mylang-lsp" ]
```

These configuration keys are available:

| Key                   | Description                                                   |
| ----                  | -----------                                                   |
| `name`                | The name of the language                                      |
| `language-id`         | The language-id for language servers, checkout the table at [TextDocumentItem](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocumentItem) for the right id |
| `scope`               | A string like `source.js` that identifies the language. Currently, we strive to match the scope names used by popular TextMate grammars and by the Linguist library. Usually `source.<name>` or `text.<name>` in case of markup languages |
| `injection-regex`     | regex pattern that will be tested against a language name in order to determine whether this language should be used for a potential [language injection][treesitter-language-injection] site. |
| `file-types`          | The filetypes of the language, for example `["yml", "yaml"]`. See the file-type detection section below. |
| `shebangs`            | The interpreters from the shebang line, for example `["sh", "bash"]` |
| `roots`               | A set of marker files to look for when trying to find the workspace root. For example `Cargo.lock`, `yarn.lock` |
| `auto-format`         | Whether to autoformat this language when saving               |
| `diagnostic-severity` | Minimal severity of diagnostic for it to be displayed. (Allowed values: `error`, `warning`, `info`, `hint`) |
| `comment-tokens`      | The tokens to use as a comment token, either a single token `"//"` or an array `["//", "///", "//!"]` (the first token will be used for commenting). Also configurable as `comment-token` for backwards compatibility|
| `block-comment-tokens`| The start and end tokens for a multiline comment either an array or single table of `{ start = "/*", end = "*/"}`. The first set of tokens will be used for commenting, any pairs in the array can be uncommented |
| `indent`              | The indent to use. Has sub keys `unit` (the text inserted into the document when indenting; usually set to N spaces or `"\t"` for tabs) and `tab-width` (the number of spaces rendered for a tab) |
| `language-servers`    | The Language Servers used for this language. See below for more information in the section [Configuring Language Servers for a language](#configuring-language-servers-for-a-language)   |
| `grammar`             | The tree-sitter grammar to use (defaults to the value of `name`) |
| `formatter`           | The formatter for the language, it will take precedence over the lsp when defined. The formatter must be able to take the original file as input from stdin and write the formatted file to stdout. The filename of the current buffer can be passed as argument by using the `%{buffer_name}` expansion variable. See below for more information in the [Configuring the formatter command](#configuring-the-formatter-command) |
| `soft-wrap`           | [editor.softwrap](./editor.md#editorsoft-wrap-section)
| `text-width`          |  Maximum line length. Used for the `:reflow` command and soft-wrapping if `soft-wrap.wrap-at-text-width` is set, defaults to `editor.text-width`   |
| `rulers`              | Overrides the `editor.rulers` config key for the language. |
| `path-completion`     | Overrides the `editor.path-completion` config key for the language. |
| `word-completion`     | Overrides the [`editor.word-completion`](./editor.md#editorword-completion-section) configuration for the language. |
| `workspace-lsp-roots`     | Directories relative to the workspace root that are treated as LSP roots. Should only be set in `.helix/config.toml`. Overwrites the setting of the same name in `config.toml` if set. |
| `persistent-diagnostic-sources` | An array of LSP diagnostic sources assumed unchanged when the language server resends the same set of diagnostics. Helix can track the position for these diagnostics internally instead. Useful for diagnostics that are recomputed on save.

### File-type detection and the `file-types` key

Helix determines which language configuration to use based on the `file-types` key
from the above section. `file-types` is a list of strings or tables, for
example:

```toml
file-types = ["toml", { glob = "Makefile" }, { glob = ".git/config" }, { glob = ".github/workflows/*.yaml" } ]
```

When determining a language configuration to use, Helix searches the file-types
with the following priorities:

1. Glob: values in `glob` tables are checked against the full path of the given
   file. Globs are standard Unix-style path globs (e.g. the kind you use in Shell)
   and can be used to match paths for a specific prefix, suffix, directory, etc.
   In the above example, the `{ glob = "Makefile" }` config would match files
   with the name `Makefile`, the `{ glob = ".git/config" }` config would match
   `config` files in `.git` directories, and the `{ glob = ".github/workflows/*.yaml" }`
   config would match any `yaml` files in `.github/workflow` directories. Note
   that globs should always use the Unix path separator `/` even on Windows systems;
   the matcher will automatically take the machine-specific separators into account.
   If the glob isn't an absolute path or doesn't already start with a glob prefix,
   `*/` will automatically be added to ensure it matches for any subdirectory.
2. Extension: if there are no glob matches, any `file-types` string that matches
   the file extension of a given file wins. In the example above, the `"toml"`
   config matches files like `Cargo.toml` or `languages.toml`.

### Configuring the formatter command

[Command line expansions](./command-line.md#expansions) are supported in the arguments
of the formatter command. In particular, the `%{buffer_name}` variable can be passed as
argument to the formatter:

```toml
formatter = { command = "mylang-formatter" , args = ["--stdin", "--stdin-filename %{buffer_name}"] }
```

## Language Server configuration

Language servers are configured separately in the table `language-server` in the same file as the languages `languages.toml`

For example:

```toml
[language-server.mylang-lsp]
command = "mylang-lsp"
args = ["--stdio"]
config = { provideFormatter = true }
environment = { "ENV1" = "value1", "ENV2" = "value2" }

[language-server.efm-lsp-prettier]
command = "efm-langserver"

[language-server.efm-lsp-prettier.config]
documentFormatting = true
languages = { typescript = [ { formatCommand ="prettier --stdin-filepath ${INPUT}", formatStdin = true } ] }
```

These are the available options for a language server.

| Key                        | Description                                                                                                                       |
| ----                       | -----------                                                                                                                       |
| `command`                  | The name or path of the language server binary to execute. Binaries must be in `$PATH`                                            |
| `args`                     | A list of arguments to pass to the language server binary                                                                         |
| `config`                   | Language server initialization options                                                                                            |
| `timeout`                  | The maximum time a request to the language server may take, in seconds. Defaults to `20`                                          |
| `environment`              | Any environment variables that will be used when starting the language server `{ "KEY1" = "Value1", "KEY2" = "Value2" }`          |
| `required-root-patterns`   | A list of `glob` patterns to look for in the working directory. The language server is started if at least one of them is found.  |

A `format` sub-table within `config` can be used to pass extra formatting options to
[Document Formatting Requests](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_formatting).
For example, with typescript:

```toml
[language-server.typescript-language-server]
# pass format options according to https://github.com/typescript-language-server/typescript-language-server#workspacedidchangeconfiguration omitting the "[language].format." prefix.
config = { format = { "semicolons" = "insert", "insertSpaceBeforeFunctionParenthesis" = true } }
```

### Configuring Language Servers for a language

The `language-servers` attribute in a language tells helix which language servers are used for this language.

They have to be defined in the `[language-server]` table as described in the previous section.

Different languages can use the same language server instance, e.g. `typescript-language-server` is used for javascript, jsx, tsx and typescript by default.

The definition order of language servers affects the order in the results list of code action menu.

In case multiple language servers are specified in the `language-servers` attribute of a `language`,
it's often useful to only enable/disable certain language-server features for these language servers.

As an example, `efm-lsp-prettier` of the previous example is used only with a formatting command `prettier`,
so everything else should be handled by the `typescript-language-server` (which is configured by default).
The language configuration for typescript could look like this:

```toml
[[language]]
name = "typescript"
language-servers = [ { name = "efm-lsp-prettier", only-features = [ "format" ] }, "typescript-language-server" ]
```

or equivalent:

```toml
[[language]]
name = "typescript"
language-servers = [ { name = "typescript-language-server", except-features = [ "format" ] }, "efm-lsp-prettier" ]
```

Each requested LSP feature is prioritized in the order of the `language-servers` array.
For example, the first `goto-definition` supported language server (in this case `typescript-language-server`) will be taken for the relevant LSP request (command `goto_definition`).
The features `diagnostics`, `code-action`, `completion`, `document-symbols` and `workspace-symbols` are an exception to that rule, as they are working for all language servers at the same time and are merged together, if enabled for the language.
If no `except-features` or `only-features` is given, all features for the language server are enabled.
If a language server itself doesn't support a feature, the next language server array entry will be tried (and so on).

The list of supported features is:

- `format`
- `goto-definition`
- `goto-declaration`
- `goto-type-definition`
- `goto-reference`
- `goto-implementation`
- `signature-help`
- `hover`
- `document-highlight`
- `completion`
- `code-action`
- `workspace-command`
- `document-symbols`
- `workspace-symbols`
- `diagnostics`
- `rename-symbol`
- `inlay-hints`

## Tree-sitter grammar configuration

The source for a language's tree-sitter grammar is specified in a `[[grammar]]`
section in `languages.toml`. For example:

```toml
[[grammar]]
name = "mylang"
source = { git = "https://github.com/example/mylang", rev = "a250c4582510ff34767ec3b7dcdd3c24e8c8aa68" }
```

Grammar configuration takes these keys:

| Key      | Description                                                              |
| ---      | -----------                                                              |
| `name`   | The name of the tree-sitter grammar                                      |
| `source` | The method of fetching the grammar - a table with a schema defined below |

Where `source` is a table with either these keys when using a grammar from a
git repository:

| Key    | Description                                               |
| ---    | -----------                                               |
| `git`  | A git remote URL from which the grammar should be cloned  |
| `rev`  | The revision (commit hash or tag) which should be fetched |
| `subpath` | A path within the grammar directory which should be built. Some grammar repositories host multiple grammars (for example `tree-sitter-typescript` and `tree-sitter-ocaml`) in subdirectories. This key is used to point `hx --grammar build` to the correct path for compilation. When omitted, the root of repository is used |

### Choosing grammars

You may use a top-level `use-grammars` key to control which grammars are
fetched and built when using `hx --grammar fetch` and `hx --grammar build`.

```toml
# Note: this key must come **before** the [[language]] and [[grammar]] sections
use-grammars = { only = [ "rust", "c", "cpp" ] }
# or
use-grammars = { except = [ "yaml", "json" ] }
```

When omitted, all grammars are fetched and built.

[treesitter-language-injection]: https://tree-sitter.github.io/tree-sitter/3-syntax-highlighting.html#language-injection
