[
  "use" "no" "require" "package" "class" "role"
] @keyword.control.import

[
  "sub" "method" "async" "extended"
] @keyword.function

[
  "if" "elsif" "else" "unless"
] @keyword.control.conditional

[
  "while" "until"
  "for" "foreach"
  "do"
] @keyword.control.repeat

[
  "my" "our" "local" "state"
] @keyword.storage.modifier

[
  "last" "next" "redo" "goto" "return"
] @keyword.control.return

[
  "undef"
] @constant.builtin

(phaser_statement phase: _ @keyword.directive)
(class_phaser_statement phase: _ @keyword.directive)

[
  "or" "xor" "and"
  "eq" "ne" "cmp" "lt" "le" "ge" "gt"
  "isa"
] @keyword.operator

(comment) @comment

(eof_marker) @keyword.directive
(data_section) @comment

(number) @constant.numeric
(version) @constant

(string_literal) @string
(interpolated_string_literal) @string
(quoted_word_list) @string
(command_string) @string
[(heredoc_token) (command_heredoc_token)] @string.special
(heredoc_content) @string
(heredoc_end) @string.special
[(escape_sequence) (escaped_delimiter)] @constant.character.escape

[(quoted_regexp) (match_regexp)] @string.regexp

(autoquoted_bareword) @string.special

[(scalar) (arraylen)] @variable
(scalar_deref_expression ["->" "$" "*"] @variable)
(array) @variable
(array_deref_expression ["->" "@" "*"] @variable)
(hash) @variable
(hash_deref_expression ["->" "%" "*"] @variable)

(array_element_expression [array:(_) "->" "[" "]"] @variable)
(slice_expression [array:(_) "->" "[" "]"] @variable)
(keyval_expression [array:(_) "->" "[" "]"] @variable)

(hash_element_expression [hash:(_) "->" "{" "}"] @variable)
(slice_expression [hash:(_) "->" "[" "]"] @variable)
(keyval_expression [hash:(_) "->" "[" "]"] @variable)

(hash_element_expression key: (bareword) @string.special)

(use_statement (package) @type)
(package_statement (package) @type)
(require_expression (bareword) @type)

(subroutine_declaration_statement name: (_) @function)
(attrlist (attribute) @attribute)

(goto_expression (label) @label)
(loopex_expression (label) @label)

(statement_label label: _ @label)

(relational_expression operator: "isa" right: (bareword) @type)

(function_call_expression (function) @function)
(method_call_expression (method) @function.method)
(method_call_expression invocant: (bareword) @type)

(func0op_call_expression function: _ @function.builtin)
(func1op_call_expression function: _ @function.builtin)

(function) @function
