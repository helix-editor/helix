[
  (import)
] @keyword.control.import

[
  (package)
] @namespace

[
  (with)
  (as)
  (every)
  (some)
  (in)
  (default)
  "null"
] @keyword.control

[
  (not)
  (if)
  (contains)
  (else)
] @keyword.control.conditional

[
  (boolean)
] @constant.builtin.boolean

[
  (assignment_operator)
  (bool_operator)
  (arith_operator)
  (bin_operator)
] @operator

[
  (string)
  (raw_string)
] @string

(term (ref (var))) @variable

(comment) @comment.line

(number) @constant.numeric.integer

(expr_call func_name: (fn_name (var) @function .))

(expr_call func_arguments: (fn_args (expr) @variable.parameter))

(rule_args (term) @variable.parameter)

[
  (open_paren)
  (close_paren)
  (open_bracket)
  (close_bracket)
  (open_curly)
  (close_curly)
] @punctuation.bracket

(rule (rule_head (var) @function.method))

(rule
  (rule_head (term (ref (var) @namespace)))
  (rule_body (query (literal (expr (expr_infix (expr (term (ref (var)) @_output)))))) (#eq? @_output @namespace))
)
