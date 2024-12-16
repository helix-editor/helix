; Copyright (C) 2021 Will Cassella (github@willcassella.com)
; 
; Licensed under the Apache License, Version 2.0 (the "License");
; you may not use this file except in compliance with the License.
; You may obtain a copy of the License at
; 
;         http://www.apache.org/licenses/LICENSE-2.0
; 
; Unless required by applicable law or agreed to in writing, software
; distributed under the License is distributed on an "AS IS" BASIS,
; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
; See the License for the specific language governing permissions and
; limitations under the License.

(identifier) @variable.builtin

(scope_access field: (_) @variable.other.member)

(call target: (_) @function)

[ "if" "else" ] @keyword.control.conditional

[
    (assign_op)
    (arithmetic_binary_op)
    (comparison_binary_op)
    (equivalence_binary_op)
    (logical_and_binary_op)
    (logical_or_binary_op)
    (negation_unary_op)
] @operator

[ "(" ")" "[" "]" "{" "}" ] @punctuation.bracket
[ "." "," ] @punctuation.delimiter

(string) @string
(string_escape) @constant.character.escape
(string_expansion [ "$" "${" "}" ] @constant.character.escape)
[ (integer) (hex) ] @constant.numeric
(boolean) @constant.builtin.boolean

(comment) @comment
