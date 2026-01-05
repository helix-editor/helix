[(code_comment) (layout_comment)] @comment.block

(do_decl "do" @keyword.control)
(val_decl "val" @keyword.storage.type)
(fun_decl ["fun" "and"] @keyword.storage.type)
(datatype_decl ["datatype" "and"] @keyword.storage.type)
(datatype_arm "of" @keyword.storage.type)
(object_decl ["object" "of"] @keyword.storage.type)
(type_decl "type" @keyword.storage.type)
(local_decl ["local" "in" "end"] @keyword.storage.modifier)
(open_decl "open" @keyword.control)
(import_decl "import" @keyword.control.import)
(with_expr ["with" "without"] @keyword.operator)
(orelse_expr ["orelse" "otherwise"] @keyword.operator)
(andalso_expr ["andalso" "andthen"] @keyword.operator)
(fn_expr ["fn" "as"] @keyword.function)
(if_expr ["if" "then" "else"] @keyword.control.conditional)
(case_expr ["case" "of"] @keyword.control)
(fail_expr "fail" @keyword.control)
(let_expr ["let" "in" "end"] @keyword.storage.modifier)
(pat "as" @keyword.operator)

(boolean_lit) @constant.builtin.boolean
(numeric_lit) @constant.numeric.integer
(float_lit) @constant.numeric.float
(string_lit) @string.quoted.double
(backslash_escape) @constant.character.escape

["=" ":" ","] @punctuation.delimiter
["->" "=>"] @operator
["(" ")" "[" "]" "{" "}"] @punctuation.bracket

[(ident) (label)] @variable.other
(type_ident) @type
(atomic_pat (ident) @variable.other)
(pat (app_pat (atomic_pat (ident) @variable.parameter)))

(type_var) @type.parameter
(atomic_type (type_ident) @type.builtin)
[(record_type) (product_type) (app_type) (arrow_type)] @type

(atomic_expr (ident) @variable.other)
(project_expr) @variable.member
(field_binding (label) @variable.member.private)
(field_binding (expr) @variable.other)
(record_pat (ident) @variable.member.private)

(app_expr
  (app_expr (atomic_expr (ident) @function.call))
  (atomic_expr))
(app_expr
  _ 
  [":=" "@" "::" "o" "==" "!=" "==." "!=." 
   "<" "<=" ">" ">=" "<." "<=." ">." ">=."
   "+" "-" "+." "-." "*" "*." "/" "/." "div" "mod"
   "shl" "shr" "andb" "xorb" "orb"] @operator
  _)

(app_expr
  (app_expr (atomic_expr (ident) @function.builtin (#eq? @function.builtin "b")))
  (atomic_expr (layout_lit (layout_content (layout_text) @markup.bold))))
(app_expr
  (app_expr (atomic_expr (ident) @function.builtin (#eq? @function.builtin "it")))
  (atomic_expr (layout_lit (layout_content (layout_text) @markup.italic))))
(app_expr
  (app_expr (atomic_expr (ident) @function.builtin (#eq? @function.builtin "rm")))
  (atomic_expr (layout_lit)))
(app_expr
  (app_expr (atomic_expr (ident) @function.builtin (#match? @function.builtin "^(tt|courier|fixedersys)$")))
  (atomic_expr (layout_lit (layout_content (layout_text) @markup.raw.inline))))
(app_expr
  (app_expr (atomic_expr (ident) @function.builtin (#eq? @function.builtin "title")))
  (atomic_expr (layout_lit (layout_content (layout_text) @markup.heading.1))))
(app_expr
  (app_expr (atomic_expr (ident) @function.builtin (#eq? @function.builtin "section")))
  (atomic_expr (layout_lit (layout_content (layout_text) @markup.heading.2))))
(app_expr
  (app_expr (atomic_expr (ident) @function.builtin (#eq? @function.builtin "subsection")))
  (atomic_expr (layout_lit (layout_content (layout_text) @markup.heading.3))))
(app_expr
  (app_expr (atomic_expr (ident) @function.builtin (#eq? @function.builtin "subsubsection")))
  (atomic_expr (layout_lit (layout_content (layout_text) @markup.heading.4))))
(app_expr
  (app_expr (atomic_expr (ident) @function.builtin (#eq? @function.builtin "paragraph")))
  (atomic_expr (layout_lit (layout_content (layout_text) @markup.heading.5))))
(app_expr
  (app_expr (atomic_expr (ident) @function.builtin (#eq? @function.builtin "blockquote")))
  (atomic_expr (layout_lit (layout_content (layout_text) @markup.quote))))
