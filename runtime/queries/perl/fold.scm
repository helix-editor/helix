(comment) @fold
(pod) @fold

; fold the block-typed package statements only
(package_statement (block)) @fold

[(subroutine_declaration_statement)
 (conditional_statement)
 (loop_statement)
 (for_statement)
 (cstyle_for_statement)
 (block_statement)
 (phaser_statement)] @fold

(anonymous_subroutine_expression) @fold

; perhaps folks want to fold these too?
[(anonymous_array_expression)
 (anonymous_hash_expression)] @fold
