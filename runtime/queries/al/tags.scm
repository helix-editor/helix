; AL (Microsoft Dynamics 365 Business Central) tag queries.
; =============================================================================
; Object Definitions
; =============================================================================
(table_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(page_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(codeunit_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(report_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(query_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(xmlport_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(enum_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(interface_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.interface

(controladdin_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

; =============================================================================
; Extension Definitions
; =============================================================================
(tableextension_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(pageextension_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(enumextension_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(reportextension_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(profileextension_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(permissionsetextension_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

; =============================================================================
; Other Object Types
; =============================================================================
(profile_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(permissionset_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(entitlement_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

(pagecustomization_declaration
  object_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.class

; =============================================================================
; Procedure and Trigger Definitions
; =============================================================================
; Procedure definitions
(procedure
  name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.function

; Event declarations
(event_declaration
  name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.function

; Trigger declarations
(trigger_declaration
  name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.function

; Scoped member-trigger name (`Object::Member`) — tag on the member
(trigger_declaration
  name: (member_trigger_name
    member: [
      (identifier)
      (quoted_identifier)
    ] @name)) @definition.function

; Interface procedures
(interface_procedure
  name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.function

; Split procedures (preprocessor-split)
(preproc_split_procedure
  name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.function

; =============================================================================
; Test Definitions (for test runners like neotest)
; =============================================================================
; [Test] as first attribute, 0+ trailing attributes before procedure
((attribute_item
  (attribute_content
    name: (identifier) @_test_attr
    (#match? @_test_attr "^[Tt][Ee][Ss][Tt]$")))
  (attribute_item)*
  .
  (procedure
    name: [
      (identifier)
      (quoted_identifier)
    ] @test.name) @test.definition)

; [Test] after 1+ other attributes (e.g. [Obsolete][Test][Scope] procedure)
((attribute_item)
  .
  (attribute_item
    (attribute_content
      name: (identifier) @_test_attr
      (#match? @_test_attr "^[Tt][Ee][Ss][Tt]$")))
  (attribute_item)*
  .
  (procedure
    name: [
      (identifier)
      (quoted_identifier)
    ] @test.name) @test.definition)

; =============================================================================
; Field Definitions
; =============================================================================
; Table field definitions
(field_declaration
  name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.field

; Enum value definitions
(enum_value_declaration
  value_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.constant

; Key definitions
(key_declaration
  name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.field

; Fieldgroup definitions
(fieldgroup_declaration
  name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.field

; Label declarations
(label_declaration
  name: (identifier) @name) @definition.constant

; =============================================================================
; Action Definitions
; =============================================================================
(action_declaration
  name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.function

(customaction_declaration
  name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.function

(systemaction_declaration
  name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.function

(fileuploadaction_declaration
  name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.function

(actionref_declaration
  action_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.function

; =============================================================================
; Module/Namespace Definitions
; =============================================================================
(namespace_declaration
  name: (namespace_name) @name) @definition.module

; =============================================================================
; View Definitions
; =============================================================================
(view_definition
  name: [
    (identifier)
    (quoted_identifier)
  ] @name) @definition.field

; =============================================================================
; Function/Method References
; =============================================================================
; Direct function calls
(call_expression
  function: (identifier) @name) @reference.call

; Method calls on objects
(call_expression
  function: (member_expression
    member: (identifier) @name)) @reference.call

; =============================================================================
; Type References
; =============================================================================
; Record type references
(record_type
  reference: [
    (identifier)
    (quoted_identifier)
  ] @name) @reference.type

; Object reference type references
(object_reference_type
  reference: [
    (identifier)
    (quoted_identifier)
    (integer)
  ] @name) @reference.type

; Database references
(database_reference
  table_name: [
    (identifier)
    (quoted_identifier)
  ] @name) @reference.class

; =============================================================================
; Interface References
; =============================================================================
(implements_clause
  interface: [
    (identifier)
    (quoted_identifier)
  ] @name) @reference.implementation

; =============================================================================
; Enum Value References
; =============================================================================
(qualified_enum_value
  enum_type: [
    (identifier)
    (quoted_identifier)
  ] @name) @reference.class

(qualified_enum_value
  value: [
    (identifier)
    (quoted_identifier)
  ] @name) @reference.constant
