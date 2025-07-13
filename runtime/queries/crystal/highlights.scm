[
  ","
  ";"
  "."
  ":"
  "*"
  "**"
] @punctuation.delimiter

[
  "alias"
  "alignof"
  "annotation"
  "asm"
  "begin"
  "case"
  "def"
  "do"
  "end"
  "extend"
  "forall"
  "fun"
  "in"
  "include"
  "instance_alignof"
  "instance_sizeof"
  "macro"
  "of"
  "offsetof"
  "out"
  "pointerof"
  "select"
  "sizeof"
  "then"
  "type"
  "typeof"
  "uninitialized"
  "verbatim"
  "when"
  "with"
] @keyword

[
  "else"
  "elsif"
  "if"
  "unless"
] @keyword.control.conditional

[
  "for"
  "until"
  "while"
] @keyword.control.repeat

["require"] @keyword.control.import

[
  "break"
  "next"
  "return"
  "yield"
] @keyword.control.return

[
  "ensure"
  "rescue"
] @keyword.control.exception

[
  "class"
  "enum"
  "lib"
  "module"
  "struct"
  "union"
] @keyword.storage.type

(conditional
  [
    "?"
    ":"
  ] @keyword.control.conditional)

[
  (private)
  (protected)
  "abstract"
] @keyword

(pseudo_constant) @constant.builtin

; literals
(char
  ["'" (literal_content)] @string)

(char
  (escape_sequence) @constant.character.escape)

(string
  ["\"" (literal_content)] @string)

(string
  (escape_sequence) @constant.character.escape)

(symbol
  [
    ":"
    ":\""
    "\""
    (literal_content)
  ] @string.special.symbol)

(symbol
  (escape_sequence) @constant.character.escape)

(command
  ["`" (literal_content)] @string.special)

(command
  (escape_sequence) @constant.character.escape)

(regex
  "/" @punctuation.bracket)

(regex
  (literal_content) @string.regexp)

(regex_modifier) @string.special.symbol

(heredoc_body
  (literal_content) @string)

(heredoc_body
  (escape_sequence) @constant.character.escape)

[
  (heredoc_start)
  (heredoc_end)
] @string.symbol

(integer) @constant.numeric.integer
(float) @constant.numeric.float

[
  (true)
  (false)
  (nil)
  (self)
] @variable.builtin

(
  (comment)+ @comment.block.documentation
  .
  [
    (class_def)
    (struct_def)
    (method_def)
    (abstract_method_def)
    (macro_def)
    (module_def)
    (enum_def)
    (annotation_def)
    (lib_def)
    (type_def)
    (c_struct_def)
    (union_def)
    (alias)
    (const_assign)
  ]
)

(comment) @comment

; Operators and punctuation
[
  "="
  "=>"
  "->"
  "&"
  (operator)
] @operator

[
  "("
  ")"
  "["
  "@["
  "]"
  "{"
  "}"
] @punctuation.bracket

(index_call
  method: (operator) @punctuation.bracket
  [
    "]"
    "]?"
  ] @punctuation.bracket)

(block
    "|" @punctuation.bracket)

[
  "{%"
  "%}"
  "{{"
  "}}"
] @keyword.directive

(interpolation
  "#{" @punctuation.special
  "}" @punctuation.special)

; TODO: {splat,double_splat,block,fun}_param + rescue param

; Types

(nilable_constant
  "?" @type)

(nilable_type
  "?" @type)

(union_type
  "|" @operator)

(annotation
  (constant) @attribute)

(identifier) @variable
(param name: (identifier) @variable.parameter)

(method_def
  name: (identifier) @function.method)

(macro_def
  name: (identifier) @function.method)

(abstract_method_def
  name: (identifier) @function.method)

(fun_def
  name: (identifier) @function.method
  real_name: (identifier)? @function.method)

(macro_var) @variable

[
  (class_var)
  (instance_var)
] @variable.other.member

(named_expr
  name: (identifier) @variable.other.member
  ":" @variable.other.member)

(named_type
    name: (identifier) @variable.other.member)

(underscore) @variable.special

; function calls

(call
    method: (_) @keyword
    arguments: (argument_list
      [
        (type_declaration
          var: (_) @function.method)
        (assign
          lhs: (_) @function.method)
        (_) @function.method
      ])
    (#match? @keyword "^(class_)?(getter|setter|property)[?!]?$"))

(call
    method: (_) @keyword
    (#match? @keyword "^(record|is_a\\?|as|as\\?|responds_to\\?|nil\\?|\\!)$"))

(call
  method: (_) @function.method)

(implicit_object_call
  method: (_) @function.method)

(method_proc
  method: (_) @function.method)

(assign_call
  method: (_) @function.method)

((identifier) @variable.builtin
  (#match? @variable.builtin "^(previous_def|super)$"))

[
  (constant)
  (generic_instance_type)
  (generic_type)
] @type
