;; Support for high-level text objects selections.
;; For instance:
;;    vaf     (v)isually select (a) (f)unction or subprogram
;;    vif     (v)isually select (i)nside a (f)unction or subprogram
;;    vai     (v)isually select (a) (i)f statement (or loop)
;;    vii     (v)isually select (i)nside an (i)f statement (or loop)
;;
;; https://github.com/nvim-treesitter/nvim-treesitter-textobjects/blob/master/README.md

(subprogram_body) @function.outer
(subprogram_body (non_empty_declarative_part) @function.inner)
(subprogram_body (handled_sequence_of_statements) @function.inner)
(function_specification) @function.outer
(procedure_specification) @function.outer
(package_declaration) @function.outer
(generic_package_declaration) @function.outer
(package_body) @function.outer
(if_statement) @block.outer
(if_statement statements: (_) @block.inner)
(if_statement else_statements: (_) @block.inner)
(elsif_statement_item statements: (_) @block.inner)
(loop_statement) @block.outer
(loop_statement statements: (_) @block.inner)
