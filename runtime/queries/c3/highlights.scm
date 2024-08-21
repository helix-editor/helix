;; NOTE In this file later patterns are assumed to have priority!

;; Punctuation
["(" ")" "[" "]" "{" "}" "(<" ">)" "[<" ">]" "{|" "|}"] @punctuation.bracket
[";" "," ":" "::"] @punctuation.delimiter

;; Constant
(const_ident) @constant
["true" "false"] @constant.builtin.boolean
["null"] @constant.builtin

;; Variable
[(ident) (ct_ident)] @variable
;; 1) Member
(field_expr field: (access_ident (ident) @variable.other.member))
(struct_member_declaration (ident) @variable.other.member)
(struct_member_declaration (identifier_list (ident) @variable.other.member))
(bitstruct_member_declaration (ident) @variable.other.member)
(initializer_list (arg (param_path (param_path_element (ident) @variable.other.member))))
;; 2) Parameter
(parameter name: (_) @variable.parameter)
(call_invocation (arg (param_path (param_path_element [(ident) (ct_ident)] @variable.parameter))))
(enum_param_declaration (ident) @variable.parameter)
;; 3) Declaration
(global_declaration (ident) @variable)
(local_decl_after_type name: [(ident) (ct_ident)] @variable.other.member)
(var_decl name: [(ident) (ct_ident)] @variable)
(try_unwrap (ident) @variable)
(catch_unwrap (ident) @variable)

;; Keyword (from `c3c --list-keywords`)
[
  "assert"
  "asm"
  "catch"
  "defer"
  "try"
  "var"
] @keyword

[
  "$alignof"
  "$and"
  "$append"
  "$assert"
  "$assignable"
  "$case"
  "$concat"
  "$default"
  "$defined"
  "$echo"
  "$else"
  "$embed"
  "$endfor"
  "$endforeach"
  "$endif"
  "$endswitch"
  "$eval"
  "$evaltype"
  "$error"
  "$exec"
  "$extnameof"
  "$feature"
  "$for"
  "$foreach"
  "$if"
  "$include"
  "$is_const"
  "$nameof"
  "$offsetof"
  "$or"
  "$qnameof"
  "$sizeof"
  "$stringify"
  "$switch"
  "$typefrom"
  "$typeof"
  "$vacount"
  "$vatype"
  "$vaconst"
  "$varef"
  "$vaarg"
  "$vaexpr"
  "$vasplat"
] @keyword.directive

"fn" @keyword.function
"macro" @keyword.function
"return" @keyword.control.return
"import" @keyword.control.import
"module" @keyword.storage.type

[
  "bitstruct"
  "def"
  "distinct"
  "enum"
  "fault"
  "interface"
  "struct"
  "union"
] @keyword.storage.type

[
  "case"
  "default"
  "else"
  "if"
  "nextcase"
  "switch"
] @keyword.control.conditional

[
  "break"
  "continue"
  "do"
  "for"
  "foreach"
  "foreach_r"
  "while"
] @keyword.control.repeat

[
  "const"
  "extern"
  "inline"
  "static"
  "tlocal"
] @keyword.storage.modifier

;; Operator (from `c3c --list-operators`)
[
  "&"
  "!"
  "~"
  "|"
  "^"
  ;; ":"
  ;; ","
  ;; ";"
  "="
  ">"
  "/"
  "."
  ;; "#"
  "<"
  ;; "{"
  ;; "["
  ;; "("
  "-"
  "%"
  "+"
  "?"
  ;; "}"
  ;; "]"
  ;; ")"
  "*"
  ;; "_"
  "&&"
  ;; "->"
  "!!"
  "&="
  "|="
  "^="
  "/="
  ".."
  "?:"
  "=="
  ">="
  "=>"
  "<="
  ;; "{|"
  ;; "(<"
  ;; "[<"
  "-="
  "--"
  "%="
  "*="
  "!="
  "||"
  "+="
  "++"
  ;; "|}"
  ;; ">)"
  ;; ">]"
  "??"
  ;; "::"
  "<<"
  ">>"
  "..."
  "<<="
  ">>="
] @keyword.operator

(range_expr ":" @keyword.operator)
(foreach_cond ":" @keyword.operator)

