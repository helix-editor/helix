; Keywords

[
  ; block delimiters
  (module_keyword)
  "endmodule"
  "program"
  "endprogram"
  "class"
  "endclass"
  "interface"
  "endinterface"
  "package"
  "endpackage"
  "checker"
  "endchecker"
  "config"
  "endconfig"

  "pure"
  "virtual"
  "extends"
  "implements"
  "super"
  (class_item_qualifier)

  "parameter"
  "localparam"
  "defparam"
  "assign"
  "typedef"
  "modport"
  "fork"
  "join"
  "join_none"
  "join_any"
  "default"
  "break"
  "assert"
  (unique_priority)
  "tagged"
  "extern"
] @keyword

[
  "function"
  "endfunction"

  "task"
  "endtask"
] @keyword.function

"return" @keyword.control.return

[
  "begin"
  "end"
] @label

[
  (always_keyword)
  "generate"
  "for"
  "foreach"
  "repeat"
  "forever"
  "initial"
  "while"
] @keyword.control

[
  "if"
  "else"
  (case_keyword)
  "endcase"
] @keyword.control.conditional

(comment) @comment

(include_compiler_directive) @keyword.directive
(package_import_declaration
 "import" @keyword.control.import)

(package_import_declaration
 (package_import_item
  (package_identifier
   (simple_identifier) @constant)))

(text_macro_identifier
 (simple_identifier) @keyword.directive)

(package_scope
 (package_identifier
  (simple_identifier) @constant))

(package_declaration
 (package_identifier
  (simple_identifier) @constant))

(parameter_port_list
 "#" @constructor)

[
  "="
  "-"
  "+"
  "/"
  "*"
  "^"
  "&"
  "|"
  "&&"
  "||"
  ":"
  (unary_operator)
  "{"
  "}"
  "'{"
  "<="
  "@"
  "or"
  "and"
  "=="
  "!="
  "==="
  "!=="
  "-:"
  "<"
  ">"
  ">="
  "%"
  ">>"
  "<<"
  "|="
  (inc_or_dec_operator)
] @keyword.operator

(cast
 ["'" "(" ")"] @keyword.operator)

(edge_identifier) @constant

(port_direction) @label
(port_identifier
 (simple_identifier) @variable)

[
  (net_type)
  (integer_vector_type)
  (integer_atom_type)
] @type.builtin

[
  "signed"
  "unsigned"
] @label

(data_type
 (simple_identifier) @type)

(method_call_body
  (method_identifier) @variable.other.member)

(interface_identifier
 (simple_identifier) @type)

(modport_identifier
 (modport_identifier
  (simple_identifier) @variable.other.member))

(net_port_type1
 (simple_identifier) @type)

[
  (double_quoted_string)
  (string_literal)
] @string

[
  (include_compiler_directive)
  (default_nettype_compiler_directive)
  (timescale_compiler_directive)
] @keyword.directive

; begin/end label
(seq_block
 (simple_identifier) @comment)

[
 ";"
 "::"
 ","
 "."
] @punctuation.delimiter


(default_nettype_compiler_directive
 (default_nettype_value) @string)

(text_macro_identifier
 (simple_identifier) @function.macro)

(module_declaration
 (module_header
  (simple_identifier) @constructor))

(class_constructor_declaration
 "new" @constructor)

(parameter_identifier
 (simple_identifier) @variable.parameter)

[
  (integral_number)
  (unsigned_number)
  (unbased_unsized_literal)
] @constant.numeric

(time_unit) @constant

(checker_instantiation
 (checker_identifier
  (simple_identifier) @constructor))

(module_instantiation
 (simple_identifier) @constructor)

(name_of_instance
 (instance_identifier
  (simple_identifier) @variable))

(interface_port_declaration
 (interface_identifier
  (simple_identifier) @type))

(net_declaration
 (simple_identifier) @type)

(lifetime) @label

(function_identifier 
 (function_identifier 
  (simple_identifier) @function))

(function_subroutine_call 
 (subroutine_call
  (tf_call
   (simple_identifier) @function)))

(function_subroutine_call 
 (subroutine_call
  (system_tf_call
   (system_tf_identifier) @function.builtin)))

(task_identifier
 (task_identifier
  (simple_identifier) @function.method))

;;TODO: fixme
;(assignment_pattern_expression
 ;(assignment_pattern
  ;(parameter_identifier) @variable.other.member))

(type_declaration
  (data_type ["packed"] @label))

(struct_union) @type

[
  "enum"
] @type

(enum_name_declaration
 (enum_identifier
  (simple_identifier) @constant))

(type_declaration
 (simple_identifier) @type)

[
  (integer_atom_type)
  (non_integer_type)
  "genvar"
] @type.builtin

(struct_union_member
 (list_of_variable_decl_assignments
  (variable_decl_assignment
   (simple_identifier) @variable.other.member)))

(member_identifier
 (simple_identifier) @variable.other.member)

(struct_union_member
 (data_type_or_void
  (data_type
   (simple_identifier) @type)))

(type_declaration
 (simple_identifier) @type)

(generate_block_identifier) @comment

[
  "["
  "]"
  "("
  ")"
] @punctuation.bracket

(ERROR) @error
