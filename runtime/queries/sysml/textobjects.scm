; Classes: namespaces and type definitions
(package
  body: (_) @class.inside) @class.around
(library_package
  body: (_) @class.inside) @class.around
(namespace
  body: (_) @class.inside) @class.around
(part_definition
  body: (_) @class.inside) @class.around
(item_definition
  body: (_) @class.inside) @class.around
(attribute_definition
  body: (_) @class.inside) @class.around
(enumeration_definition
  body: (_) @class.inside) @class.around
(occurrence_definition
  body: (_) @class.inside) @class.around
(port_definition
  body: (_) @class.inside) @class.around
(connection_definition
  body: (_) @class.inside) @class.around
(interface_definition
  body: (_) @class.inside) @class.around
(allocation_definition
  body: (_) @class.inside) @class.around
(flow_definition
  body: (_) @class.inside) @class.around
(metadata_definition
  body: (_) @class.inside) @class.around
(extended_definition
  body: (_) @class.inside) @class.around
(state_definition
  body: (_) @class.inside) @class.around
(constraint_definition
  body: (_) @class.inside) @class.around
(requirement_definition
  body: (_) @class.inside) @class.around
(concern_definition
  body: (_) @class.inside) @class.around
(case_definition
  body: (_) @class.inside) @class.around
(analysis_case_definition
  body: (_) @class.inside) @class.around
(verification_case_definition
  body: (_) @class.inside) @class.around
(use_case_definition
  body: (_) @class.inside) @class.around
(view_definition
  body: (_) @class.inside) @class.around
(viewpoint_definition
  body: (_) @class.inside) @class.around
(rendering_definition
  body: (_) @class.inside) @class.around

; Functions: behaviours, constraints and states
(action_definition
  body: (_) @function.inside) @function.around
(calculation_definition
  body: (_) @function.inside) @function.around
(action_usage
  body: (_) @function.inside) @function.around
(calculation_usage
  body: (_) @function.inside) @function.around
(perform_action_usage
  body: (_) @function.inside) @function.around
(constraint_usage
  body: (_) @function.inside) @function.around
(assert_constraint_usage
  body: (_) @function.inside) @function.around
(requirement_constraint_usage
  body: (_) @function.inside) @function.around
(state_usage
  body: (_) @function.inside) @function.around
(exhibit_state_usage
  body: (_) @function.inside) @function.around
(transition_usage
  body: (_) @function.inside) @function.around

; Parameters and arguments

(argument_list
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(sequence_expression
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

; Comments

[
  (line_note)
  (block_note)
  (regular_comment)
] @comment.inside

(line_note)+ @comment.around

(block_note) @comment.around

(documentation) @comment.around

(comment) @comment.around
