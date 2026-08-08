; AL (Microsoft Dynamics 365 Business Central) local scope queries.
; =============================================================================
; Scope Definitions
; =============================================================================
; Object declarations create scopes
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
  (profile_declaration)
  (profileextension_declaration)
  (permissionset_declaration)
  (permissionsetextension_declaration)
  (entitlement_declaration)
  (dotnet_declaration)
] @local.scope

; Procedures and triggers create scopes
(procedure) @local.scope

(trigger_declaration) @local.scope

(event_declaration) @local.scope

(interface_procedure) @local.scope

(preproc_split_procedure) @local.scope

; Code blocks create scopes
(code_block) @local.scope

; Control flow statements with bodies create scopes
(if_statement) @local.scope

(case_statement) @local.scope

(case_branch) @local.scope

(for_statement) @local.scope

(foreach_statement) @local.scope

(while_statement) @local.scope

(repeat_statement) @local.scope

(with_statement) @local.scope

; =============================================================================
; Local Definitions
; =============================================================================
; Variable declarations define local variables
(variable_declaration
  name: (identifier) @local.definition.variable)

(variable_declaration
  name: (quoted_identifier) @local.definition.variable)

; Parameters define local variables
(parameter
  name: (identifier) @local.definition.variable.parameter)

(parameter
  name: (quoted_identifier) @local.definition.variable.parameter)

; Return values define local variables
(procedure
  return_value: (identifier) @local.definition.variable)

(procedure
  return_value: (quoted_identifier) @local.definition.variable)

(trigger_declaration
  return_value: (identifier) @local.definition.variable)

(trigger_declaration
  return_value: (quoted_identifier) @local.definition.variable)

; Label declarations
(label_declaration
  name: (identifier) @local.definition.variable)

; Object members are not locals. Captured without a definition suffix so
; they act as discards, stopping them resolving as variable references.
(field_declaration
  name: [
    (identifier)
    (quoted_identifier)
  ] @local.member)

(enum_value_declaration
  value_name: [
    (identifier)
    (quoted_identifier)
  ] @local.member)

(key_declaration
  name: [
    (identifier)
    (quoted_identifier)
  ] @local.member)

; For loop variables
(for_statement
  variable: (identifier) @local.definition.variable)

; Foreach loop variables
(foreach_statement
  variable: (identifier) @local.definition.variable)

; =============================================================================
; References
; =============================================================================
; Identifiers are references
(identifier) @local.reference

; Quoted identifiers are references
(quoted_identifier) @local.reference
