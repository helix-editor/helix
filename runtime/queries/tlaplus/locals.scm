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
(choose (identifier) @local.definition)
(choose (tuple_of_identifiers (identifier) @local.definition))
(constant_declaration (identifier) @local.definition)
(constant_declaration (operator_declaration name: (_) @local.definition))
(function_definition name: (identifier) @local.definition)
(lambda (identifier) @local.definition)
(module_definition name: (_) @local.definition)
(module_definition parameter: (identifier) @local.definition)
(module_definition parameter: (operator_declaration name: (_) @local.definition))
(operator_definition name: (_) @local.definition)
(operator_definition parameter: (identifier) @local.definition)
(operator_definition parameter: (operator_declaration name: (_) @local.definition))
(quantifier_bound (identifier) @local.definition)
(quantifier_bound (tuple_of_identifiers (identifier) @local.definition))
(unbounded_quantification (identifier) @local.definition)
(variable_declaration (identifier) @local.definition)

; Proof scopes and definitions
[
  (non_terminal_proof)
  (suffices_proof_step)
  (theorem)
] @local.scope

(assume_prove (new (identifier) @local.definition))
(assume_prove (new (operator_declaration name: (_) @local.definition)))
(assumption name: (identifier) @local.definition)
(pick_proof_step (identifier) @local.definition)
(take_proof_step (identifier) @local.definition)
(theorem name: (identifier) @local.definition)

;PlusCal scopes and definitions
[
  (pcal_algorithm)
  (pcal_macro)
  (pcal_procedure)
  (pcal_with)
] @local.scope

(pcal_macro_decl parameter: (identifier) @local.definition)
(pcal_proc_var_decl (identifier) @local.definition)
(pcal_var_decl (identifier) @local.definition)
(pcal_with (identifier) @local.definition)

; References
(identifier_ref) @local.reference
((prefix_op_symbol) @local.reference)
(bound_prefix_op symbol: (_) @local.reference)
((infix_op_symbol) @local.reference)
(bound_infix_op symbol: (_) @local.reference)
((postfix_op_symbol) @local.reference)
(bound_postfix_op symbol: (_) @local.reference)
(bound_nonfix_op symbol: (_) @local.reference)
