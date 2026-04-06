(
  [
    (compound_statement (block (string_literal (content) @injection.content)))
    (macrocall_expression
      (macro_identifier "@" (identifier) @doc (#eq? @doc "doc"))
      (macro_argument_list (string_literal (content) @injection.content)))
    (module_definition (block (string_literal (content) @injection.content)))
    (source_file (string_literal (content) @injection.content))
    (prefixed_string_literal
      (identifier) @markdown
      (content) @injection.content
      (#eq? @markdown "md"))
  ]
  (#set! injection.language "markdown"))

(
  (#set! injection.language "markdown"))

(
  [(line_comment) (block_comment)] @injection.content
  (#set! injection.language "comment"))

(
  (prefixed_string_literal
    (identifier) @html
    (content) @injection.content
    (#eq? @html "html"))
  (#set! injection.language "html"))
(macrocall_expression
  (macro_identifier "@" (identifier) @html_str (#eq? @html_str "html_str"))
  (macro_argument_list (string_literal (content) @injection.content))
  (#set! injection.language "html"))

(
  (prefixed_string_literal
    (identifier) @regex
    (content) @injection.content
    (#eq? @regex "r"))
  (#set! injection.language "regex"))

((command_literal (content) @injection.content (#set! injection.language "bash")))

; GraphQLClient.jl
; (
;   (prefixed_string_literal
;     (identifier) @gql
;     (content) @injection.content
;     (#eq? @gql "gql"))
;   (#set! injection.language "graphql"))
; (macrocall_expression
;   (macro_identifier "@" (identifier) @gql_str (#eq? @gql_str "gql_str"))
;   (macro_argument_list (string_literal (content) @injection.content))
;   (#set! injection.language "graphql"))

; HypertextLiteral.jl
; (
;   (prefixed_string_literal
;     (identifier) @htl
;     (content) @injection.content
;     (#eq? @htl "htl"))
;   (#set! injection.language "html"))
; (macrocall_expression
;   (macro_identifier "@" (identifier) @htl_str (#eq? @htl_str "htl_str"))
;   (macro_argument_list (string_literal (content) @injection.content))
;   (#set! injection.language "html"))
; (macrocall_expression
;   (macro_identifier "@" (identifier) @htl_2 (#eq? @htl_2 "htl"))
;   (macro_argument_list (string_literal (content) @injection.content))
;   (#set! injection.language "html"))

; Latexify.jl
; (
;   (prefixed_string_literal
;     (identifier) @latex
;     (content) @injection.content
;     (#eq? @latex "L"))
;   (#set! injection.language "latex"))
; (macrocall_expression
;   (macro_identifier "@" (identifier) @L_str (#eq? @L_str "L_str"))
;   (macro_argument_list (string_literal (content) @injection.content))
;   (#set! injection.language "latex"))

; PyCall.jl
; (
;   (prefixed_string_literal
;     (identifier) @py
;     (content) @injection.content
;     (#eq? @py "py"))
;   (#set! injection.language "python"))
; (macrocall_expression
;   (macro_identifier "@" (identifier) @py_str (#eq? @py_str "py_str"))
;   (macro_argument_list (string_literal (content) @injection.content))
;   (#set! injection.language "python"))

; SQLStrings.jl
; (
;   (prefixed_command_literal
;     (identifier) @sql
;     (content) @injection.content
;     (#eq? @sql "sql"))
;   (#set! injection.language "sql"))
; (macrocall_expression
;   (macro_identifier "@" (identifier) @sql_cmd (#eq? @sql_cmd "sql_cmd"))
;   (macro_argument_list (string_literal (content) @injection.content))
;   (#set! injection.language "sql"))

; Typstry.jl
; (
;   (prefixed_string_literal
;     (identifier) @typst
;     (content) @injection.content
;     (#eq? @typst "typst"))
;   (#set! injection.language "typst"))
; (macrocall_expression
;   (macro_identifier "@" (identifier) @typst_str (#eq? @typst_str "typst_str"))
;   (macro_argument_list (string_literal (content) @injection.content))
;   (#set! injection.language "typst"))
