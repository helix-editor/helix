;; Literals

(integer_literal) @constant.numeric.integer
(natural_literal) @constant.numeric.integer
(double_literal) @constant.numeric.float
(boolean_literal) @constant.builtin.boolean
(text_literal) @string
(local_import) @string.special.path
(http_import) @string.special.url
(import_hash) @string

;; Comments
[
  (line_comment)
  (block_comment)
] @comment

;; Keywords
[
  ("let")
  ("in")
  (assign_operator)
  (type_operator)
  (lambda_operator)
  (arrow_operator)
  (infix_operator)
  (completion_operator)
  ("using")
  ("assert")
  (assert_operator)
  ("as")
  (forall_operator)
  ("with")
] @keyword

;; Builtins
[
  (builtin_function)
  (missing_import)
] @function.builtin

[ 
  (builtin)
  (import_as_text)
] @type.builtin

;; Conditionals
[
  ("if")
  ("then")
  ("else")
] @keyword.control.conditional
