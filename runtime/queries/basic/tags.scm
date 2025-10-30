(for_statement
  variable: (identifier) @name.definition.variable)

(let_statement
  variable: (identifier) @name.definition.variable)

(function_call
  function: (_) @name.reference.call)
