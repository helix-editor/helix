; References
[
  (python_identifier)
  (identifier)
] @reference

; Imports
(aliased_import
  alias: (python_identifier) @definition.import)
(import_statement
  name: (dotted_name ((python_identifier) @definition.import)))
(import_from_statement
  name: (dotted_name ((python_identifier) @definition.import)))

; Function with parameters, defines parameters
(parameters
  (python_identifier) @definition.parameter)

(default_parameter
  (python_identifier) @definition.parameter)

(typed_parameter
  (python_identifier) @definition.parameter)

(typed_default_parameter
  (python_identifier) @definition.parameter)

; *args parameter
(parameters
  (list_splat_pattern
    (python_identifier) @definition.parameter))

; **kwargs parameter
(parameters
  (dictionary_splat_pattern
    (python_identifier) @definition.parameter))

; Function defines function and scope
((python_function_definition
  name: (python_identifier) @definition.function) @scope
 (#set! definition.function.scope "parent"))

(function_definition (identifier) @definition.function)

(anonymous_python_function (identifier) @definition.function)

;;; Loops
; not a scope!
(for_statement
  left: (pattern_list
          (python_identifier) @definition.var))
(for_statement
  left: (tuple_pattern
          (python_identifier) @definition.var))
(for_statement
  left: (python_identifier) @definition.var)

; not a scope!
;(while_statement) @scope

; for in list comprehension
(for_in_clause
  left: (python_identifier) @definition.var)
(for_in_clause
  left: (tuple_pattern
          (python_identifier) @definition.var))
(for_in_clause
  left: (pattern_list
          (python_identifier) @definition.var))

(dictionary_comprehension) @scope
(list_comprehension) @scope
(set_comprehension) @scope

;;; Assignments

(assignment
 left: (python_identifier) @definition.var)

(assignment
 left: (pattern_list
   (python_identifier) @definition.var))
(assignment
 left: (tuple_pattern
   (python_identifier) @definition.var))

(assignment
 left: (attribute
   (python_identifier)
   (python_identifier) @definition.field))

(variable_assignment (identifier) operator: [ "=" "?=" "??=" ":=" ] @definition.var)

; Walrus operator  x := 1
(named_expression
  (python_identifier) @definition.var)

(as_pattern
  alias: (as_pattern_target) @definition.var)
