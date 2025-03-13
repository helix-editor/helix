; ; Intended for consumption by GitHub and the tree-sitter highlight command
; ; Default capture names found here:
; ; https://github.com/tree-sitter/tree-sitter/blob/f5d1c0b8609f8697861eab352ead44916c068c74/cli/src/highlight.rs#L150-L171
; ; In this file, captures defined earlier take precedence over captures defined later.

; TLA⁺ Keywords
[
  "ACTION"
  "ASSUME"
  "ASSUMPTION"
  "AXIOM"
  "BY"
  "CASE"
  "CHOOSE"
  "CONSTANT"
  "CONSTANTS"
  "COROLLARY"
  "DEF"
  "DEFINE"
  "DEFS"
  "ELSE"
  "EXCEPT"
  "EXTENDS"
  "HAVE"
  "HIDE"
  "IF"
  "IN"
  "INSTANCE"
  "LAMBDA"
  "LEMMA"
  "LET"
  "LOCAL"
  "MODULE"
  "NEW"
  "OBVIOUS"
  "OMITTED"
  "ONLY"
  "OTHER"
  "PICK"
  "PROOF"
  "PROPOSITION"
  "PROVE"
  "QED"
  "RECURSIVE"
  "SF_"
  "STATE"
  "SUFFICES"
  "TAKE"
  "TEMPORAL"
  "THEN"
  "THEOREM"
  "USE"
  "VARIABLE"
  "VARIABLES"
  "WF_"
  "WITH"
  "WITNESS"
  (address)
  (all_map_to)
  (assign)
  (case_arrow)
  (case_box)
  (def_eq)
  (exists)
  (forall)
  (gets)
  (label_as)
  (maps_to)
  (set_in)
  (temporal_exists)
  (temporal_forall)
] @keyword

;  PlusCal keywords
[
  "algorithm"
  "assert"
  "await"
  "begin"
  "call"
  "define"
  "either"
  "else"
  "elsif"
  "end"
  "fair"
  "goto"
  "if"
  "macro"
  "or"
  "print"
  "procedure"
  "process"
  "variable"
  "variables"
  "when"
  "with"
  "then"
  (pcal_algorithm_start)
  (pcal_end_either)
  (pcal_end_if)
  (pcal_return)
  (pcal_skip)
  (pcal_process ("="))
  (pcal_with ("="))
] @keyword

; Literals
(binary_number (format) @keyword)
(binary_number (value) @constant.numeric)
(boolean) @constant.builtin.boolean
(boolean_set) @type
(hex_number (format) @keyword)
(hex_number (value) @constant.numeric)
(int_number_set) @type
(nat_number) @constant.numeric.integer
(nat_number_set) @type
(octal_number (format) @keyword)
(octal_number (value) @constant.numeric)
(real_number) @constant.numeric.integer
(real_number_set) @type
(string) @string
(escape_char) @string.special.symbol
(string_set) @type

; Namespaces and includes
(extends (identifier_ref) @namespace)
(instance (identifier_ref) @namespace)
(module name: (_) @namespace)
(pcal_algorithm name: (identifier) @namespace)

; Constants and variables
(constant_declaration (identifier) @constant)
(constant_declaration (operator_declaration name: (_) @constant))
(pcal_var_decl (identifier) @variable.builtin)
(pcal_with (identifier) @variable.parameter)
((".") . (identifier) @attribute)
(record_literal (identifier) @attribute)
(set_of_records (identifier) @attribute)
(variable_declaration (identifier) @variable.builtin)

; Parameters
(choose (identifier) @variable.parameter)
(choose (tuple_of_identifiers (identifier) @variable.parameter))
(lambda (identifier) @variable.parameter)
(module_definition (operator_declaration name: (_) @variable.parameter))
(module_definition parameter: (identifier) @variable.parameter)
(operator_definition (operator_declaration name: (_) @variable.parameter))
(operator_definition parameter: (identifier) @variable.parameter)
(pcal_macro_decl parameter: (identifier) @variable.parameter)
(pcal_proc_var_decl (identifier) @variable.parameter)
(quantifier_bound (identifier) @variable.parameter)
(quantifier_bound (tuple_of_identifiers (identifier) @variable.parameter))
(unbounded_quantification (identifier) @variable.parameter)

; Operators, functions, and macros
(function_definition name: (identifier) @function)
(module_definition name: (_) @namespace)
(operator_definition name: (_) @operator)
(pcal_macro_decl name: (identifier) @function)
(pcal_macro_call name: (identifier) @function)
(pcal_proc_decl name: (identifier) @function)
(pcal_process name: (identifier) @function)
(recursive_declaration (identifier) @operator)
(recursive_declaration (operator_declaration name: (_) @operator))

; Delimiters
[
  (langle_bracket)
  (rangle_bracket)
  (rangle_bracket_sub)
  "{"
  "}"
  "["
  "]"
  "]_"
  "("
  ")"
] @punctuation.bracket
[
  ","
  ":"
  "."
  "!"
  ";"
  (bullet_conj)
  (bullet_disj)
  (prev_func_val)
  (placeholder)
] @punctuation.delimiter

; Proofs
(assume_prove (new (identifier) @variable.parameter))
(assume_prove (new (operator_declaration name: (_) @variable.parameter)))
(assumption name: (identifier) @constant)
(pick_proof_step (identifier) @variable.parameter)
(proof_step_id "<" @punctuation.bracket)
(proof_step_id (level) @tag)
(proof_step_id (name) @tag)
(proof_step_id ">" @punctuation.bracket)
(proof_step_ref "<" @punctuation.bracket)
(proof_step_ref (level) @tag)
(proof_step_ref (name) @tag)
(proof_step_ref ">" @punctuation.bracket)
(take_proof_step (identifier) @variable.parameter)
(theorem name: (identifier) @constant)

; Comments and tags
(block_comment "(*" @comment.block)
(block_comment "*)" @comment.block)
(block_comment_text) @comment.block
(comment) @comment
(single_line) @comment
(_ label: (identifier) @tag)
(label name: (_) @tag)
(pcal_goto statement: (identifier) @tag)

; Put these last so they are overridden by everything else
(bound_infix_op symbol: (_) @function.builtin)
(bound_nonfix_op symbol: (_) @function.builtin)
(bound_postfix_op symbol: (_) @function.builtin)
(bound_prefix_op symbol: (_) @function.builtin)
((prefix_op_symbol) @function.builtin)
((infix_op_symbol) @function.builtin)
((postfix_op_symbol) @function.builtin)
