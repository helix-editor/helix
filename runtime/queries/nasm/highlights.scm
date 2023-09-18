(comment) @comment

(label) @label

(preproc_expression) @keyword.directive

[
  (line_here_token)
  (section_here_token)
] @variable.builtin

(unary_expression
  operator: _ @operator)
(binary_expression
  operator: _ @operator)
(conditional_expression
  "?" @operator
  ":" @operator)

[
  ":"
  ","
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(instruction_prefix) @keyword
(actual_instruction
  instruction: (word) @function)

(call_syntax_expression
  base: (word) @function)

(size_hint) @type
(struc_declaration
  name: (word) @type)
(struc_instance
  name: (word) @type)

(effective_address
 hint: _ @type)
(effective_address
 segment: _ @constant.builtin)

(register) @constant.builtin

(number_literal) @constant.numeric.integer
(string_literal) @string
(float_literal) @constant.numeric.float
(packed_bcd_literal) @constant.numeric.integer

((word) @constant
  (#match? @constant "^[A-Z_][?A-Z_0-9]+$"))
((word) @constant.builtin
  (#match? @constant.builtin "^__\\?[A-Z_a-z0-9]+\\?__$"))
(word) @variable

(preproc_arg) @keyword.directive

[
  (preproc_def)
  (preproc_function_def)
  (preproc_undef)
  (preproc_alias)
  (preproc_multiline_macro)
  (preproc_multiline_unmacro)
  (preproc_if)
  (preproc_rotate)
  (preproc_rep_loop)
  (preproc_include)
  (preproc_pathsearch)
  (preproc_depend)
  (preproc_use)
  (preproc_push)
  (preproc_pop)
  (preproc_repl)
  (preproc_arg)
  (preproc_stacksize)
  (preproc_local)
  (preproc_reporting)
  (preproc_pragma)
  (preproc_line)
  (preproc_clear)
] @keyword.directive
[
  (pseudo_instruction_dx)
  (pseudo_instruction_resx)
  (pseudo_instruction_incbin_command)
  (pseudo_instruction_equ_command)
  (pseudo_instruction_times_prefix)
  (pseudo_instruction_alignx_macro)
] @function.special
[
  (assembl_directive_target)
  (assembl_directive_defaults)
  (assembl_directive_sections)
  (assembl_directive_absolute)
  (assembl_directive_symbols)
  (assembl_directive_common)
  (assembl_directive_symbolfixes)
  (assembl_directive_cpu)
  (assembl_directive_floathandling)
  (assembl_directive_org)
  (assembl_directive_sectalign)

  (assembl_directive_primitive_target)
  (assembl_directive_primitive_defaults)
  (assembl_directive_primitive_sections)
  (assembl_directive_primitive_absolute)
  (assembl_directive_primitive_symbols)
  (assembl_directive_primitive_common)
  (assembl_directive_primitive_symbolfixes)
  (assembl_directive_primitive_cpu)
  (assembl_directive_primitive_floathandling)
  (assembl_directive_primitive_org)
  (assembl_directive_primitive_sectalign)
  (assembl_directive_primitive_warning)
  (assembl_directive_primitive_map)
] @keyword
