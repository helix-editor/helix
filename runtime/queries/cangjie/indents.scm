; 缩进规则
(classDefinition
  (classBody) @indent.begin
) 

(init) @indent.begin

(functionDefinition
  (block) @indent.begin
)

(operatorFunctionDefinition
  (block) @indent.begin
)

(callSuffix) @indent.begin

(arrayLiteral) @indent.begin

(ifExpression) @indent.begin

(matchExpression
  (matchCase) @indent.begin
) @indent.begin

(forInExpression) @indent.begin

[
  "]"
  ")"
  "}"
] @indent.end @indent.branch

[
  (lineComment)
  (blockComment)
] @indent.auto
