; == StrictDoc Code Navigation Queries ==

; -- Document Configuration Definitions --
(
(document_config
uid: (uid_string) @name) @definition.document.uid
)
(
(document_version
version: (single_line_string) @name) @definition.document.version
)
(
(document_date
date: (date) @name) @definition.document.date
)
(
(document_classification
classification: (single_line_string) @name) @definition.document.classification
)
(
(document_requirement
requirement_prefix: (single_line_string) @name) @definition.document.requirement_prefix
)
(
(document_config
root: (boolean_choice) @name) @definition.document.root
)
(
(document_config
options: (document_config_options) @name) @definition.document.options
)
(
(enable_mid
(boolean_choice) @name) @definition.document.options.enable_mid
)
(
(markup
(markup_choice) @name) @definition.document.options.markup
)
(
(auto_levels
(auto_levels_choice) @name) @definition.document.options.auto_levels
)
(
(layout
(layout_choice) @name) @definition.document.options.layout
)
(
(view_style
(style_choice) @name) @definition.document.options.view_style
)
(
(in_toc_tag
(boolean_choice) @name) @definition.document.options.in_toc_tag
)
(
(default_view
(single_line_string) @name) @definition.document.options.default_view
)
(
(document_config
metadata: (document_custom_metadata) @name) @definition.document.metadata
)

; -- Grammar Definition -- ( (document_grammar) @definition.grammar.document_grammar )

; -- Import From File Reference --
(
(import_from_file
import_path: (file_path) @name) @reference.grammar.file_path
)

; -- Grammar Elements Definitions --
(
(grammar_elements) @definition.grammar.elements
)
(
(grammar_element) @definition.grammar.element
)

; -- Grammar Properties Definitions --
(
(grammar_properties) @definition.grammar.properties
)
(
(grammar_property_is_composite
(boolean_choice) @name) @definition.grammar.property.is_composite
)
(
(grammar_property_prefix
(single_line_string) @name) @definition.grammar.property.prefix
)
(
(grammar_property_view_style
(style_choice) @name) @definition.grammar.property.view_style
)

; -- Grammar Fields Definitions --
(
(grammar_fields) @definition.grammar.fields
)
(
(grammar_field_title
title: (field_name) @name) @definition.grammar.field.title
)
(
(grammar_field_required
value: (boolean_choice) @name) @definition.grammar.field.required
)
(
(grammar_field_string) @definition.grammar.field.string
)
(
(grammar_field_single_choice) @definition.grammar.field.single_choice
)
(
(grammar_field_multiple_choice) @definition.grammar.field.multiple_choice
)
(
(grammar_field_tag) @definition.grammar.field.tag
)

; -- Grammar Relations Definitions --
(
(grammar_relations) @definition.grammar.relations
)
(
(grammar_relation_parent
(single_line_string) @name) @definition.grammar.relation.parent
)
(
(grammar_relation_child
(single_line_string) @name) @definition.grammar.relation.child
)
(
(grammar_relation_file
(single_line_string) @name) @definition.grammar.relation.file
)

; -- Document Custom Metadata --
(
  (document_custom_metadata) @definition.document.metadata
)
(
  (document_custom_metadata_key_value_pair
    key: (document_custom_metadata_key) @name
    value: (single_line_string) @doc) @definition.document.metadata.entry
)

; -- Document View Definitions --
(
(document_view) @definition.document.view
)
(
(view_element
id: (uid_string) @name) @definition.view.element.id
)
(
(view_element
name: (single_line_string) @name) @definition.view.element.name
)
(
(view_element_tag
object_type: (single_line_string) @name) @definition.view.element.tag.object_type
)
(
(view_element_field
name: (single_line_string) @name) @definition.view.element.field.name
)
(
(view_element_field
placement: (single_line_string) @name) @definition.view.element.field.placement
)
(
(view_element_hidden_tag
hidden_tag: (single_line_string) @name) @definition.view.element.hidden_tag
)

; -- Section & Requirement Definitions --

(
(section_or_requirement_list) @definition.section.list
)
(
  (sdoc_section) @definition.section
)

(
  (section_body
    mid: (single_line_string) @name) @definition.section.mid
)

(
(section_body
  uid: (uid_string) @name) @definition.section.uid
)

(
  (section_body
    custom_level: (single_line_string) @name) @definition.section.level
)

(
  (section_body
    title: (single_line_string) @name) @definition.section.title
)

(
  (section_body
    requirement_prefix: (single_line_string) @name) @definition.section.requirement_prefix
)
; -- Document From File Reference --
(
(document_from_file
path: (file_path) @name) @definition.document.from_file
)

; -- SDoc Node Definitions --
(
(sdoc_node) @definition.node
)
(
(sdoc_composite_node) @definition.composite_node
)
(
(sdoc_composite_node_opening
node_type_opening: (sdoc_composite_node_type_name) @name) @definition.composite_node.opening
)
(
(sdoc_composite_node_type_name) @definition.composite_node.type
)

; -- SDoc Node Field Definitions --
(
  (sdoc_node_field_mid
    mid: (single_line_string) @name) @definition.node.mid
)

(
  (sdoc_node_field_uid
    uid: (uid_string) @name) @definition.node.uid
)

(
  (sdoc_node_field_generic
    field_name: (field_name) @name
  ) @definition.node.field
)

(
  (parent_req_reference
    ref_uid: (req_reference_value_id) @name) @reference.node.uid
)

(
  (child_req_reference
    ref_uid: (req_reference_value_id) @name) @reference.node.uid
)
