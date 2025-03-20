(self) @variable.builtin

(unit_definition (identifier) @function)

(parameter (identifier) @variable.parameter)

((pipeline_reg_marker) @keyword)

(scoped_identifier
  path: (identifier) @namespace)
(scoped_identifier
 (scoped_identifier
  name: (identifier) @namespace))

((builtin_type) @type.builtin)

((identifier) @type.builtin
 (#any-of?
    @type.builtin
    "uint"
    "Option"
    "Memory"))

((identifier) @type.enum.variant.builtin
 (#any-of? @type.enum.variant.builtin "Some" "None"))

((pipeline_stage_name) @label)

((stage_reference
    stage: (identifier) @label))

[
    "pipeline"
    "let"
    "set"
    "entity"
    "fn"
    "reg"
    "reset"
    "initial"
    "inst"
    "assert"
    "struct"
    "enum"
    "stage"
    "impl"
    "port"
    "decl"
    "mod"
    "where"
    "trait"
] @keyword

[
 "use"
] @keyword.import

[
    "gen"
] @keyword.directive

((gen_if_expression  ["if" "else"] @keyword.directive))
((naked_gen_if_expression  ["if" "else"] @keyword.directive))

((attribute) ["#" "[" "]"] @punctuation.delimiter)

[
  "else"
  "if"
  "match"
] @keyword.control.conditional

(bool_literal) @constant.builtin.boolean
(int_literal) @constant.numeric.integer

[
  "&"
  "inv"
  "-"
  "=>"
  ">"
  "<"
  "::<"
  "::$<"
  "="
  "->"
  "~"
  "!"
] @operator


((op_add) @operator)
((op_sub) @operator)
((op_mul) @operator)
((op_equals) @operator)
((op_lt) @operator)
((op_gt) @operator)
((op_le) @operator)
((op_ge) @operator)
((op_lshift) @operator)
((op_rshift) @operator)
((op_bitwise_and) @operator)
((op_bitwise_xor) @operator)
((op_bitwise_or) @operator)
((op_logical_and) @operator)
((op_logical_or) @operator)


[
  (line_comment)
  (block_comment)
] @comment

[
  (doc_comment)
] @comment.block.documentation


((identifier) @type
  (#match? @type "[A-Z]"))

((scoped_identifier
    name: (identifier) @type)
 (#match? @type "^[A-Z]"))

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]*$"))

