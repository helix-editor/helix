; Builtin functions

((call
  function: (identifier) @function.builtin)
 (#match?
   @function.builtin
   "^(abs|all|any|ascii|bin|bool|breakpoint|bytearray|bytes|callable|chr|classmethod|compile|complex|delattr|dict|dir|divmod|enumerate|eval|exec|filter|float|format|frozenset|getattr|globals|hasattr|hash|help|hex|id|input|int|isinstance|issubclass|iter|len|list|locals|map|max|memoryview|min|next|object|oct|open|ord|pow|print|property|range|repr|reversed|round|set|setattr|slice|sorted|staticmethod|str|sum|super|tuple|type|vars|zip|__import__)$"))

; Function calls

(call
  function: (attribute attribute: (identifier) @constructor)
 (#match? @constructor "^[A-Z]"))
(call
  function: (identifier) @constructor
 (#match? @constructor "^[A-Z]"))

(call
  function: (attribute attribute: (identifier) @function.method))

(call
  function: (identifier) @function)

; Function definitions

(function_definition
  name: (identifier) @constructor
 (#match? @constructor "^(__new__|__init__)$"))

(function_definition
  name: (identifier) @function)

; Decorators

(decorator) @function
(decorator (identifier) @function)
(decorator (attribute attribute: (identifier) @function))
(decorator (call
  function: (attribute attribute: (identifier) @function)))

; Parameters

((identifier) @variable.builtin
 (#match? @variable.builtin "^(self|cls)$"))

(parameters (identifier) @variable.parameter)
(parameters (typed_parameter (identifier) @variable.parameter))
(parameters (default_parameter name: (identifier) @variable.parameter))
(parameters (typed_default_parameter name: (identifier) @variable.parameter))
(keyword_argument name: (identifier) @variable.parameter)

; Types

((identifier) @type.builtin
 (#match?
   @type.builtin
   "^(bool|bytes|dict|float|frozenset|int|list|set|str|tuple)$"))

; In type hints make everything types to catch non-conforming identifiers
; (e.g., datetime.datetime) and None
(type [(identifier) (none)] @type)
; Handle [] . and | nesting 4 levels deep
(type
  (_ [(identifier) (none)]? @type
    (_ [(identifier) (none)]? @type
      (_ [(identifier) (none)]? @type
        (_ [(identifier) (none)]? @type)))))

(class_definition name: (identifier) @type)
(class_definition superclasses: (argument_list (identifier) @type))

; Variables

((identifier) @constant
 (#match? @constant "^[A-Z_]{2,}$"))

((identifier) @type
 (#match? @type "^[A-Z]")) 

(attribute attribute: (identifier) @variable.other.member)
(identifier) @variable

; Literals

[
  (none)
  (true)
  (false)
] @constant.builtin

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
  "%"
  "%="
  "^"
  "+"
  "->"
  "+="
  "<"
  "<<"
  "<="
  "<>"
  "="
  ":="
  "=="
  ">"
  ">="
  ">>"
  "|"
  "~"
] @operator

[
  "as"
  "assert"
  "await"
  "break"
  "continue"
  "elif"
  "else"
  "except"
  "finally"
  "for"
  "from"
  "if"
  "import"
  "pass"
  "raise"
  "return"
  "try"
  "while"
  "with"
  "yield"
] @keyword.control

(for_statement "in" @keyword.control)
(for_in_clause "in" @keyword.control)

[
  "and"
  "async"
  "class"
  "def"
  "del"
  "exec"
  "global"
  "in"
  "is"
  "lambda"
  "nonlocal"
  "not"
  "or"
  "print"
] @keyword

