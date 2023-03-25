;; Constants, Comments, and Literals

(comment) @comment.line
(multilineComment) @comment.block
(docComment) @comment.block.documentation
(multilineDocComment) @comment.block.documentation
; comments

[(literal) (generalizedLit)] @constant
[(nil_lit)] @constant.builtin
[(bool_lit)] @constant.builtin.boolean
[(char_lit)] @constant.character
[(char_esc_seq) (str_esc_seq)] @constant.character.escape
[(custom_numeric_lit)] @constant.numeric
[(int_lit) (int_suffix)] @constant.numeric.integer
[(float_lit) (float_suffix)] @constant.numeric.float
; literals
; note: somewhat irritatingly for testing, lits have the same syntax highlighting as types

[
  (str_lit)
  (triplestr_lit)
  (rstr_lit)
  (generalized_str_lit)
  (generalized_triplestr_lit)
  (interpolated_str_lit)
  (interpolated_triplestr_lit)
] @string
; [] @string.regexp
; string literals

[
  "."
  ","
  ";"
  ":"
] @punctuation.delimiter
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "{."
  ".}"
  "#["
  "]#"
] @punctuation.bracket
(interpolated_str_lit "&" @punctuation.special)
(interpolated_str_lit "{" @punctuation.special)
(interpolated_str_lit "}" @punctuation.special)
; punctuation

[
  "and"
  "or"
  "xor"
  "not"
  "in"
  "notin"
  "is"
  "isnot"
  "div"
  "mod"
  "shl"
  "shr"
] @keyword.operator
; operators: we list them explicitly to deliminate them from symbolic operators

[(operator) (opr) "="] @operator
; all operators (must come after @keyword.operator)

(pragma) @attribute
; pragmas


;; Imports and Exports

(importStmt
  (keyw) @keyword.control.import
  (expr (primary (symbol) @namespace))?
  (expr (primary (arrayConstr (exprColonExprList (exprColonExpr (expr (primary (symbol) @namespace)))))))?)
(exportStmt
  (keyw) @keyword.control.import
  (expr (primary (symbol) @namespace))?
  (expr (primary (arrayConstr (exprColonExprList (exprColonExpr (expr (primary (symbol) @namespace)))))))?)
(fromStmt
  (keyw) @keyword.control.import
  (expr (primary (symbol) @namespace))?
  (expr (primary (arrayConstr (exprColonExprList (exprColonExpr (expr (primary (symbol) @namespace)))))))?)
(includeStmt
  (keyw) @keyword.control.import
  (expr (primary (symbol) @namespace))?
  (expr (primary (arrayConstr (exprColonExprList (exprColonExpr (expr (primary (symbol) @namespace)))))))?)
(importExceptStmt
  (keyw) @keyword.control.import
  (expr (primary (symbol) @namespace))?
  (expr (primary (arrayConstr (exprColonExprList (exprColonExpr (expr (primary (symbol) @namespace)))))))?)
; import statements
; yeah, this is a bit gross.


;; Control Flow

(ifStmt (keyw) @keyword.control.conditional)
(whenStmt (keyw) @keyword.control.conditional)
(elifStmt (keyw) @keyword.control.conditional)
(elseStmt (keyw) @keyword.control.conditional)
(caseStmt (keyw) @keyword.control.conditional)
(ofBranch (keyw) @keyword.control.conditional)
(inlineIfStmt (keyw) @keyword.control.conditional)
(inlineWhenStmt (keyw) @keyword.control.conditional)
; conditional statements
; todo: do block

(forStmt
  . (keyw) @keyword.control.repeat
  . (symbol) @variable
  . (keyw) @keyword.control.repeat)
(whileStmt (keyw) @keyword.control.repeat)
; loop statements

(returnStmt (keyw) @keyword.control.repeat)
(yieldStmt (keyw) @keyword.control.repeat)
(discardStmt (keyw) @keyword.control.repeat)
(breakStmt (keyw) @keyword.control.repeat)
(continueStmt (keyw) @keyword.control.repeat)
; control flow statements

(raiseStmt (keyw) @keyword.control.exception)
(tryStmt (keyw) @keyword.control.exception)
(tryExceptStmt (keyw) @keyword.control.exception)
(tryFinallyStmt (keyw) @keyword.control.exception)
(inlineTryStmt (keyw) @keyword.control.exception)
; (inlineTryExceptStmt (keyw) @keyword.control.exception)
; (inlineTryFinallyStmt (keyw) @keyword.control.exception)
; exception handling statements

