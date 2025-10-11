; 缩进规则
(classDefinition
  (classBody) @indent.begin
) 

(init) @indent.begin

(functionDefinition
  (block) @indent.begin
    (#set! indent.open_delimiter "{")
    (#set! indent.close_delimiter "}")
;    (#set! indent.immediate true)  ; 立即应用缩进变化
)

(operatorFunctionDefinition
  (block) @indent.begin
)

(callSuffix) @indent.begin

;(callSuffix  
;  "(" @indent.begin
;  ")" @indent.end) @indent.branch

(arrayLiteral) @indent.begin
  (#set! indent.open_delimiter "[")
  (#set! indent.close_delimiter "]")

(ifExpression) @indent.begin
;(ifExpression
;  consequence: "}" @indent.branch
;  alternative: "}" @indent.end
;)

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
