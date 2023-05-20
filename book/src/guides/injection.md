# Adding Injection Queries

Writing language injection queries allows one to highlight a specific node as a different language.
In addition to the [standard](upstream-docs) language injection options used by tree-sitter, there
are a few Helix specific extensions that allow for more control.

And example of a simple query that would highlight all strings as bash in Nix:
```scm
((string_expression (string_fragment) @injection.content)
  (#set! injection.language "bash"))
```

## Capture Types

- `@injection.language` (standard):
The captured node may contain the language name used to highlight the node captured by
`@injection.content`.

- `@injection.content` (standard):
Marks the content to be highlighted as the language captured with `@injection.language` _et al_.

- `@injection.filename` (extension):
The captured node may contain a filename with a file-extension known to Helix,
highlighting `@injection.content` as that language. This uses the language extensions defined in
both the default languages.toml distributed with Helix, as well as user defined languages.

- `@injection.shebang` (extension):
The captured node may contain a shebang used to choose a language to highlight as. This also uses
the shebangs defined in the default and user `languages.toml`.

## Settings

- `injection.combined` (standard):
Indicates that all the matching nodes in the tree should have their content parsed as one
nested document.

- `injection.language` (standard):
Forces the captured content to be highlighted as the given language

- `injection.include-children` (standard):
Indicates that the content node’s entire text should be re-parsed, including the text of its child
nodes. By default, child nodes’ text will be excluded from the injected document.

- `injection.include-unnamed-children` (extension):
Same as `injection.include-children` but only for unnamed child nodes.

## Predicates

- `#eq?` (standard):
The first argument (a capture) must be equal to the second argument
(a capture or a string).

- `#match?` (standard):
The first argument (a capture) must match the regex given in the
second argument (a string).

[upstream-docs]: http://tree-sitter.github.io/tree-sitter/syntax-highlighting#language-injection
