(type_definition) @class.around

(executable_definition) @function.around

(arguments_definition
  (input_value_definition) @parameter.inside @parameter.movement)

(arguments
  (argument) @parameter.inside @parameter.movement)

(selection
  [(field) (fragment_spread)] @entry.around)

(selection
  (field (selection_set) @entry.inside))

(field_definition
  (_) @entry.inside) @entry.around

(input_fields_definition
  (input_value_definition ) @entry.around)

(enum_value) @entry.around
