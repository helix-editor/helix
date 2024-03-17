;; Support for high-level text objects selections.
;; For instance:
;;    vaf     (v)isually select (a) (f)unction or subprogram
;;    vif     (v)isually select (i)nside a (f)unction or subprogram
;;    vai     (v)isually select (a) (i)f statement (or loop)
;;    vii     (v)isually select (i)nside an (i)f statement (or loop)
;;
;; https://github.com/nvim-treesitter/nvim-treesitter-textobjects/blob/master/README.md

(subprogram_body) @function.around
(subprogram_body (non_empty_declarative_part) @function.inside)
(subprogram_body (handled_sequence_of_statements) @function.inside)
(function_specification) @function.around
(procedure_specification) @function.around
(package_declaration) @function.around
(generic_package_declaration) @function.around
(package_body) @function.around
(if_statement) @object.around
(if_statement statements: (_) @object.inside)
(if_statement else_statements: (_) @object.inside)
(elsif_statement_item statements: (_) @object.inside)
(loop_statement) @object.around
(loop_statement statements: (_) @object.inside)
