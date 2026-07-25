; Comments

(comment_line) @comment.line
(comment_block) @comment.block

; Function

[
  (action_command)
  (control_command)
  (known_test)
] @function.builtin

(command
  name: (identifier) @function)
(test
  name: (identifier) @function)

; Keywords

[
  "if"
  "elsif"
  "else"
] @keyword.control.conditional

[
  "include"
  "require"
] @keyword.control.import

"foreverypart" @keyword.control.repeat

[
  "break"
  "return"
  "stop"
] @keyword.control.return

[
  "allof"
  "anyof"
  "not"
] @keyword.operator

(declare_command) @keyword.storage

; Builtin constants

[
  "false"
  "true"
] @constant.builtin.boolean

(escape_sequence) @constant.character.escape

(number_maybe_unit) @constant.numeric.float

[
  (hex_pair)
  (unicode_val)
] @constant.numeric.integer

; Punctuation

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

";" @punctuation.delimiter

(interpolation_marker) @punctuation.special

; String

[
  (string)
  (string_content)
] @string

((string
  (_
    (string_content) @_inbox))
    (#match? @_inbox "^INBOX\..*")) @string.special.path

(command
  (known_command
    (action_command) @keyword.control.import
    (#eq? @keyword.control.import "include"))
  (arguments
    (argument
      (tag))*
    .
    (argument
      (string
        (_
          (string_content) @string.special.path))
    )
  )
)

; Variables

(variable_interpolation (variable_name) @variable)

(command
  (known_command
    (declare_command) @keyword.storage
    (#any-of? @keyword.storage "global" "set"))
  (arguments
    (argument
      (tag))*
    .
    (argument
      (string
        (_
          (string_content) @variable))
    )
  )
)

; Namespaces

(namespace)* @namespace

; Operators

(tag) @operator