(staticStmt (keyw) @keyword)
(deferStmt (keyw) @keyword)
(asmStmt (keyw) @keyword)
(bindStmt (keyw) @keyword)
(mixinStmt (keyw) @keyword)
; miscellaneous blocks

(blockStmt
  (keyw) @keyword.control
  (symbol) @label)
; block statements


;; Types and Type Declarations

(typeDef
  (keyw) @keyword.storage.type
  (symbol) @type)
; names of new types type declarations

(exprColonEqExpr
  . (expr (primary (symbol) @variable))
  . (expr (primary (symbol) @type)))
; variables in inline tuple declarations

(primarySuffix
  (indexSuffix
    (exprColonEqExprList
      (exprColonEqExpr
        (expr
          (primary
            (symbol) @type))))))
; nested types in brackets, i.e. seq[string]

(primaryTypeDef (symbol) @type)
; primary types of type declarations (NOT nested types)

(primaryTypeDef (primaryPrefix (keyw) @type))
; for consistency

(primaryTypeDesc (symbol) @type)
; type annotations, on declarations or in objects

(primaryTypeDesc (primaryPrefix (keyw) @type))
; var types etc

(genericParamList (genericParam (symbol) @type))
; types in generic blocks

(enumDecl (keyw) @keyword.storage.type)
(enumElement (symbol) @type.enum.variant)
; enum declarations and elements

(tupleDecl (keyw) @keyword.storage.type)
; tuple declarations

(objectDecl (keyw) @keyword.storage.type)
(objectPart (symbol) @variable.other.member)
; object declarations and fields

(objectCase
  (keyw) @keyword.control.conditional
  (symbol) @variable.other.member)
(objectBranch (keyw) @keyword.control.conditional)
(objectElif (keyw) @keyword.control.conditional)
(objectElse (keyw) @keyword.control.conditional)
(objectWhen (keyw) @keyword.control.conditional)
; variant objects

(conceptDecl (keyw) @keyword.storage.type)
(conceptParam (keyw) @type)
(conceptParam (symbol) @variable)
; concept declarations, parameters, and qualifiers on those parameters

((expr
  (primary (symbol))
  (operator) @operator
  (primary (symbol) @type))
 (#match? @operator "is"))
((exprStmt
  (primary (symbol))
  (operator) @operator
  (primary (symbol) @type))
 (#match? @operator "is"))
; symbols likely to be types: "x is t" means t is either a type or a type variable

; distinct?


;; Functions

(routine
  . (keyw) @keyword.function
  . (symbol) @function)
; function declarations

(routineExpr (keyw) @keyword.function)
; discarded function

(routineExprTypeDesc (keyw) @keyword.function)
; function declarations as types

(primary
  . (symbol) @function.call
  . (primarySuffix (functionCall)))
; regular function calls

(primary
  . (symbol) @function.call
  . (primarySuffix (cmdCall)))
; function calls without parenthesis

(primary
  (primarySuffix (qualifiedSuffix (symbol) @function.call))
  . (primarySuffix (functionCall)))
; uniform function call syntax calls

(primary
  (primarySuffix (qualifiedSuffix (symbol) @function.call))
  . (primarySuffix (cmdCall)))
; just in case

(primary
  (symbol) @constructor
  (primarySuffix (objectConstr)))
; object constructor

; does not appear to be a way to distinguish these without verbatium matching
; [] @function.builtin
; [] @function.method
; [] @function.macro
; [] @function.special


;; Variables

(paramList (paramColonEquals (symbol) @variable.parameter))
; parameter identifiers

(identColon (ident) @variable.other.member)
; named parts of tuples

(symbolColonExpr (symbol) @variable)
; object constructor parameters

(symbolEqExpr (symbol) @variable)
; named parameters

(variable
  (keyw) @keyword.storage.type
  (declColonEquals (symbol) @variable))
; let, var, const expressions

((primary (symbol) @variable.builtin)
 (#match? @variable.builtin "result"))
; `result` is an implicit builtin variable inside function scopes

((primary (symbol) @type)
 (#match? @type "^[A-Z]"))
; assume PascalCase identifiers to be types

((primary
  (primarySuffix
    (qualifiedSuffix
      (symbol) @type)))
 (#match? @type "^[A-Z]"))
; assume PascalCase member variables to be enum entries

(primary (symbol) @variable)
; overzealous, matches variables

(primary (primarySuffix (qualifiedSuffix (symbol) @variable.other.member)))
; overzealous, matches member variables: i.e. x in foo.x

(keyw) @keyword
; more specific matches are done above whenever possible
