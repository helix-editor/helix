(comment) @comment

[
  ";"
  "."
  ","
  "="
  ":"
  (ellipses)
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

"assert" @keyword.control.exception
"or" @keyword.operator
"rec" @keyword.control.repeat

[
  "if" 
  "then"
  "else"
] @keyword.control.conditional

[
  "let"
  "inherit"
  "in"
  "with" 
] @keyword

(variable_expression name: (identifier) @variable)

(select_expression
  attrpath: (attrpath attr: (identifier)) @variable.other.member)

(apply_expression
  function: [
    (variable_expression name: (identifier) @function)
    (select_expression
      attrpath: (attrpath
        attr: (identifier) @function .))])

((identifier) @variable.builtin
 (#match? @variable.builtin "^(__currentSystem|__currentTime|__nixPath|__nixVersion|__storeDir|builtins)$")
 (#is-not? local))

((identifier) @function.builtin
 (#match? @function.builtin "^(__add|__addErrorContext|__all|__any|__appendContext|__attrNames|__attrValues|__bitAnd|__bitOr|__bitXor|__catAttrs|__compareVersions|__concatLists|__concatMap|__concatStringsSep|__deepSeq|__div|__elem|__elemAt|__fetchurl|__filter|__filterSource|__findFile|__foldl'|__fromJSON|__functionArgs|__genList|__genericClosure|__getAttr|__getContext|__getEnv|__hasAttr|__hasContext|__hashFile|__hashString|__head|__intersectAttrs|__isAttrs|__isBool|__isFloat|__isFunction|__isInt|__isList|__isPath|__isString|__langVersion|__length|__lessThan|__listToAttrs|__mapAttrs|__match|__mul|__parseDrvName|__partition|__path|__pathExists|__readDir|__readFile|__replaceStrings|__seq|__sort|__split|__splitVersion|__storePath|__stringLength|__sub|__substring|__tail|__toFile|__toJSON|__toPath|__toXML|__trace|__tryEval|__typeOf|__unsafeDiscardOutputDependency|__unsafeDiscardStringContext|__unsafeGetAttrPos|__valueSize|abort|baseNameOf|derivation|derivationStrict|dirOf|fetchGit|fetchMercurial|fetchTarball|fromTOML|import|isNull|map|placeholder|removeAttrs|scopedImport|throw|toString)$")
 (#is-not? local))

[
  (string_expression)
  (indented_string_expression)
] @string

[
  (path_expression)
  (hpath_expression)
  (spath_expression)
] @string.special.path

(uri_expression) @string.special.uri

; boolean
((identifier) @constant.builtin.boolean (#match? @constant.builtin.boolean "^(true|false)$")) @constant.builtin.boolean
; null
((identifier) @constant.builtin (#eq? @constant.builtin "null")) @constant.builtin

(integer_expression) @constant.numeric.integer
(float_expression) @constant.numeric.float

(escape_sequence) @constant.character.escape
(dollar_escape) @constant.character.escape

(function_expression
  "@"? @punctuation.delimiter
  universal: (identifier) @variable.parameter
  "@"? @punctuation.delimiter
)

(formal
  name: (identifier) @variable.parameter
  "?"? @punctuation.delimiter)

(interpolation
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

(unary_expression
  operator: _ @operator)

(binary_expression
  operator: _ @operator)

(binding
  attrpath: (attrpath attr: (identifier)) @variable.other.member)

(inherit_from attrs: (inherited_attrs attr: (identifier) @variable.other.member))
(inherited_attrs attr: (identifier) @variable)

(has_attr_expression
  expression: (_)
  "?" @operator
  attrpath: (attrpath
    attr: (identifier) @variable.other.member))