(ternary_expr
  [
    "?"
    ":"
  ] @keyword.control.conditional)

(elvis_orelse_expr
  [
    "?:"
    "??"
  ] @keyword.control.conditional)

;; Literal
(integer_literal) @type.builtin
(real_literal) @type.builtin
(char_literal) @type.builtin
(bytes_literal) @type.builtin

;; String
(string_literal) @string
(raw_string_literal) @string

;; Escape Sequence
(escape_sequence) @string

;; Builtin (constants)
((builtin) @constant.builtin (#match? @constant.builtin "_*[A-Z][_A-Z0-9]*"))

;; Type Property (from `c3c --list-type-properties`)
(type_access_expr (access_ident [(ident) "typeid"] @variable.builtin
                                (#any-of? @variable.builtin
                                          "alignof"
                                          "associated"
                                          "elements"
                                          "extnameof"
                                          "inf"
                                          "is_eq"
                                          "is_ordered"
                                          "is_substruct"
                                          "len"
                                          "max"
                                          "membersof"
                                          "min"
                                          "nan"
                                          "inner"
                                          "kindof"
                                          "names"
                                          "nameof"
                                          "params"
                                          "parentof"
                                          "qnameof"
                                          "returns"
                                          "sizeof"
                                          "values"
                                          ;; Extra token
                                          "typeid")))

;; Label
[
  (label)
  (label_target)
] @keyword.storage.type

;; Module
(module_resolution (ident) @keyword.storage.type)
(module (path_ident (ident) @keyword.storage.type))
(import_declaration (path_ident (ident) @keyword.storage.type))

;; Attribute
(attribute name: (_) @attribute)
(define_attribute name: (_) @attribute)
(call_inline_attributes (at_ident) @attribute)
(asm_block_stmt (at_ident) @attribute)

;; Type
[
  (type_ident)
  (ct_type_ident)
] @type
(base_type_name) @type.builtin

;; Function Definition
(func_header name: (_) @function)
(func_header method_type: (_) name: (_) @function.method)
;; NOTE macro_declaration can also have a func_header
(macro_header name: (_) @function)
(macro_header method_type: (_) name: (_) @function.method)

;; Function Call
(call_expr function: [(ident) (at_ident)] @function)
(call_expr function: [(builtin)] @function.builtin)
(call_expr function: (module_ident_expr ident: (_) @function))
(call_expr function: (trailing_generic_expr argument: (module_ident_expr ident: (_) @function)))
(call_expr function: (field_expr field: (access_ident [(ident) (at_ident)] @function.method))) ; NOTE Ambiguous, could be calling a method or function pointer
;; Method on type
(call_expr function: (type_access_expr field: (access_ident [(ident) (at_ident)] @function.method)))

;; Assignment
;; (assignment_expr left: (ident) @variable.member)
;; (assignment_expr left: (module_ident_expr (ident) @variable.member))
;; (assignment_expr left: (field_expr field: (_) @variable.member))
;; (assignment_expr left: (unary_expr operator: "*" @variable.member))
;; (assignment_expr left: (subscript_expr ["[" "]"] @variable.member))

;; (update_expr argument: (ident) @variable.member)
;; (update_expr argument: (module_ident_expr ident: (ident) @variable.member))
;; (update_expr argument: (field_expr field: (_) @variable.member))
;; (update_expr argument: (unary_expr operator: "*" @variable.member))
;; (update_expr argument: (subscript_expr ["[" "]"] @variable.member))

;; (unary_expr operator: ["--" "++"] argument: (ident) @variable.member)
;; (unary_expr operator: ["--" "++"] argument: (module_ident_expr (ident) @variable.member))
;; (unary_expr operator: ["--" "++"] argument: (field_expr field: (access_ident (ident)) @variable.member))
;; (unary_expr operator: ["--" "++"] argument: (subscript_expr ["[" "]"] @variable.member))

;; Asm
(asm_instr [(ident) "int"] @function.builtin)
(asm_expr [(ct_ident) (ct_const_ident)] @variable.builtin)

;; Comment
[
  (line_comment)
  (block_comment)
] @comment

(doc_comment) @comment.block.documentation
