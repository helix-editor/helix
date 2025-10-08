; Classes and similar constructs
[
  (classDefinition (classBody) @class.inside)
  (structDefinition (structBody) @class.inside)
  (interfaceDefinition (interfaceBody) @class.inside)
  (enumDefinition (enumBody) @class.inside)
  (extendDefinition (extendBody) @class.inside)
] @class.around

; Functions
[
  (functionDefinition (block) @function.inside)
  (operatorFunctionDefinition (block) @function.inside)
  (mainDefinition (block) @function.inside)
  (propertyDefinition) ; TODO
  (macroDefinition (block) @class.inside)
] @function.around

; Comments
[
  (lineComment)
  (blockComment)
] @comment.inside

[
  (lineComment)+
  (blockComment)
] @comment.around

; Parameters
[
  (parameter)
  (namedParameter)
] @parameter.inside

[
  (typeParameters (identifier) @parameter.inside)
  (lambdaParameters (lambdaParameter) @parameter.inside)
  (parameterList)
  (primaryInitParamList)
] @parameter.around
