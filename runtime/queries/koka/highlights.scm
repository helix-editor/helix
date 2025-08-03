; Comment

[
  (linecomment)
  (blockcomment)
] @comment

; Literals

(string) @string
(char) @constant.character

(escape) @constant.character.escape

(float) @constant.numeric.float
(int) @constant.numeric.integer

; Delimiters

(matchrule "|" @punctuation.delimiter)

(tatomic "|" @punctuation.delimiter)

[
  ","
  "->"
  "."
  ":"
  "::"
  "<-"
  ";"
] @punctuation.delimiter

[
  "<"
  ">"
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

; Keywords

[
  "as"
  (externtarget)
  "forall"
  "handle"
  "handler"
  "in"
  "infix"
  "infixl"
  "infixr"
  "mask"
  (behindmod)
  (pub)
  "some"
] @keyword

; Lazy constructor
(constructor
  "lazy" @keyword)

; Lazy match
(matchexpr
  "lazy" @keyword)

[
  (con)
  "ctl"
  "fn"
  "fun"
] @keyword.function

"with" @keyword.control

[
  "elif"
  "else"
  "if"
  "match"
  "then"
] @keyword.control.conditional

[
  "import"
  ;"include"
  "module"
] @keyword.control.import

[
  "alias"
  "effect"
  "struct"
  "type"
  "val"
  "var"
] @keyword.storage.type

[
  "abstract"
  "extern"
  "final"
  (inlinemod)
  (externinline)
  (typemod)
  (structmod)
  (effectmod)
  "named"
  (override)
  (controlmod)
  ;"scoped" ; scoped is actually an effect modifier, but it is not in the current parser.
  (tailmod)
] @keyword.storage.modifier

(fipmod
  ["fip" "fbip"] @keyword.storage.modifier)

"return" @keyword.control.return

; Operators

[
  "!"
  "~"
  "="
  ":="
  (idop)
  (op)
  (qidop)
] @operator

(modulepath) @namespace

; Variables

(pattern
  (identifier
    (varid) @variable))

(paramid
  (identifier
    (varid) @variable.parameter))

(pparameter
  (pattern
    (identifier
      (varid) @variable.parameter)))

(pparameter
  (qimplicit) @variable.parameter)

(puredecl
  (binder
    (qidentifier) @constant))

; Named arguments
(argument
  [(identifier) (qimplicit)] @variable.parameter
  "="
  (expr))

; Types

(typecon
  [(varid) (qvarid)] @type)

(tbinder
  (varid) @type)

(typeid
  (varid) @type)

(typedecl
  "effect"
  (varid) @type)

; Function definitions

(fundecl
  (identifier) @function)

(puredecl
  (qidentifier) @function)

(externdecl
  (qidentifier) @function)

; Effect definitions/usages

(opclause
  (qidentifier) @function)

(operation
  (identifier) @function)
  

; Function calls

(opexpr
  (atom
    (name) @function)
  .
  [
    call: "(" (arguments)? ")"
    trailing_lambda: [(block) (fnexpr)]
  ])

(opexpr
  (atom)
  (name) @function)

(ntlexpr
  (atom
    (name) @function)
  .
  ("(" (arguments)? ")"))

(ntlexpr
  (atom)
  (name) @function)

[(conid) (qconid)] @constructor

[
  "initially"
  "finally"
] @function.builtin
