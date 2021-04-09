(class_declaration
  name: (name) @name) @definition.class

(function_definition
  name: (name) @name) @definition.function

(method_declaration
  name: (name) @name) @definition.function

(object_creation_expression
  [
    (qualified_name (name) @name)
    (variable_name (name) @name)
  ]) @reference.class

(function_call_expression
  function: [
    (qualified_name (name) @name)
    (variable_name (name)) @name
  ]) @reference.call

(scoped_call_expression
  name: (name) @name) @reference.call

(member_call_expression
  name: (name) @name) @reference.call
