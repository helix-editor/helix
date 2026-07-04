; tree-sitter highlighting resolves in .scm order. For a given byte
; offset, the last matching capture wins. This file is ordered from
; generic to specific: base variables and keywords first, then more
; targeted builtin / call / member patterns that override them.

; Base ----------------------------------------------------------------

; Generic identifier reference. Specific rules (builtins, calls,
; booleans, constants) later in the file override this.
(variable_expression (identifier) @variable)

; Comments ------------------------------------------------------------

(comment) @comment
(doc_comment) @comment.documentation

; Keywords ------------------------------------------------------------

[
  "assert"
  "in"
  "inherit"
  "let"
  "rec"
  "with"
] @keyword

[
  "if"
  "then"
  "else"
] @keyword.control.conditional

"or" @keyword.operator

; Literals ------------------------------------------------------------

(integer_expression) @constant.numeric.integer
(float_expression) @constant.numeric.float

[
  (string_expression)
  (indented_string_expression)
] @string

(escape_sequence) @constant.character.escape
(dollar_escape) @constant.character.escape

[
  (path_expression)
  (hpath_expression)
  (spath_expression)
] @string.special.path

(uri_expression) @string.special.url

; Functions -----------------------------------------------------------

; Parameters: `x: body` and `{ a, b }: body`.
(function_expression
  universal: (identifier) @variable.parameter)

(formal
  name: (identifier) @variable.parameter
  "?"? @punctuation.delimiter)

(ellipses) @variable.parameter.builtin

; Attrset members -----------------------------------------------------

(binding
  attrpath: (attrpath (identifier)) @variable.other.member)

(select_expression
  attrpath: (attrpath (identifier)) @variable.other.member)

(inherit attrs: (inherited_attrs attr: (identifier) @variable.other.member))
(inherit_from attrs: (inherited_attrs attr: (identifier) @variable.other.member))

; Function calls ------------------------------------------------------
; After member rules so `lib.isBool` in apply position is @function,
; not @variable.other.member.

(apply_expression
  function: [
    (variable_expression (identifier) @function)
    (select_expression
      attrpath: (attrpath
        attr: (identifier) @function .))])

; Pipe operators evaluate the side pointed to by the operator as a function:
; `value |> lib.foo` and `lib.foo <| value`.
(binary_expression
  operator: "|>"
  right: [
    (variable_expression (identifier) @function)
    (select_expression
      attrpath: (attrpath
        attr: (identifier) @function .))])

(binary_expression
  left: [
    (variable_expression (identifier) @function)
    (select_expression
      attrpath: (attrpath
        attr: (identifier) @function .))]
  operator: "<|")

(binding
  attrpath: (attrpath
    attr: (identifier) @function)
  expression: (function_expression))

; Builtins ------------------------------------------------------------

; `builtins.*` method-style calls: highlight the attr as a builtin
; function. `builtins` itself is painted by the @constant.builtin rule
; further down.
((select_expression
  expression: (variable_expression
    name: (identifier) @_id)
  attrpath: (attrpath
    attr: (identifier) @function.builtin))
 (#eq? @_id "builtins"))

; In apply position: `map f xs` -> `map` is a function builtin.
((apply_expression
  function: (variable_expression
    (identifier) @function.builtin))
 (#match? @function.builtin "^(__add|__addDrvOutputDependencies|__addErrorContext|__all|__any|__appendContext|__attrNames|__attrValues|__bitAnd|__bitOr|__bitXor|__catAttrs|__ceil|__compareVersions|__concatLists|__concatMap|__concatStringsSep|__convertHash|__deepSeq|__div|__elem|__elemAt|__fetchurl|__filter|__filterSource|__findFile|__flakeRefToString|__floor|__foldl'|__fromJSON|__functionArgs|__genList|__genericClosure|__getAttr|__getContext|__getEnv|__getFlake|__groupBy|__hasAttr|__hasContext|__hashFile|__hashString|__head|__intersectAttrs|__isAttrs|__isBool|__isFloat|__isFunction|__isInt|__isList|__isPath|__isString|__length|__lessThan|__listToAttrs|__mapAttrs|__match|__mul|__parseDrvName|__parseFlakeRef|__partition|__path|__pathExists|__readDir|__readFile|__readFileType|__replaceStrings|__seq|__sort|__split|__splitVersion|__storePath|__stringLength|__sub|__substring|__tail|__toFile|__toJSON|__toPath|__toXML|__trace|__traceVerbose|__tryEval|__typeOf|__unsafeDiscardOutputDependency|__unsafeDiscardStringContext|__unsafeGetAttrPos|__warn|__zipAttrsWith|baseNameOf|break|derivation|derivationStrict|dirOf|fetchGit|fetchMercurial|fetchTarball|fetchTree|fromTOML|isNull|map|placeholder|removeAttrs|scopedImport|toString)$"))

; `import` behaves like a keyword.
((variable_expression (identifier) @keyword.control.import)
 (#eq? @keyword.control.import "import"))

; `abort` / `throw` are control-flow exceptions.
((variable_expression (identifier) @keyword.control.exception)
 (#any-of? @keyword.control.exception "abort" "throw"))

; Booleans / constants ------------------------------------------------

((variable_expression (identifier) @constant.builtin.boolean)
 (#any-of? @constant.builtin.boolean "true" "false"))

((variable_expression (identifier) @constant.builtin)
 (#any-of? @constant.builtin
   "builtins" "null"
   "__curPos" "__currentSystem" "__currentTime" "__langVersion"
   "__nixPath" "__nixVersion" "__storeDir"))

; Operators -----------------------------------------------------------

(unary_expression
  operator: _ @operator)

(binary_expression
  operator: _ @operator)

[
  "="
  "@"
] @operator

; Punctuation ---------------------------------------------------------

[
  ";"
  "."
  ","
  ":"
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(interpolation
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

(has_attr_expression
  expression: (_)
  "?" @operator
  attrpath: (attrpath
    attr: (identifier) @variable.other.member))
