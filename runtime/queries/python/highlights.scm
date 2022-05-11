; Types

(type (identifier) @type)
(type (subscript (identifier) @type)) ; only one deep...
(class_definition name: (identifier) @type)
(class_definition superclasses: (argument_list (identifier) @type))

; Decorators

(decorator) @function
(decorator (identifier) @function)
(decorator (call function: (identifier) @function))

; Builtin functions

((call
  function: (identifier) @function.builtin)
 (#match?
   @function.builtin
   "^(abs|all|any|ascii|bin|bool|breakpoint|bytearray|bytes|callable|chr|classmethod|compile|complex|delattr|dict|dir|divmod|enumerate|eval|exec|filter|float|format|frozenset|getattr|globals|hasattr|hash|help|hex|id|input|int|isinstance|issubclass|iter|len|list|locals|map|max|memoryview|min|next|object|oct|open|ord|pow|print|property|range|repr|reversed|round|set|setattr|slice|sorted|staticmethod|str|sum|super|tuple|type|vars|zip|__import__)$"))

; Function calls

((call
  function: (identifier) @constructor)
 (#match? @constructor "^[A-Z]"))
(call
  function: (attribute attribute: (identifier) @function.method))
(call
  function: (identifier) @function)

; Function definitions

(function_definition
  name: (identifier) @function)

; First parameter of a classmethod
((class_definition
  body: (block
          (decorated_definition
            (decorator (identifier) @_decorator)
            definition: (function_definition
              parameters: (parameters . (identifier) @variable.builtin)))))
 (#eq? @variable.builtin "cls")
 (#eq? @_decorator "classmethod"))

((identifier) @variable.builtin
 (#eq? @variable.builtin "self"))

(parameters
  (identifier) @variable.parameter)
(parameters (typed_parameter (identifier) @variable.parameter))
(attribute attribute: (identifier) @variable.other.member)

; Identifier naming conventions

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z_]*$"))

((identifier) @type
 (#match? @type "^[A-Z].*[a-z]$"))

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
  "and"
  "in"
  "is"
  "not"
  "or"
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
] @keyword
