; Credits to nvim-treesitter/nvim-treesitter-context
(binary_operator
  left: (_)
  right: (_) @context)

(pair
  key: (_)
  value: (_) @context)

((unary_operator
   operand: (call
      target: (identifier)
      (arguments (_)))) @_op (#lua-match? @_op "@[%w_]+")) @context

(stab_clause) @context

(call) @context
