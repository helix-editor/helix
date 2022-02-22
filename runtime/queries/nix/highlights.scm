(comment) @comment

[
  "if"
  "then"
  "else"
  "let"
  "inherit"
  "in"
  "rec"
  "with"
  "assert"
] @keyword

((identifier) @variable.builtin
 (#match? @variable.builtin "^(__currentSystem|__currentTime|__nixPath|__nixVersion|__storeDir|builtins)$")
 (#is-not? local))

((identifier) @function.builtin
 (#match? @function.builtin "^(__add|__addErrorContext|__all|__any|__appendContext|__attrNames|__attrValues|__bitAnd|__bitOr|__bitXor|__catAttrs|__compareVersions|__concatLists|__concatMap|__concatStringsSep|__deepSeq|__div|__elem|__elemAt|__fetchurl|__filter|__filterSource|__findFile|__foldl'|__fromJSON|__functionArgs|__genList|__genericClosure|__getAttr|__getContext|__getEnv|__hasAttr|__hasContext|__hashFile|__hashString|__head|__intersectAttrs|__isAttrs|__isBool|__isFloat|__isFunction|__isInt|__isList|__isPath|__isString|__langVersion|__length|__lessThan|__listToAttrs|__mapAttrs|__match|__mul|__parseDrvName|__partition|__path|__pathExists|__readDir|__readFile|__replaceStrings|__seq|__sort|__split|__splitVersion|__storePath|__stringLength|__sub|__substring|__tail|__toFile|__toJSON|__toPath|__toXML|__trace|__tryEval|__typeOf|__unsafeDiscardOutputDependency|__unsafeDiscardStringContext|__unsafeGetAttrPos|__valueSize|abort|baseNameOf|derivation|derivationStrict|dirOf|fetchGit|fetchMercurial|fetchTarball|fromTOML|import|isNull|map|placeholder|removeAttrs|scopedImport|throw|toString)$")
 (#is-not? local))

[
  (string)
  (indented_string)
] @string

[
  (path)
  (hpath)
  (spath)
] @string.special.path

(uri) @string.special.uri

; boolean
((identifier) @constant.builtin.boolean (#match? @constant.builtin.boolean "^(true|false)$")) @constant.builtin.boolean
; null
((identifier) @constant.builtin (#eq? @constant.builtin "null")) @constant.builtin

(integer) @constant.numeric.integer
(float) @constant.numeric.float

(interpolation
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

(escape_sequence) @constant.character.escape

(function
  universal: (identifier) @variable.parameter
)

(formal
  name: (identifier) @variable.parameter
  "?"? @punctuation.delimiter)

(app
  function: [
    (identifier) @function
    (select
      attrpath: (attrpath
        attr: (attr_identifier) @function .))])


(unary
  operator: _ @operator)

(binary
  operator: _ @operator)

(attr_identifier) @variable.other.member
(inherit attrs: (attrs_inherited (identifier) @variable.other.member) )

[
  ";"
  "."
  ","
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(identifier) @variable
