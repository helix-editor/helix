; inherits: c

((foreach_statement body: (_) @_body) @indent.begin
 (#not-has-type? @_body compound_statement))

((foreach_instance_statement body: (_) @_body) @indent.begin
 (#not-has-type? @_body compound_statement))
