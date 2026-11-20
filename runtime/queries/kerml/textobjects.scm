; Classes: namespaces and type definitions
(package
  body: (_) @class.inside) @class.around
(library_package
  body: (_) @class.inside) @class.around
(namespace
  body: (_) @class.inside) @class.around
(type
  body: (_) @class.inside) @class.around
(classifier
  body: (_) @class.inside) @class.around
(data_type
  body: (_) @class.inside) @class.around
(class
  body: (_) @class.inside) @class.around
(structure
  body: (_) @class.inside) @class.around
(association
  body: (_) @class.inside) @class.around
(association_structure
  body: (_) @class.inside) @class.around
(metaclass
  body: (_) @class.inside) @class.around
(interaction
  body: (_) @class.inside) @class.around

; Functions: behaviours, constraints and states
(behavior
  body: (_) @function.inside) @function.around
(function
  body: (_) @function.inside) @function.around
(predicate
  body: (_) @function.inside) @function.around
(step
  body: (_) @function.inside) @function.around
(expression_feature
  body: (_) @function.inside) @function.around
(boolean_expression
  body: (_) @function.inside) @function.around
(invariant
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
