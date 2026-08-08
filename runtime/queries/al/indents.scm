; AL (Microsoft Dynamics 365 Business Central) indent queries.
;
; One indent level per bracketed scope, per begin/end code block, and per
; control-flow body; the matching close token dedents.
; =============================================================================
; Nodes that open an indented scope
; =============================================================================
; Object declarations (all use { ... })
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
  (profile_declaration)
  (profileextension_declaration)
  (permissionset_declaration)
  (permissionsetextension_declaration)
  (entitlement_declaration)
] @indent

; Code blocks (begin ... end)
(code_block) @indent

; Sections that use { ... }
[
  (fields_section)
  (keys_section)
  (fieldgroups_section)
  (layout_section)
  (actions_section)
  (dataset_section)
  (elements_section)
  (labels_section)
  (rendering_section)
  (requestpage_section)
  (views_section)
  (schema_section)
] @indent

; Layout elements
[
  (area_section)
  (group_section)
  (repeater_section)
  (cuegroup_section)
  (fixed_section)
  (grid_section)
  (part_section)
  (systempart_section)
  (usercontrol_section)
] @indent

; Action sections
[
  (action_area_section)
  (action_group_section)
  (action_declaration)
  (customaction_declaration)
  (systemaction_declaration)
  (fileuploadaction_declaration)
  (actionref_declaration)
] @indent

; Report sections
[
  (report_dataitem)
  (report_column)
  (rendering_layout)
  (label_section)
] @indent

; Query sections
[
  (query_dataitem)
  (query_column)
  (query_filter)
] @indent

; XMLport elements
[
  (xmlport_element)
  (xmlport_attribute)
] @indent

; Declarations with bodies
[
  (field_declaration)
  (enum_value_declaration)
  (key_declaration)
  (view_definition)
  (assembly_declaration)
] @indent

; Var sections
(var_section) @indent

; Control flow (bodies are indented)
[
  (case_branch)
  (case_else_branch)
] @indent

; case ... of ... end; the branches are indented, `end` outdents.
[
  (case_statement)
  (case_branch)
  (case_else_branch)
] @indent

; Modification blocks
[
  (addafter_modification)
  (addbefore_modification)
  (addfirst_modification)
  (addlast_modification)
  (modify_modification)
  (moveafter_modification)
  (movebefore_modification)
  (movefirst_modification)
  (movelast_modification)
  (addafter_action_modification)
  (addbefore_action_modification)
  (addfirst_action_modification)
  (addlast_action_modification)
  (modify_action_modification)
  (addafter_dataset_modification)
  (addbefore_dataset_modification)
  (addfirst_dataset_modification)
  (addlast_dataset_modification)
  (add_dataset_modification)
  (addfirst_fieldgroup_modification)
  (addlast_fieldgroup_modification)
  (addafter_views_modification)
  (addbefore_views_modification)
  (addfirst_views_modification)
  (addlast_views_modification)
] @indent

; Preprocessor conditionals
[
  (preproc_conditional)
  (preproc_conditional_actions)
  (preproc_conditional_case)
  (preproc_conditional_controladdin)
  (preproc_conditional_dataset)
  (preproc_conditional_fieldgroups)
  (preproc_conditional_fields)
  (preproc_conditional_keys)
  (preproc_conditional_layout)
  (preproc_conditional_object)
  (preproc_conditional_query)
  (preproc_conditional_report)
  (preproc_conditional_statement)
  (preproc_conditional_var)
  (preproc_conditional_var_block)
  (preproc_conditional_xmlport)
] @indent

; Argument and parameter lists
(argument_list) @indent

(parameter_list) @indent

; =============================================================================
; Tokens that close it
; =============================================================================
; AL is written Allman style: the braces live inside the declaration node,
; so the declaration scope already covers the line the brace sits on.
"{" @outdent

"}" @outdent

")" @outdent

"]" @outdent

(end_keyword) @outdent

; repeat ... until brackets its body, so it scopes like begin/end.
(repeat_statement) @indent

(repeat_statement
  (until_keyword) @outdent)

; Brace-less bodies: a single statement on the line after then/else/do.
; When the body is a begin/end block the code_block capture already opens the
; scope, so these must not fire as well.
(if_statement
  then_branch: (_) @indent
  (#not-kind-eq? @indent "code_block")
  (#set! "scope" "header"))

(if_statement
  else_branch: (_) @indent
  (#not-kind-eq? @indent "code_block")
  (#not-kind-eq? @indent "if_statement")
  (#set! "scope" "header"))

(while_statement
  body: (_) @indent
  (#not-kind-eq? @indent "code_block")
  (#set! "scope" "header"))

(for_statement
  body: (_) @indent
  (#not-kind-eq? @indent "code_block")
  (#set! "scope" "header"))

(foreach_statement
  body: (_) @indent
  (#not-kind-eq? @indent "code_block")
  (#set! "scope" "header"))

(with_statement
  body: (_) @indent
  (#not-kind-eq? @indent "code_block")
  (#set! "scope" "header"))
