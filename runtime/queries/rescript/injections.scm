((comment) @injection.content (#set! injection.language "comment"))

; %re
(extension_expression
  (extension_identifier) @_name
  (#eq? @_name "re")
  (expression_statement (_) @injection.content (#set! injection.language "regex")))

; %raw
(extension_expression
  (extension_identifier) @_name
  (#eq? @_name "raw")
  (expression_statement
    (_ (_)  @injection.content (#set! injection.language "javascript"))))

; %graphql
(extension_expression
  (extension_identifier) @_name
  (#eq? @_name "graphql")
  (expression_statement
    (_ (_) @injection.content (#set! injection.language "graphql"))))

; %relay
(extension_expression
  (extension_identifier) @_name
  (#eq? @_name "relay")
  (expression_statement
    (_ (_) @injection.content (#set! injection.language "graphql") )))

