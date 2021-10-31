;; Methods
(method_declaration (identifier) @type (identifier) @function)

;; Types
(interface_declaration name: (identifier) @type)
(class_declaration name: (identifier) @type)
(enum_declaration name: (identifier) @type)
(struct_declaration (identifier) @type)
(record_declaration (identifier) @type)
(namespace_declaration name: (identifier) @type)

(constructor_declaration name: (identifier) @type)

[
  (implicit_type)
  (nullable_type)
  (pointer_type)
  (function_pointer_type)
  (predefined_type)
] @type.builtin

;; Enum
(enum_member_declaration (identifier) @variable.other.member)

;; Literals
[
  (real_literal)
  (integer_literal)
] @constant.numeric.integer

(character_literal) @constant.character
[
  (string_literal)
  (verbatim_string_literal)
  (interpolated_string_text)
  (interpolated_verbatim_string_text)
  "\""
  "$\""
  "@$\""
  "$@\""
 ] @string

(boolean_literal) @constant.builtin.boolean
[
  (null_literal)
  (void_keyword)
] @constant.builtin

;; Comments
(comment) @comment

;; Tokens
[
  ";"
  "."
  ","
] @punctuation.delimiter

[
  "--"
  "-"
  "-="
  "&"
  "&&"
  "+"
  "++"
  "+="
  "<"
  "<<"
  "="
  "=="
  "!"
  "!="
  "=>"
  ">"
  ">>"
  "|"
  "||"
  "?"
  "??"
  "^"
  "~"
  "*"
  "/"
  "%"
  ":"
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket

;; Keywords
(modifier) @keyword
(this_expression) @keyword
(escape_sequence) @constant.character.escape

[
  "as"
  "base"
  "break"
  "case"
  "catch"
  "checked"
  "class"
  "continue"
  "default"
  "delegate"
  "do"
  "else"
  "enum"
  "event"
  "explicit"
  "finally"
  "for"
  "foreach"
  "goto"
  "if"
  "implicit"
  "interface"
  "is"
  "lock"
  "namespace"
  "operator"
  "params"
  "return"
  "sizeof"
  "stackalloc"
  "struct"
  "switch"
  "throw"
  "try"
  "typeof"
  "unchecked"
  "using"
  "while"
  "new"
  "await"
  "in"
  "yield"
  "get"
  "set"
  "when"
  "out"
  "ref"
  "from"
  "where"
  "select"
  "record"
  "init"
  "with"
  "let"
] @keyword


;; Linq
(from_clause (identifier) @variable)
(group_clause)
(order_by_clause)
(select_clause (identifier) @variable)
(query_continuation (identifier) @variable) @keyword

;; Record
(with_expression
  (with_initializer_expression
    (simple_assignment_expression
      (identifier) @variable)))

;; Exprs
(binary_expression (identifier) @variable (identifier) @variable)
(binary_expression (identifier)* @variable)
(conditional_expression (identifier) @variable)
(prefix_unary_expression (identifier) @variable)
(postfix_unary_expression (identifier)* @variable)
(assignment_expression (identifier) @variable)
(cast_expression (identifier) @type (identifier) @variable)

;; Class
(base_list (identifier) @type)
(property_declaration (generic_name))
(property_declaration
  type: (nullable_type) @type
  name: (identifier) @variable)
(property_declaration
  type: (predefined_type) @type
  name: (identifier) @variable)
(property_declaration
  type: (identifier) @type
  name: (identifier) @variable)

;; Lambda
(lambda_expression) @variable

;; Attribute
(attribute) @type

;; Parameter
(parameter
  type: (identifier) @type
  name: (identifier) @variable.parameter)
(parameter (identifier) @variable.parameter)
(parameter_modifier) @keyword

;; Typeof
(type_of_expression (identifier) @type)

;; Variable
(variable_declaration (identifier) @type)
(variable_declarator (identifier) @variable)

;; Return
(return_statement (identifier) @variable)
(yield_statement (identifier) @variable)

;; Type
(generic_name (identifier) @type)
(type_parameter (identifier) @variable.parameter)
(type_argument_list (identifier) @type)

;; Type constraints
(type_parameter_constraints_clause (identifier) @variable.parameter)
(type_constraint (identifier) @type)

;; Exception
(catch_declaration (identifier) @type (identifier) @variable)
(catch_declaration (identifier) @type)

;; Switch
(switch_statement (identifier) @variable)
(switch_expression (identifier) @variable)

;; Lock statement
(lock_statement (identifier) @variable)
