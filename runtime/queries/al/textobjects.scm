; AL (Microsoft Dynamics 365 Business Central) textobject queries.
; =============================================================================
; Objects / types  ->  class.inside / class.around
; =============================================================================
[
  (table_declaration)
  (tableextension_declaration)
  (page_declaration)
  (pageextension_declaration)
  (pagecustomization_declaration)
  (codeunit_declaration)
  (report_declaration)
  (reportextension_declaration)
  (query_declaration)
  (xmlport_declaration)
  (enum_declaration)
  (enumextension_declaration)
  (interface_declaration)
  (controladdin_declaration)
  (dotnet_declaration)
  (permissionset_declaration)
  (permissionsetextension_declaration)
] @class.around

(table_declaration
  body: (_) @class.inside)

(tableextension_declaration
  body: (_) @class.inside)

(page_declaration
  body: (_) @class.inside)

(pageextension_declaration
  body: (_) @class.inside)

(pagecustomization_declaration
  body: (_) @class.inside)

(codeunit_declaration
  body: (_) @class.inside)

(report_declaration
  body: (_) @class.inside)

(reportextension_declaration
  body: (_) @class.inside)

(query_declaration
  body: (_) @class.inside)

(xmlport_declaration
  body: (_) @class.inside)

(enum_declaration
  body: (_) @class.inside)

(enumextension_declaration
  body: (_) @class.inside)

(interface_declaration
  body: (_) @class.inside)

(controladdin_declaration
  body: (_) @class.inside)

(dotnet_declaration
  body: (_) @class.inside)

(permissionset_declaration
  body: (_) @class.inside)

(permissionsetextension_declaration
  body: (_) @class.inside)

; =============================================================================
; Procedures / triggers  ->  function.inside / function.around
; (code blocks keep begin/end inside the body node, Rust `block` model)
; =============================================================================
(procedure) @function.around

(trigger_declaration) @function.around

(interface_procedure) @function.around

; Inside = the statement run only (excludes begin/end). The code_block now nests
; a content-only statement_block (no delimiters); target it for a clean inside.
(procedure
  body: (code_block
    body: (statement_block) @function.inside))

(trigger_declaration
  body: (code_block
    body: (statement_block) @function.inside))

; Parameters — each parameter selectable individually
(parameter) @parameter.inside

(parameter) @parameter.around

; =============================================================================
; Comments
; =============================================================================
(comment) @comment.inside

(multiline_comment) @comment.inside

[
  (comment)
  (multiline_comment)
] @comment.around
