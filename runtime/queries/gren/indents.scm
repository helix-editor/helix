((module_declaration) @indent)
((value_declaration) @indent)
((type_alias_declaration) @indent)
((type_declaration) @indent @extend)

((when_is_expr) @indent)
((when_is_branch) @indent)
((when_is_branch expr: (_)) @indent)

((let_in_expr "let") @indent @extend)

((if_else_expr "if") @indent)
((if_else_expr "if" (_) "then") @indent)
((if_else_expr "if" (_) "then" (_) "else") @indent)
