(identifier) @variable
(string_literal) @string
(number_literal) @constant.numeric
(boolean_literal) @constant.builtin.boolean
(comment) @comment

[
 (intrinsic_type)
 "allocatable"
 "attributes"
 "device"
 "dimension"
 "endtype"
 "global"
 "grid_global"
 "host"
 "import"
 "in"
 "inout"
 "intent"
 "optional"
 "out"
 "pointer"
 "type"
 "value"
 ] @keyword.storage.modifier

[
 "contains"
 "private"
 "public"
 ] @keyword.directive

[
 (none)
 "implicit"
 ] @attribute

[
 "endfunction"
 "endprogram"
 "endsubroutine"
 "function"
 "procedure"
 "subroutine"
 ] @keyword.function

[
 (default)
 (procedure_qualifier)
 "abstract"
 "bind"
 "call"
 "class"
 "continue"
 "cycle"
 "end"
 "endenum"
 "endinterface"
 "endmodule"
 "endprocedure"
 "endprogram"
 "endsubmodule"
 "enum"
 "enumerator"
 "equivalence"
 "exit"
 "extends"
 "format"
 "goto"
 "include"
 "interface"
 "intrinsic"
 "non_intrinsic"
 "module"
 "namelist"
 "only"
 "parameter"
 "print"
 "procedure"
 "program"
 "read"
 "stop"
 "submodule"
 "use"
 "write"
 ] @keyword

"return" @keyword.control.return

[
 "else"
 "elseif"
 "elsewhere"
 "endif"
 "endwhere"
 "if"
 "then"
 "where"
 ] @keyword.control.conditional

[
 "do"
 "enddo"
 "forall"
 "while"
 ] @keyword.control.repeat

[
 "*"
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
 "<<<"
 ">>>"
 ] @punctuation.bracket

;; Delimiter
[
 "::"
 ","
 "%"
 ] @punctuation.delimiter

[
  "defined"
  "#define"
  "#elif"
  "#else"
  "#endif"
  "#if"
  "#ifdef"
  "#ifndef"
  "#include"
 (preproc_directive)
] @keyword.directive

(parameters
  (identifier) @variable.parameter)

(program_statement
  (name) @namespace)

(module_statement
  (name) @namespace)

(submodule_statement
  (module_name) (name) @namespace)

(function_statement
  (name) @function)

(subroutine_statement
  (name) @function)

(module_procedure_statement
  (name) @function)

(end_program_statement
  (name) @namespace)

(end_module_statement
  (name) @namespace)

(end_submodule_statement
  (name) @namespace)

(end_function_statement
  (name) @function)

(end_subroutine_statement
  (name) @function)

(end_module_procedure_statement
  (name) @function)

(subroutine_call
  (identifier) @function)

(keyword_argument
  name: (identifier) @keyword)

(derived_type_member_expression
  (type_member) @variable.other.member)

(call_expression
  (identifier) @function)
