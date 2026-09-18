(function_definition
  body: (_) @function.inside) @function.around

(law_declaration
  (law_body) @function.inside) @function.around

(type_declaration) @class.around

(type_declaration
  (constructors) @class.inside)

; Parameters and arguments include a trailing comma in the `around` object
; when one is present, which makes deletion behave cleanly.
(parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(type_parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(lambda_expression
  parameter: (_) @parameter.inside) @parameter.around

(comment) @comment.inside
(comment)+ @comment.around

(case_clause) @entry.around
(constructor_declaration) @entry.around
(quantifier) @entry.around
