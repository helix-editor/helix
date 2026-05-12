(class_definition
  body: (_) @class.inside) @class.around

(mixin_declaration
  (class_body) @class.inside) @class.around

(extension_declaration
  (extension_body) @class.inside) @class.around

(enum_declaration
  body: (_) @class.inside) @class.around

(type_alias) @class.around

(_
  (
    [
      (getter_signature)
      (setter_signature)
      (function_signature)
      (method_signature)
      (constructor_signature)
    ]
    .
    (function_body) @function.inside @function.around
  )  @function.around
)

(declaration
  [
    (constant_constructor_signature)
    (constructor_signature)
    (factory_constructor_signature)
    (redirecting_factory_constructor_signature)
    (getter_signature)
    (setter_signature)
    (operator_signature)
    (function_signature)
  ]
) @function.around

(lambda_expression
    body: (_) @function.inside
) @function.around

(function_expression
    body: (_) @function.inside
) @function.around

[
  (comment)
  (documentation_comment)
] @comment.inside

(comment)+ @comment.around

(documentation_comment)+ @comment.around

(formal_parameter_list
  (
    (formal_parameter) @parameter.inside . ","? @parameter.around
  ) @parameter.around
)

(optional_formal_parameters
  (
    (formal_parameter) @parameter.inside . ","? @parameter.around
  ) @parameter.around
)

(arguments
  (
    [
      (argument) @parameter.inside
      (named_argument (label) . (_)* @parameter.inside)
    ]
    . ","? @parameter.around
  ) @parameter.around
)

(type_arguments
  (
    ((_) . ("." . (_) @parameter.inside @parameter.around)?) @parameter.inside
    . ","? @parameter.around
  ) @parameter.around
)

(expression_statement
  ((identifier) @_name (#any-of? @_name "test" "testWidgets"))
  .
  (selector (argument_part (arguments . (_) . (argument) @test.inside)))
) @test.around

