;; Harneet Programming Language - Text Objects for Helix Editor
;; Tree-sitter text object queries for better navigation

;; Function definitions
(function_declaration) @function.around
(function_declaration body: (block) @function.inside)

;; Function calls
(call_expression) @function.around

;; Classes/Types (if Harneet adds struct support in future)
(type_declaration) @class.around
(type_declaration body: (struct_type) @class.inside)

;; Comments
(line_comment) @comment.around
(block_comment) @comment.around
(block_comment) @comment.inside

;; Parameters
(parameter_list) @parameter.around
(parameter_list "," @parameter.inside)

;; Arguments
(argument_list) @parameter.around 
(argument_list "," @parameter.inside)

;; Blocks
(block) @block.around
(block "{" "}" @block.inside)

;; If statements
(if_statement) @conditional.around
(if_statement condition: (_) @conditional.inside)

;; For loops
(for_statement) @loop.around
(for_statement body: (block) @loop.inside)

;; Switch statements
(switch_statement) @conditional.around
(switch_statement body: (block) @conditional.inside)

;; Case clauses
(case_clause) @conditional.around
(case_clause body: (_) @conditional.inside)