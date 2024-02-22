;; ----------------------------------------------------------------------------
;; Literals and comments

[
  (line_comment)
  (block_comment)
  (block_comment_content)
] @comment


;; ----------------------------------------------------------------------------
;; Punctuation

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
  "[|"
  "|]"
  "[<"
  ">]"
] @punctuation.bracket

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
  (infix_op)
  (prefix_op)
  (symbolic_op)
] @operator



(attribute) @attribute

[
  "if"
  "then"
  "else"
  "elif"
  "when"
  "match"
  "match!"
  "and"
  "or"
  "&&"
  "||"
  "then"
] @keyword.control.conditional

[
  "return"
  "return!"
] @keyword.control.return

[
  "for"
  "while"
] @keyword.control.return


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
  "internal"
  "mutable"
  "override"
  "private"
  "public"
  "rec"
] @keyword.storage.modifier

[
  "enum"
  "let"
  "let!"
  "member"
  "module"
  "namespace"
  "type"
] @keyword.storage

[
  "as"
  "assert"
  "begin"
  "default"
  "do"
  "do!"
  "done"
  "downcast"
  "downto"
  "end"
  "event"
  "field"
  "finally"
  "fun"
  "function"
  "get"
  "global"
  "inherit"
  "interface"
  "lazy"
  "new"
  "not"
  "null"
  "of"
  "param"
  "property"
  "set"
  "struct"
  "try"
  "upcast"
  "use"
  "use!"
  "val"
  "with"
  "yield"
  "yield!"
] @keyword

[
 "true"
 "false"
 "unit"
 ] @constant.builtin

[
 (type)
 (const)
] @constant

[
 (union_type_case)
 (rules (rule (identifier_pattern)))
] @type.enum

(fsi_directive_decl (string) @namespace)

[
  (import_decl (long_identifier))
  (named_module (long_identifier))  
  (namespace (long_identifier))  
  (named_module 
    name: (long_identifier) )
  (namespace 
    name: (long_identifier) )
] @namespace


(dot_expression
  base: (long_identifier_or_op) @variable.other.member
  field: (long_identifier_or_op) @function)

[
 ;;(value_declaration_left (identifier_pattern) ) 
 (function_declaration_left (identifier) ) 
 (call_expression (long_identifier_or_op (long_identifier)))
 ;;(application_expression (long_identifier_or_op (long_identifier)))
] @function

[
  (string)
  (triple_quoted_string)
] @string

[
  (int)
  (int16)
  (int32)
  (int64)
  (float)
  (decimal)
] @constant.numeric


