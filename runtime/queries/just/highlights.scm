; From <https://github.com/IndianBoy42/tree-sitter-just/blob/6c2f018ab1d90946c0ce029bb2f7d57f56895dff/queries-flavored/helix/highlights.scm>

; This file specifies how matched syntax patterns should be highlighted

[
  "export"
  "import"
] @keyword.control.import

"mod" @keyword.directive

[
  "alias"
  "set"
  "shell"
] @keyword

[
  "if"
  "else"
] @keyword.control.conditional

; Variables

(value
  (identifier) @variable)

(alias
  left: (identifier) @variable)

(assignment
  left: (identifier) @variable)

; Functions

(recipe_header
  name: (identifier) @function)

(dependency
  name: (identifier) @function)

(dependency_expression
  name: (identifier) @function)

(function_call
  name: (identifier) @function)

; Parameters

(parameter
  name: (identifier) @variable.parameter)

; Namespaces

(module
  name: (identifier) @namespace)

; Operators

[
  ":="
  "?"
  "=="
  "!="
  "=~"
  "@"
  "="
  "$"
  "*"
  "+"
  "&&"
  "@-"
  "-@"
  "-"
  "/"
  ":"
] @operator

; Punctuation

"," @punctuation.delimiter

[
  "{"
  "}"
  "["
  "]"
  "("
  ")"
  "{{"
  "}}"
] @punctuation.bracket

[ "`" "```" ] @punctuation.special

; Literals

(boolean) @constant.builtin.boolean

[
  (string)
  (external_command)
] @string

(escape_sequence) @constant.character.escape

; Comments

(comment) @comment.line

(shebang) @keyword.directive

; highlight known settings (filtering does not always work)
(setting
  left: (identifier) @keyword
  (#any-of? @keyword
    "allow-duplicate-recipes"
    "dotenv-filename"
    "dotenv-load"
    "dotenv-path"
    "export"
    "fallback"
    "ignore-comments"
    "positional-arguments"
    "shell"
    "tempdi"
    "windows-powershell"
    "windows-shell"))

; highlight known attributes (filtering does not always work)
(attribute
  (identifier) @attribute
  (#any-of? @attribute
    "private"
    "allow-duplicate-recipes"
    "dotenv-filename"
    "dotenv-load"
    "dotenv-path"
    "export"
    "fallback"
    "ignore-comments"
    "positional-arguments"
    "shell"
    "tempdi"
    "windows-powershell"
    "windows-shell"))

; Numbers are part of the syntax tree, even if disallowed
(numeric_error) @error
