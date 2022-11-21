; Upstream: https://github.com/alex-pinkus/tree-sitter-swift/blob/8d2fd80e3322df51e3f70952e60d57f5d4077eb8/queries/highlights.scm

(line_string_literal
  ["\\(" ")"] @punctuation.special)

["." ";" ":" "," ] @punctuation.delimiter
["(" ")" "[" "]" "{" "}"] @punctuation.bracket ; TODO: "\\(" ")" in interpolations should be @punctuation.special

; Identifiers
(attribute) @variable
(type_identifier) @type
(self_expression) @variable.builtin

; Declarations
"func" @keyword.function
[
  (visibility_modifier)
  (member_modifier)
  (function_modifier)
  (property_modifier)
  (parameter_modifier)
  (inheritance_modifier)
] @keyword

(function_declaration (simple_identifier) @function.method)
(function_declaration "init" @constructor)
(throws) @keyword
"async" @keyword
(where_keyword) @keyword
(parameter external_name: (simple_identifier) @variable.parameter)
(parameter name: (simple_identifier) @variable.parameter)
(type_parameter (type_identifier) @variable.parameter)
(inheritance_constraint (identifier (simple_identifier) @variable.parameter))
(equality_constraint (identifier (simple_identifier) @variable.parameter))
(pattern bound_identifier: (simple_identifier)) @variable

[
  "typealias"
  "struct"
  "class"
  "actor"
  "enum"
  "protocol"
  "extension"
  "indirect"
  "nonisolated"
  "override"
  "convenience"
  "required"
  "some"
] @keyword

[
  (getter_specifier)
  (setter_specifier)
  (modify_specifier)
] @keyword

(class_body (property_declaration (pattern (simple_identifier) @variable.other.member)))
(protocol_property_declaration (pattern (simple_identifier) @variable.other.member))

(import_declaration "import" @keyword.control.import)

(enum_entry "case" @keyword)

; Function calls
(call_expression (simple_identifier) @function) ; foo()
(call_expression ; foo.bar.baz(): highlight the baz()
  (navigation_expression
    (navigation_suffix (simple_identifier) @function)))
((navigation_expression
   (simple_identifier) @type) ; SomeType.method(): highlight SomeType as a type
   (#match? @type "^[A-Z]"))

(directive) @function.macro
(diagnostic) @function.macro

; Statements
(for_statement "for" @keyword.control.repeat)
(for_statement "in" @keyword.control.repeat)
(for_statement item: (simple_identifier) @variable)
(else) @keyword
(as_operator) @keyword

["while" "repeat" "continue" "break"] @keyword.control.repeat

["let" "var"] @keyword

(guard_statement "guard" @keyword.control.conditional)
(if_statement "if" @keyword.control.conditional)
(switch_statement "switch" @keyword.control.conditional)
(switch_entry "case" @keyword)
(switch_entry "fallthrough" @keyword)
(switch_entry (default_keyword) @keyword)
"return" @keyword.return
(ternary_expression
  ["?" ":"] @keyword.control.conditional)

["do" (throw_keyword) (catch_keyword)] @keyword

(statement_label) @label

; Comments
(comment) @comment
(multiline_comment) @comment

; String literals
(line_str_text) @string
(str_escaped_char) @string
(multi_line_str_text) @string
(raw_str_part) @string
(raw_str_end_part) @string
(raw_str_interpolation_start) @punctuation.special
["\"" "\"\"\""] @string

; Lambda literals
(lambda_literal "in" @keyword.operator)

; Basic literals
[
  (hex_literal)
  (oct_literal)
  (bin_literal)
] @constant.numeric
(integer_literal) @constant.numeric.integer
(real_literal) @constant.numeric.float
(boolean_literal) @constant.builtin.boolean
"nil" @variable.builtin

"?" @type
(type_annotation "!" @type)

(simple_identifier) @variable

; Operators
[
  "try"
  "try?"
  "try!"
  "!"
  "+"
  "-"
  "*"
  "/"
  "%"
  "="
  "+="
  "-="
  "*="
  "/="
  "<"
  ">"
  "<="
  ">="
  "++"
  "--"
  "&"
  "~"
  "%="
  "!="
  "!=="
  "=="
  "==="
  "??"

  "->"

  "..<"
  "..."
  (custom_operator)
] @operator
