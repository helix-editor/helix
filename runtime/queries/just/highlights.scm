; This file specifies how matched syntax patterns should be highlighted

[
  "export"
  "import"
  "unexport"
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

[
  "&&"
  "||"
] @operator

; Variables

(value
  (identifier) @variable)

(alias
  alias_name: (identifier) @variable)

(assignment
  name: (identifier) @variable)

(shell_variable_name) @variable

(unexport
  name: (identifier) @variable)

; Functions

(recipe
  name: (identifier) @function)

(recipe_dependency
  name: (identifier) @function.call)

(function_call
  name: (identifier) @function.builtin)

; Parameters

(recipe_parameter
  name: (identifier) @variable.parameter)

; Namespaces

(mod
  name: (identifier) @namespace)

(module_path
  name: (identifier) @namespace)

; Paths

(mod
  (path) @string.special.path)

(import
  (path) @string.special.path)

; Shebangs

(shebang_line) @keyword.directive
(shebang_line
  (shebang_shell) @string.special)


(shell_expanded_string
  [
    (expansion_short_start)
    (expansion_long_start)
    (expansion_long_middle)
    (expansion_long_end)
  ] @punctuation.special)

; Operators

[
  ":="
  "?"
  "=="
  "!="
  "=~"
  "!~"
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

; Booleans are not allowed anywhere except in settings
(setting
  (boolean) @constant.builtin.boolean)

[
  (string)
  (external_command)
] @string

[
  (escape_sequence)
  (escape_variable_end)
] @constant.character.escape

; Comments

(comment) @comment.line

; highlight known settings
(setting
  name: (_) @keyword.function)

; highlight known attributes
(attribute
  name: (identifier) @attribute)

; Numbers are part of the syntax tree, even if disallowed
(numeric_error) @error
