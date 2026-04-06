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

(
  (prefixed_string_literal
    (identifier) @regex
    (content) @injection.content
    (#eq? @regex "r"))
  (#set! injection.language "regex"))

((command_literal (content) @injection.content (#set! injection.language "bash")))

; Latexify.jl
; (
;   (prefixed_string_literal
;     (identifier) @latex
;     (content) @injection.content
;     (#eq? @latex "L"))
;   (#set! injection.language "latex"))

; Typstry.jl
; (
;   (prefixed_string_literal
;     (identifier) @typst
;     (content) @injection.content
;     (#eq? @typst "typst"))
;   (#set! injection.language "typst"))
