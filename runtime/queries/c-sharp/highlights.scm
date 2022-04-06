;; Methods
(method_declaration
  name: (identifier) @function)

(method_declaration
  type: [(identifier) (qualified_name)] @type)

(invocation_expression
  (member_access_expression
    name: (identifier) @function))

(invocation_expression
  (member_access_expression
    expression: (identifier) @variable))

(invocation_expression
  function: (conditional_access_expression
    (member_binding_expression
      name: (identifier) @function)))

(invocation_expression
      [(identifier) (qualified_name)] @function)

; Generic Method invocation with generic type
(invocation_expression
  function: (generic_name
              . (identifier) @function))

;; Namespaces

(namespace_declaration
  name: [(identifier) (qualified_name)] @namespace)

;; Types
(interface_declaration name: (identifier) @type)
(class_declaration name: (identifier) @type)
(enum_declaration name: (identifier) @type)
(struct_declaration (identifier) @type)
(record_declaration (identifier) @type)
(namespace_declaration name: (identifier) @type)
(using_directive (_) @namespace)
(constructor_declaration name: (identifier) @type)
(object_creation_expression [(identifier) (qualified_name)] @type)
(type_parameter_list (type_parameter) @type)

[
  (implicit_type)
  (nullable_type)
  (pointer_type)
  (function_pointer_type)
  (predefined_type)
] @type.builtin

;; Generic Types
(type_of_expression
  (generic_name
    (identifier) @type))

(base_list
  (generic_name
    (identifier) @type))

(type_constraint
  (generic_name
    (identifier) @type))

(object_creation_expression
  (generic_name
   (identifier) @type))

(property_declaration
  (generic_name
    (identifier) @type))

(_
  type: (generic_name
   (identifier) @type))

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
  "<="
  "<<"
  "<<="
  "="
  "=="
  "!"
  "!="
  "=>"
  ">"
  ">="
  ">>"
  ">>="
  "|"
  "||"
  "|="
  "?"
  "??"
  "^"
  "^="
  "~"
  "*"
  "*="
  "/"
  "/="
  "%"
  "%="
  ":"
  ".."
  "&="
  "->"
  "??="
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
  "static"
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

(preprocessor_call) @keyword.directive
(preprocessor_call (_) @keyword.directive)

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
(binary_expression [(identifier) (qualified_name)] @variable [(identifier) (qualified_name)] @variable)
(binary_expression [(identifier) (qualified_name)]* @variable)
(conditional_expression [(identifier) (qualified_name)] @variable)
(prefix_unary_expression [(identifier) (qualified_name)] @variable)
(postfix_unary_expression [(identifier) (qualified_name)]* @variable)
(assignment_expression [(identifier) (qualified_name)] @variable)
(cast_expression [(identifier) (qualified_name)] @type [(identifier) (qualified_name)] @variable)

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
  
;; Delegate

(delegate_declaration (identifier) @type)

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

(parameter_list
  (parameter
   name: (identifier) @parameter))

(parameter_list
  (parameter
   type: [(identifier) (qualified_name)] @type))

;; Typeof
(type_of_expression [(identifier) (qualified_name)] @type)

;; Variable
(variable_declaration [(identifier) (qualified_name)] @type)
(variable_declarator [(identifier) (qualified_name)] @variable)

;; Return
(return_statement (identifier) @variable)
(yield_statement (identifier) @variable)

;; Type
(generic_name (identifier) @type)
(type_parameter [(identifier) (qualified_name)] @variable.parameter)
(type_argument_list [(identifier) (qualified_name)] @type)

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

;; Rest
(member_access_expression) @variable
(element_access_expression (identifier) @variable)
(argument (identifier) @variable)
(for_statement (identifier) @variable)
(for_each_statement (identifier) @variable)
(expression_statement (identifier) @variable)
(member_access_expression expression: (identifier) @variable)
(member_access_expression name: (identifier) @variable)
(conditional_access_expression [(identifier) (qualified_name)] @variable)
((identifier) @comment.unused
 (#eq? @comment.unused "_"))
