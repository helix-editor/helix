[
  (intrinsic_type)
  "dimension"
  "intent"
  "in"
  "out"
  "inout"
  "type"
  "endtype"
  "attributes"
  "global"
  "device"
  "host"
  "grid_global"
  "pointer"
] @keyword.storage.modifier

[
  "contains"
  "public"
  "private"
] @keyword.directive

[
"implicit"
(none)
] @attribute

[
  "function"
  "endfunction"
  "endprogram"
  "subroutine"
  "endsubroutine"
] @keyword.storage

[
  "module"
  "endmodule"
  "bind"
  "call"
  "class"
  "continue"
  "cycle"
  "enumerator"
  "equivalence"
  "exit"
  "format"
  "goto"
  "include"
  "interface"
  "endinterface"
  "only"
  "parameter"
  "procedure"
  "print"
  "program"
  "endprogram"
  "read"
  "return"
  "result"
  "stop"
  "use"
  "write"
  "enum"
  "endenum"
  (default)
  (procedure_qualifier)
] @keyword

[
  "if" 
  "then"
  "else"
  "elseif"
  "endif"
  "where"
  "endwhere"
] @keyword.control.conditional

[
  "do"
  "enddo"
  "while"
  "forall"
] @keyword.control.repeat

[
  "*"
  "**"
  "+"
  "-"
  "/"
  "="
  "<"
  ">"
  "<="
  ">="
  "=="
  "/="
] @operator

[
  "\\.and\\."
  "\\.or\\."
  "\\.lt\\."
  "\\.gt\\."
  "\\.ge\\."
  "\\.le\\."
  "\\.eq\\."
  "\\.eqv\\."
  "\\.neqv\\."
] @keyword.operator

 ;; Brackets
 [
  "("
  ")"
  "["
  "]"
 ] @punctuation.bracket

 ;; Delimiter
 [
  "::"
  ","
  "%"
 ] @punctuation.delimiter

(parameters
  (identifier) @variable.parameter)

(program_statement
  (name) @namespace)

(module_statement
  (name) @namespace)

(function_statement
  (name) @function)

(subroutine_statement
  (name) @function)

(end_program_statement
  (name) @namespace)

(end_module_statement
  (name) @namespace)

(end_function_statement
  (name) @function)

(end_subroutine_statement
  (name) @function)

(subroutine_call
	(name) @function)

(keyword_argument
  name: (identifier) @keyword)

(derived_type_member_expression
  (type_member) @variable.other.member)

(identifier) @variable
(string_literal) @string
(number_literal) @constant.numeric
(boolean_literal) @constant.builtin.boolean
(comment) @comment

