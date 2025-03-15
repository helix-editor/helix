; TLA⁺ scopes and definitions
[
  (bounded_quantification)
  (choose)
  (function_definition) 
  (function_literal)
  (lambda) 
  (let_in)
  (module) 
  (module_definition)
  (operator_definition)
  (set_filter)
  (set_map)
  (unbounded_quantification)
] @local.scope

; Definitions
(choose (identifier) @local.definition.variable.parameter)
(choose (tuple_of_identifiers (identifier) @local.definition.variable.parameter))
(constant_declaration (identifier) @local.definition.constant)
(constant_declaration (operator_declaration name: (_) @local.definition.constant))
(function_definition name: (identifier) @local.definition.function)
(lambda (identifier) @local.definition.function)
(module_definition name: (_) @local.definition.namespace)
(module_definition parameter: (identifier) @local.definition.variable.parameter)
(module_definition parameter: (operator_declaration name: (_) @local.definition.variable.parameter))
(operator_definition name: (_) @local.definition.operator)
(operator_definition parameter: (identifier) @local.definition.variable.parameter)
(operator_definition parameter: (operator_declaration name: (_) @local.definition.variable.parameter))
(quantifier_bound (identifier) @local.definition.variable.parameter)
(quantifier_bound (tuple_of_identifiers (identifier) @local.definition.variable.parameter))
(unbounded_quantification (identifier) @local.definition.variable.parameter)
(variable_declaration (identifier) @local.definition.variable.builtin)

; Proof scopes and definitions
[
  (non_terminal_proof)
  (suffices_proof_step)
  (theorem)
] @local.scope

(assume_prove (new (identifier) @local.definition.variable.parameter))
(assume_prove (new (operator_declaration name: (_) @local.definition.variable.parameter)))
(assumption name: (identifier) @local.definition.constant)
(pick_proof_step (identifier) @local.definition.variable.parameter)
(take_proof_step (identifier) @local.definition.variable.parameter)
(theorem name: (identifier) @local.definition.constant)

;PlusCal scopes and definitions
[
  (pcal_algorithm)
  (pcal_macro)
  (pcal_procedure)
  (pcal_with)
] @local.scope

(pcal_macro_decl parameter: (identifier) @local.definition.variable.parameter)
(pcal_proc_var_decl (identifier) @local.definition.variable.parameter)
(pcal_var_decl (identifier) @local.definition.variable.parameter)
(pcal_with (identifier) @local.definition.variable.parameter)

; References
(identifier_ref) @local.reference
((prefix_op_symbol) @local.reference)
(bound_prefix_op symbol: (_) @local.reference)
((infix_op_symbol) @local.reference)
(bound_infix_op symbol: (_) @local.reference)
((postfix_op_symbol) @local.reference)
(bound_postfix_op symbol: (_) @local.reference)
(bound_nonfix_op symbol: (_) @local.reference)
