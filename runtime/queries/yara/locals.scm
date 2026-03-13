(rule_definition name: (identifier) @local.definition.string.special)
(string_identifier) @local.definition.string.special.symbol

(for_expression
  (string_identifier) @local.reference)

(for_of_expression
  (string_identifier) @local.reference)

(of_expression
  (string_set
    (string_identifier) @local.reference))

(string_count
  (string_identifier) @local.reference)

(string_offset
  (string_identifier) @local.reference)

(string_length
  (string_identifier) @local.reference)
