["class"  "feature" "end" "do" "alias" "convert"
 "invariant" "across" "as" "loop" "check"
 "if" "attached" "then" "else" "elseif"
 "inspect" "when" "then"
 "note" "local" "create" "require" "ensure"
 "from" "variant" "until" "and" "and then" "or" "or else" "xor"
 "detachable" "old" "∀" "∃" "¦" "all" "some"
 "implies" "once" (unary_not) "attribute" "agent" "like" "export" "all"
] @keyword

[ "frozen" "deferred" "inherit" "redefine" "undefine" "rename" "select" ] @keyword.modifier

(conditional ["if" "elseif" "end"] @keyword.conditional)
(else_part ["else"] @keyword.conditional)
(then_part ["then"] @keyword.conditional)
(conditional_expression ["if" "else" "elseif" "end"] @keyword.conditional)
(else_part_expression ["else"] @keyword.conditional)
(then_part_expression ["then"] @keyword.conditional)

(quantifier_loop ["∀" "∃" ":" "¦"] @keyword.repeat)
(quantifier_loop_body ["all" "some"] @keyword.repeat)
(iteration ["across" "as"] @keyword.repeat)
(initialization ["from"] @keyword.repeat)
(exit_condition ["until"] @keyword.repeat)
(loop (invariant "invariant" @keyword.repeat))
(loop ["⟳" ":" "¦" "⟲"]@keyword.repeat)
(loop "end" @keyword.repeat)
(loop_body ["loop"] @keyword.repeat)
(variant ["variant"] @keyword.repeat)

[["(" ")" "[" "]" "<<" ">>"]] @punctuation.bracket
[["," ":"]] @punctuation.delimiter
[[(unary) ":=" (binary_caret) (binary_mul_div) (binary_plus_minus)
  (binary_comparison) (binary_and) (binary_or) (binary_implies)
  (comparison)]] @operator
[(result)] @variable.builtin
(anchored (call (_) @variable))
[(verbatim_string) (basic_manifest_string)] @string
[(integer_constant) (real_constant)] @constant.numeric
[(boolean_constant)] @constant.boolean
[(void) (current)] @constant
(extended_feature_name (identifier) @function.method)

(iteration (identifier) @variable)
(quantifier_loop (identifier) @variable)
(entity_declaration_group (identifier) @variable)
[
 (class_name)
] @type

(extended_feature_name (identifier) @function)

[ (comment) ] @comment
[(header_comment)] @comment.documentation
