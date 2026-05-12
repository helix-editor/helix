[
  "module"
  "type"
  "assume"
  "const"
  "var"
  "val"
  "nondet"
  "def"
  "pure"
  "action"
  "temporal"
  "run"
] @keyword

(match_expr "match" @keyword.control.conditional)

(if_else_condition 
  "if" @keyword.control.conditional
  "else" @keyword.control.conditional)

(import "import" @keyword.control.import)
(import "as" @keyword.control.import)
(import "from" @keyword.control.import)
(export "export" @keyword.control.import)
(export "as" @keyword.control.import)

[
  "true"
  "false"
  "Int"
  "Nat"
  "Bool"
] @constant.builtin

[
  ";"
  "."
  ","
] @punctuation.delimiter

[
  "-"
  "+"
  "*"
  "/"
  "%"
  "<"
  "<="
  "="
  "=="
  "!="
  "=>"
  ">"
  ">="
  "^"
  "->"
] @operator

(infix_and "and" @operator)
(infix_or "or" @operator)
(infix_iff "iff" @operator)
(infix_implies "implies" @operator)

(braced_and "and" @keyword)
(braced_or "or" @keyword)
(braced_all "all" @keyword)
(braced_any "any" @keyword)

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(polymorphic_type 
  (type) @type.parameter)

(variant_constructor) @type.enum.variant

(type) @type
(int_literal) @constant.numeric.integer
(comment) @comment
(string) @string

(operator_application
  operator: (qualified_identifier) @function)

(operator_definition
  name: (qualified_identifier) @function
  arguments: (typed_argument_list))
