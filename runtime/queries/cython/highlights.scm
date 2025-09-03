; Punctuation

["," "." ":" ";" (ellipsis)] @punctuation.delimiter
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
(interpolation
  "{" @punctuation.special
  "}" @punctuation.special)

; Identifier naming conventions

(identifier) @variable

((identifier) @constructor
 (#match? @constructor "^[A-Z]"))

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z_]*$"))

; Function calls

(decorator) @function

(call
  function: (attribute attribute: (identifier) @function.method))
(call
  function: (identifier) @function)

; Builtin functions

((call
  function: (identifier) @function.builtin)
 (#any-of?
   @function.builtin
   "abs" "all" "any" "ascii" "bin" "bool" "breakpoint" "bytearray" "bytes" "callable" "chr" "classmethod" "compile" "complex" "delattr" "dict" "dir" "divmod" "enumerate" "eval" "exec" "filter" "float" "format" "frozenset" "getattr" "globals" "hasattr" "hash" "help" "hex" "id" "input" "int" "isinstance" "issubclass" "iter" "len" "list" "locals" "map" "max" "memoryview" "min" "next" "object" "oct" "open" "ord" "pow" "print" "property" "range" "repr" "reversed" "round" "set" "setattr" "slice" "sorted" "staticmethod" "str" "sum" "super" "tuple" "type" "vars" "zip" "__import__"))

; Types

(maybe_typed_name
  type: ((_) @type))

(type
  (identifier) @type)

(c_type
  type: ((_) @type))
(c_type
  ((identifier) @type))
(c_type
  ((int_type) @type))

(maybe_typed_name
  name: ((identifier) @variable))

; Function definitions

(function_definition
  name: (identifier) @function)

(cdef_statement
  (cvar_def
    (maybe_typed_name
      name: ((identifier) @function))
    (c_function_definition)))

(cvar_decl
  (c_type
    ([(identifier) (int_type)]))
  (c_name
    ((identifier) @function))
  (c_function_definition))

(attribute attribute: (identifier) @variable.other.member)

; Literals

[
  (none)
] @constant.builtin

[
  (true)
  (false)
] @constant.builtin.boolean

(integer) @constant.numeric.integer
(float) @constant.numeric.float

(comment) @comment
(string) @string
(escape_sequence) @constant.character.escape

(interpolation
  "{" @punctuation.special
  "}" @punctuation.special) @embedded

[
  "-"
  "-="
  "!="
  "*"
  "**"
  "**="
  "*="
  "/"
  "//"
  "//="
  "/="
  "&"
  "&="
  "%"
  "%="
  "^"
  "^="
  "+"
  "->"
  "+="
  "<"
  "<<"
  "<<="
  "<="
  "<>"
  "="
  ":="
  "=="
  ">"
  ">="
  ">>"
  ">>="
  "|"
  "|="
  "~"
  "@="
  "and"
  "in"
  "is"
  "not"
  "or"
 "@"
] @operator

[
  "as"
  "assert"
  "async"
  "await"
  "break"
  "class"
  "continue"
  "def"
  "del"
  "elif"
  "else"
  "except"
  "exec"
  "finally"
  "for"
  "from"
  "global"
  "if"
  "import"
  "lambda"
  "nonlocal"
  "pass"
  "print"
  "raise"
  "return"
  "try"
  "while"
  "with"
  "yield"
  "match"
  "case"

  ; cython-specific
  "cdef"
  "cpdef"
  "ctypedef"
  "cimport"
  "nogil"
  "gil"
  "extern"
  "inline"
  "public"
  "readonly"
  "struct"
  "union"
  "enum"
  "fused"
  "property"
  "namespace"
  "cppclass"
  "const"
] @keyword.control

(dotted_name
  (identifier)* @namespace)

(aliased_import
  alias: (identifier) @namespace)
