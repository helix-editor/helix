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
(
  ((methodCallExpr (identifier) @methodName (argumentList (slStringLiteral) @injection.content))
    (#set! injection.language "regex"))
  (#eq? @methodName "Regex"))
 
((lineComment) @injection.content
 (#set! injection.language "comment"))

((blockComment) @injection.content
 (#set! injection.language "comment"))

((docComment) @injection.content
 (#set! injection.language "markdown"))
