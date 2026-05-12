;; ----------------------------------------------------------------------------
;; Literals and comments

(line_comment) @comment.line

(block_comment) @comment.block

(xml_doc) @comment.block.documentation

(const
  [
   (_) @constant
   (unit) @constant.builtin
  ])

(primary_constr_args (_) @variable.parameter)

((identifier_pattern (long_identifier (identifier) @special))
 (#match? @special "^\_.*"))

((long_identifier
  (identifier)+
  .
  (identifier) @variable.other.member))

;; ----------------------------------------------------------------------------
;; Punctuation

(wildcard_pattern) @string.special

(type_name type_name: (_) @type)

[
 (type)
 (atomic_type)
] @type

(member_signature
  .
  (identifier) @function.method
  (curried_spec
    (arguments_spec
      "*"* @operator
      (argument_spec
        (argument_name_spec
          "?"? @special
          name: (_) @variable.parameter)))))

(union_type_case) @constant

(rules
  (rule
    pattern: (_) @constant
    block: (_)))

(identifier_pattern
  .
  (_) @constant
  .
  (_) @variable)

(fsi_directive_decl . (string) @namespace)

(import_decl . (_) @namespace)
(named_module
  name: (_) @namespace)
(namespace
  name: (_) @namespace)
(module_defn
  .
  (_) @namespace)

(ce_expression
  .
  (_) @function.macro)

(field_initializer
  field: (_) @variable.other.member)

(record_fields
  (record_field
    .
    (identifier) @variable.other.member))

(dot_expression
  base: (_) @namespace
  field: (_) @variable.other.member)

(value_declaration_left . (_) @variable)

(function_declaration_left
  . (_) @function
  [
    (argument_patterns)
    (argument_patterns (long_identifier (identifier)))
  ] @variable.parameter)

(member_defn
  (method_or_prop_defn
    [
      (property_or_ident) @function
      (property_or_ident
        instance: (identifier) @variable.builtin
        method: (identifier) @function.method)
    ]
    args: (_)* @variable.parameter))

(application_expression
  .
  [
    (long_identifier_or_op [
      (long_identifier (identifier)* (identifier) @function)
      (identifier) @function
    ])
    (typed_expression . (long_identifier_or_op (long_identifier (identifier)* . (identifier) @function.call)))
    (dot_expression base: (_) @variable.other.member field: (_) @function)
  ] @function)

((infix_expression
  .
  (_)
  .
  (infix_op) @operator
  .
  (_) @function
  )
 (#eq? @operator "|>")
 )

((infix_expression
  .
  (_) @function
  .
  (infix_op) @operator
  .
  (_)
  )
 (#eq? @operator "<|")
 )

[
  (xint)
  (int)
  (int16)
  (uint16)
  (int32)
  (uint32)
  (int64)
  (uint64)
  (nativeint)
  (unativeint)
] @constant.numeric.integer

[
  (ieee32)
  (ieee64)
  (float)
  (decimal)
] @constant.numeric.float

(bool) @constant.builtin.boolean

([
  (string)
  (triple_quoted_string)
  (verbatim_string)
  (char)
] @string)

(compiler_directive_decl) @keyword.directive

(attribute) @attribute

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
  "[|"
  "|]"
  "{|"
  "|}"
  "[<"
  ">]"
] @punctuation.bracket

(format_string_eval
  [
    "{"
    "}"
  ] @punctuation.special)

[
  ","
  ";"
] @punctuation.delimiter

[
  "|"
  "="
  ">"
  "<"
  "-"
  "~"
  "->"
  "<-"
  "&&"
  "||"
  ":>"
  ":?>"
  (infix_op)
  (prefix_op)
] @operator

[
  "if"
  "then"
  "else"
  "elif"
  "when"
  "match"
  "match!"
] @keyword.control.conditional

[
  "and"
  "or"
  "not"
  "upcast"
  "downcast"
] @keyword.operator

[
  "return"
  "return!"
  "yield"
  "yield!"
] @keyword.control.return

[
  "for"
  "while"
  "downto"
  "to"
] @keyword.control.repeat


[
  "open"
  "#r"
  "#load"
] @keyword.control.import

[
  "abstract"
  "delegate"
  "static"
  "inline"
  "mutable"
  "override"
  "rec"
  "global"
  (access_modifier)
] @keyword.storage.modifier

[
  "let"
  "let!"
  "use"
  "use!"
  "member"
] @keyword.function

[
  "enum"
  "type"
  "inherit"
  "interface"
] @keyword.storage.type

(try_expression
  [
    "try"
    "with"
    "finally"
  ] @keyword.control.exception)

((identifier) @keyword.control.exception
 (#any-of? @keyword.control.exception "failwith" "failwithf" "raise" "reraise"))

[
  "as"
  "assert"
  "begin"
  "end"
  "done"
  "default"
  "in"
  "do"
  "do!"
  "event"
  "field"
  "fun"
  "function"
  "get"
  "set"
  "lazy"
  "new"
  "of"
  "param"
  "property"
  "struct"
  "val"
  "module"
  "namespace"
  "with"
] @keyword

[
  "null"
] @constant.builtin

(match_expression "with" @keyword.control.conditional)

((type
  (long_identifier (identifier) @type.builtin))
 (#any-of? @type.builtin "bool" "byte" "sbyte" "int16" "uint16" "int" "uint" "int64" "uint64" "nativeint" "unativeint" "decimal" "float" "double" "float32" "single" "char" "string" "unit"))

(preproc_if
  [
    "#if" @keyword.directive
    "#endif" @keyword.directive
  ]
  condition: (_)? @keyword.directive)

(preproc_else
  "#else" @keyword.directive)

((long_identifier
  (identifier)+ @namespace
  .
  (identifier)))

(long_identifier_or_op
  (op_identifier) @operator)

((identifier) @namespace
 (#any-of? @namespace "Array" "Async" "Directory" "File" "List" "Option" "Path" "Map" "Set" "Lazy" "Seq" "Task" "String" "Result" ))
