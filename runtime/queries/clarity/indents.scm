; Definitions
[
  (trait_definition)
  (trait_implementation)
  (trait_usage)
  (fungible_token_definition)
  (non_fungible_token_definition)
  (constant_definition)
  (variable_definition)
  (mapping_definition)
  (private_function)
  (read_only_function)
  (public_function)
] @indent

; Function calls
[
  (basic_native_form)
  (contract_function_call)
  (let_expression)
] @indent

; Bindings and signatures
[
  (local_binding)
  (function_signature)
  (function_parameter)
  (function_signature_for_trait)
] @indent

; Literals
[
  (list_lit)
  (some_lit)
  (response_lit)
  (tuple_lit)
] @indent

; Types
[
  (buffer_type)
  (ascii_string_type)
  (utf8_string_type)
  (list_type)
  (optional_type)
  (tuple_type)
  (tuple_type_for_trait)
  (response_type)
] @indent

; Closing delimiters
[")" "}"] @outdent
