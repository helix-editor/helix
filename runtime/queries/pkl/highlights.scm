; Copyright © 2024 Apple Inc. and the Pkl project authors. All rights reserved.
;
; Licensed under the Apache License, Version 2.0 (the "License");
; you may not use this file except in compliance with the License.
; You may obtain a copy of the License at
;
;     https://www.apache.org/licenses/LICENSE-2.0
;
; Unless required by applicable law or agreed to in writing, software
; distributed under the License is distributed on an "AS IS" BASIS,
; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
; See the License for the specific language governing permissions and
; limitations under the License.

; this definition is imprecise in that 
; * any qualified or unqualified call to a method named "Regex" is considered a regex
; * string delimiters are considered part of the regex

; Operators

[
  "??"
  "@"
  "="
  "<"
  ">"
  "!"
  "=="
  "!="
  "<="
  ">="
  "&&"
  "||"
  "+"
  "-"
  "**"
  "*"
  "/"
  "~/"
  "%"
  "|>"
] @keyword.operator

[
  "?"
  "|"
  "->"
] @operator.type

[
  ","
  ":"
  "."
  "?."
] @punctuation.delimiter

[
  "("
  ")"
  "]"
  "{"
  "}"
  ; "[" @punctuation.bracket TODO: FIGURE OUT HOW TO REFER TO CUSTOM TOKENS
] @punctuation.bracket

; Keywords

[
  "abstract"
  "amends"
  "as"
  "class"
  "extends"
  "external"
  "function"
  "hidden"
  "import"
  "import*"
  "in"
  "let"
  "local"
  "module"
  "new"
  "open"
  "out"
  "typealias"
  "when"
] @keyword

[
  "if"
  "is"
  "else"
] @keyword.control.conditional

[
  "for"
] @keyword.control.repeat

(importExpr "import" @keyword.control.import)
(importGlobExpr "import*" @keyword.control.import)

"read" @function.builtin
"read?" @function.builtin
"read*" @function.builtin
"throw" @function.builtin
"trace" @function.builtin

(moduleExpr "module" @type.builtin)
"nothing" @type.builtin
"unknown" @type.builtin

(outerExpr) @variable.builtin
"super" @variable.builtin
(thisExpr) @variable.builtin

[
  (falseLiteral)
  (nullLiteral)
  (trueLiteral)
] @constant.builtin

; Literals

(stringConstant) @string
(slStringLiteral) @string
(mlStringLiteral) @string

(escapeSequence) @constant.character.escape

(intLiteral) @constant.numeric.integer
(floatLiteral) @constant.numeric.float

(interpolationExpr
  "\\(" @punctuation.special
  ")" @punctuation.special) @embedded

(interpolationExpr
 "\\#(" @punctuation.special
 ")" @punctuation.special) @embedded

(interpolationExpr
  "\\##(" @punctuation.special
  ")" @punctuation.special) @embedded

(lineComment) @comment
(blockComment) @comment
(docComment) @comment

; Identifiers

(identifier) @variable

(classProperty (identifier) @variable.other.member)
(objectProperty (identifier) @variable.other.member)

(parameterList (typedIdentifier (identifier) @variable.parameter))
(objectBodyParameters (typedIdentifier (identifier) @variable.parameter))

; Method definitions

(classMethod (methodHeader (identifier)) @function.method)
(objectMethod (methodHeader (identifier)) @function.method)

; Method calls

(methodCallExpr
  (identifier) @function.method)

; Types

(clazz (identifier) @type)
(typeAlias (identifier) @type)
((identifier) @type
 (#match? @type "^[A-Z]"))

(typeArgumentList
  "<" @punctuation.bracket
  ">" @punctuation.bracket)
