; tags.scm

(
  (comment)* @doc
  .
  [
    (function_definition
      [(function_rule name: (_) @name) @definition.function
       (function_fact name: (_) @name) @definition.function])
    (predicate_definition
      [(predicate_rule name: (_) @name) @definition.function
       (predicate_fact name: (_) @name) @definition.function])
    (actor_definition
      [(action_rule name: (_) @name) @definition.function
       (nonbacktrackable_predicate_rule name: (_) @name) @definition.function])
  ]
  (#strip! @doc "^%\\s*")
  (#select-adjacent! @doc @definition.function)
)

(function_call function: (_) @name @reference.call)

(dot_expression right: (_) @reference.call)

(import_declaration (_) @name) @reference.module

(module_declaration (_) @name) @definition.module
