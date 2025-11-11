; Function definitions (defn, defn-, defmacro, defmethod, etc.)
(list_lit
  .
  (sym_lit) @_keyword
  .
  (sym_lit)
  (_)* @function.inside
  (#match? @_keyword "^(defn|defn-|defmacro|defmethod|defmulti|definline)$")) @function.around

; Anonymous functions (fn)
(list_lit
  .
  (sym_lit) @_fn
  (_)* @function.inside
  (#match? @_fn "^fn$")) @function.around

; Anonymous function shorthand #()
(anon_fn_lit
  (_)* @function.inside) @function.around

; deftype, defrecord, defprotocol
(list_lit
  .
  (sym_lit) @_keyword
  .
  (sym_lit)
  (_)* @class.inside
  (#match? @_keyword "^(deftype|defrecord|defprotocol|definterface|defstruct)$")) @class.around

; Test definitions (deftest)
(list_lit
  .
  (sym_lit) @_keyword
  .
  (sym_lit)
  (_)* @test.inside
  (#match? @_keyword "^deftest$")) @test.around

; Function parameters in vectors
(vec_lit
  (_)* @parameter.inside) @parameter.around

; List entries
(list_lit
  (_) @entry.inside @entry.around)

; Vector entries
(vec_lit
  (_) @entry.inside @entry.around)

; Map entries
(map_lit
  (_) @entry.inside @entry.around)

; Set entries
(set_lit
  (_) @entry.inside @entry.around)

; Comments
(comment) @comment.inside
(comment)+ @comment.around

; Discard expressions (also treated as comments)
(dis_expr) @comment.inside

; Comment special form (comment ...)
(list_lit
  .
  (sym_lit) @_comment
  (_)* @comment.inside
  (#match? @_comment "^comment$")) @comment.around
