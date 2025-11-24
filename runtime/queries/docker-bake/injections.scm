((comment) @injection.content
  (#set! injection.language "comment"))

; https://docs.docker.com/build/bake/reference/#targetdockerfile-inline
(block
  (identifier) @_target (#eq? @_target "target")
  (body
    (attribute
      (identifier) @_attr (#eq? @_attr "dockerfile-inline")
      (expression
        (template_expr
          (heredoc_template
            (template_literal) @injection.content)))))
  (#set! injection.language "dockerfile"))

(function_call
  (identifier) @_name (#eq? @_name "regex")
  (function_arguments
    (expression
      (literal_value
        (string_lit
          (template_literal) @injection.content))))
  (#set! injection.language "regex"))
