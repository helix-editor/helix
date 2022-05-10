; Identifier naming conventions

((identifier) @constant
 (#match? @constant "^[A-Z_]*$"))

((identifier) @constructor
 (#match? @constructor "^[A-Z]"))

; Types

((identifier) @type
  (#match?
    @type
    "^(bool|bytes|dict|float|frozenset|int|list|set|str|tuple)$"))

(type (identifier)) @type

; Builtin functions

((call
  function: (identifier) @function.builtin)
 (#match?
   @function.builtin
   "^(abs|all|any|ascii|bin|breakpoint|bytearray|callable|chr|classmethod|compile|complex|delattr|dir|divmod|enumerate|eval|exec|filter|format|getattr|globals|hasattr|hash|help|hex|id|input|isinstance|issubclass|iter|len|locals|map|max|memoryview|min|next|object|oct|open|ord|pow|print|property|range|repr|reversed|round|setattr|slice|sorted|staticmethod|sum|super|type|vars|zip|__import__)$"))

; Function calls

(decorator) @function

(call
  function: (attribute attribute: (identifier) @function.method))
(call
  function: (identifier) @function)

; Function definitions

(function_definition
  name: (identifier) @function)

(identifier) @variable
(attribute attribute: (identifier) @variable.other.member)

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

