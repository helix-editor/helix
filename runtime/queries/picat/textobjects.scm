[
  (function_definition
    [(function_rule) (function_fact)] @function.inside)
  (predicate_definition
    [(predicate_rule) (predicate_fact)] @function.inside)
  (actor_definition
    [(action_rule) (nonbacktrackable_predicate_rule)] @function.inside)
] @function.around

(import_declaration
  (atom) @function.around)

(parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.inside
(comment)+ @comment.around

(array_expression
  (_) @entry.around)

(list_expression
  (_) @entry.around)
