[
  (loop_generate_construct)
  (loop_statement)
  (conditional_statement)
  (case_item)
  (function_declaration)
  (always_construct)
  (module_declaration)
] @scope

(parameter_declaration
 (list_of_param_assignments
  (param_assignment
   (parameter_identifier
    (simple_identifier) @local.definition.variable.parameter))))

(local_parameter_declaration
 (list_of_param_assignments
  (param_assignment
   (parameter_identifier
    (simple_identifier) @local.definition.variable.parameter))))

;; TODO: fixme
;(function_declaration
 ;(function_identifier
  ;(simple_identifier) @local.definition.function))

(function_declaration
 (function_body_declaration
  (function_identifier
   (function_identifier
    (simple_identifier) @local.definition.function))))

(tf_port_item1
 (port_identifier
  (simple_identifier) @local.definition.variable.parameter))

; too broad, now includes types etc
(simple_identifier) @local.reference
