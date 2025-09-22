;; Harneet Programming Language - Indentation Rules for Helix Editor
;; Tree-sitter indentation queries

;; Increase indentation for block-like constructs
[
  (block)
  (if_statement)
  (for_statement) 
  (switch_statement)
  (case_clause)
  (function_declaration)
] @indent

;; Decrease indentation for closing braces
[
  "}"
] @outdent

;; Special handling for switch cases
(case_clause) @indent
(case_clause "case") @outdent

;; Function parameters with multiple lines
(parameter_list
  "(" @indent
  ")" @outdent)

;; Function calls with multiple arguments
(argument_list
  "(" @indent  
  ")" @outdent)

;; Composite literals and array/slice literals
(composite_literal
  "{" @indent
  "}" @outdent)

;; Import declarations with multiple imports
(import_spec_list
  "(" @indent
  ")" @outdent)