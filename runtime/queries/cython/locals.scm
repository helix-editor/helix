; Program structure
(module) @local.scope

(class_definition
  body: (block
    (expression_statement
      (assignment
        left: (identifier) @local.definition)))) @local.scope

(class_definition
  body: (block
    (expression_statement
      (assignment
        left: (_
          (identifier) @local.definition))))) @local.scope

; Imports
(aliased_import
  alias: (identifier) @local.definition.namespace)

(import_statement
  name: (dotted_name
    (identifier) @local.definition.namespace))

(import_from_statement
  name: (dotted_name
    (identifier) @local.definition.namespace))

; Function with parameters, defines parameters
(parameters
  (identifier) @local.definition.variable.parameter)

(default_parameter
  (identifier) @local.definition.variable.parameter)

(typed_parameter
  (identifier) @local.definition.variable.parameter)

(typed_default_parameter
  (identifier) @local.definition.variable.parameter)

; *args parameter
(parameters
  (list_splat_pattern
    (identifier) @local.definition))

; **kwargs parameter
(parameters
  (dictionary_splat_pattern
    (identifier) @local.definition.variable.parameter))

(class_definition
  body: (block
    (function_definition
      name: (identifier) @local.definition.function)))

; Loops
; not a scope!
(for_in_loop
  left: (pattern_list
    (identifier) @local.definition.variable))

(for_in_loop
  left: (tuple_pattern
    (identifier) @local.definition.variable))

(for_in_loop
  left: (identifier) @local.definition.variable)

; not a scope!
;(while_statement) @local.scope
; for in list comprehension
(for_in_clause
  left: (identifier) @local.definition.variable)

(for_in_clause
  left: (tuple_pattern
    (identifier) @local.definition.variable))

(for_in_clause
  left: (pattern_list
    (identifier) @local.definition.variable))

(dictionary_comprehension) @local.scope

(list_comprehension) @local.scope

(set_comprehension) @local.scope

; Assignments
(assignment
  left: (identifier) @local.definition.variable)

(assignment
  left: (pattern_list
    (identifier) @local.definition.variable))

(assignment
  left: (tuple_pattern
    (identifier) @local.definition.variable))

(assignment
  left: (attribute
    (identifier)
    (identifier) @local.definition.variable))

; Walrus operator  x := 1
(named_expression
  (identifier) @local.definition.variable)

(as_pattern
  alias: (as_pattern_target) @local.definition.variable)

; REFERENCES
(identifier) @local.reference
