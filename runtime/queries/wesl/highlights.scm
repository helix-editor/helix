; reserved: must not be used in source code. https://www.w3.org/TR/WGSL/#reserved-words

; ((identifier) @special
;   (#any-of? @special
;   "NULL" "Self" "abstract" "active" "alignas" "alignof" "as" "asm"
; "asm_fragment" "async" "attribute" "auto" "await" "become" "binding_array"
; "cast" "catch" "class" "co_await" "co_return" "co_yield" "coherent"
; "column_major" "common" "compile" "compile_fragment" "concept" "const_cast"
; "consteval" "constexpr" "constinit" "crate" "debugger" "decltype" "delete"
; "demote" "demote_to_helper" "do" "dynamic_cast" "enum" "explicit" "export"
; "extends" "extern" "external" "fallthrough" "filter" "final" "finally" "friend"
; "from" "fxgroup" "get" "goto" "groupshared" "highp" "impl" "implements" "import"
; "inline" "instanceof" "interface" "layout" "lowp" "macro" "macro_rules" "match"
; "mediump" "meta" "mod" "module" "move" "mut" "mutable" "namespace" "new"
; "nil" "noexcept" "noinline" "nointerpolation" "non_coherent" "noncoherent"
; "noperspective" "null" "nullptr" "of" "operator" "package" "packoffset"
; "partition" "pass" "patch" "pixelfragment" "precise" "precision" "premerge"
; "priv" "protected" "pub" "public" "readonly" "ref" "regardless" "register"
; "reinterpret_cast" "require" "resource" "restrict" "self" "set" "shared"
; "sizeof" "smooth" "snorm" "static" "static_assert" "static_cast" "std"
; "subroutine" "super" "target" "template" "this" "thread_local" "throw" "trait"
; "try" "type" "typedef" "typeid" "typename" "typeof" "union" "unless" "unorm"
; "unsafe" "unsized" "use" "using" "varying" "virtual" "volatile" "wgsl" "where"
; "with" "writeonly" "yield"))

; comments

(line_comment) @comment.line
(block_comment) @comment.block

; imports (WESL extension)

(import_item (identifier) @type
  (#match? @type "^[A-Z]"))

(import_item (identifier) @constant
  (#match? @constant "^[A-Z0-9_]+$"))

(import_item (identifier) @namespace)

(import_path (identifier) @namespace)

(ident_path (identifier) @namespace)

; types

((identifier) @constant
  (#match? @constant "^[A-Z0-9_]+$"))

((identifier) @type
  (#match? @type "^[A-Z]"))

(type_specifier
    (identifier) @type)

; functions

(function_decl 
  (function_header
    (identifier) @function))

(call_expression
  (identifier) @function)

; templates

(template_list) @punctuation

(variable_decl ; this is var<storage> et.al
  (template_list
    (identifier) @keyword.storage.modifier))

(type_specifier
  (template_list
    (identifier) @type))

(template_list
  (template_list
    (identifier) @type))

; attributes

(attribute
  (identifier) @attribute) @attribute

(attribute
  (identifier) @attr-name
  (argument_list
    (identifier) @variable.builtin)
  (#eq? @attr-name "builtin"))

; variables, names

(param
  (identifier) @variable.parameter)
(variable_decl
  (identifier) @variable)
(const_assert_statement) @variable

(struct_decl
  (identifier) @type)

(struct_member
  name: (_) @variable.other.member)

(named_component_expression
  component: (_) @variable.other.member)

(identifier) @variable

; literals

(bool_literal) @constant.builtin.boolean
(int_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float


; keywords

[
  "if"
  "else"
] @keyword.control.conditional
[
  "loop"
  "for"
  "while"
  "break"
  "continue"
] @keyword.control.repeat
[
  "return"
] @keyword.control.return
[
  "switch"
  "case"
  "default"
  "discard"
] @keyword.control
[ ; WESL import extension
  "import"
  "as"
] @keyword.control.import
[
  "fn"
] @keyword.function
[
  "var"
  "let"
  "const"
  "struct"
] @keyword.storage.type
[
  "alias"
  "virtual" ; Bevy / naga_oil extension
  "override" ; Bevy / naga_oil extension
] @keyword

; expressions

[
  "-" "!" "~" "*" "&" ; unary
  "^" "|" "/" "%" "+" (shift_left) (shift_right) ; binary
  (less_than) (greater_than) (less_than_equal) (greater_than_equal) "==" "!=" ; relational
  "+=" "-=" "*=" "/=" "%=" "|=" "^=" "++" "--" "=" ; assign
  "->" ; return
] @operator

; punctuation

[ "(" ")" "[" "]" "{" "}" ] @punctuation.bracket
[ "," "." ":" ";" ] @punctuation.delimiter

; preprocessor

[ (preproc_directive) "#import" ] @keyword.directive
