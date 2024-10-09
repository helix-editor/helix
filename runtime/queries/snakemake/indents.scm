; inherits: python

((rule_definition) @indent.begin
  (#set! indent.immediate 1))

((checkpoint_definition) @indent.begin
  (#set! indent.immediate 1))

((rule_inheritance) @indent.begin
  (#set! indent.immediate 1))

((rule_import "with" ":") @indent.begin
  (#set! indent.immediate 1))

((module_definition) @indent.begin
  (#set! indent.immediate 1))

((directive) @indent.begin
  (#set! indent.immediate 1))

; end indentation after last parameter node (no following ',')
(directive_parameters (_)* (_) @indent.end)
