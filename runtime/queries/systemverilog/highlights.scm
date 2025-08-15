;; Comments
(one_line_comment) @comment
(block_comment) @comment


;; Strings
(string_literal) @string
(quoted_string) @string ; `include strings
(system_lib_string) @string


;; Keywords
(["begin" "end" "this"]) @keyword
(["input" "output" "inout" "ref"]) @keyword
(["alias" "and" "assert" "assign" "assume" "before" "bind" "binsof" "break"
  "case" "checker" "class" "class" "clocking" "config" "const" "constraint"
  "cover" "covergroup" "coverpoint" "cross" "default" "defparam" "disable"
  "do" "else" "endcase" "endchecker" "endclass" "endclocking" "endconfig"
  "endfunction" "endgenerate" "endgroup" "endinterface" "endmodule"
  "endpackage" "endprogram" "endproperty" "endsequence" "endtask" "enum"
  "extends" "extern" "final" "first_match" "for" "force" "foreach" "forever"
  "fork" "forkjoin" "function" "generate" "genvar" "if" "iff" "illegal_bins"
  "implements" "import" "initial" "inside" "interconnect" "interface"
  "intersect" "join" "join_any" "join_none" "local" "localparam" "matches"
  "modport" "new" "null" "option" "or" "package" "packed" "parameter"
  "program" "property" "pure" "randcase" "randomize" "release" "repeat"
  "return" "sequence" "showcancelled" "soft" "solve" "struct" "super" "tagged"
  "task" "timeprecision" "timeunit" "type" "typedef" "union" "unique"
  "virtual" "wait" "while" "with"
  (always_keyword)             ; always, always_comb, always_latch, always_ff
  (bins_keyword)               ; bins, illegal_bins, ignore_bins
  (case_keyword)               ; case, casez, casex
  (class_item_qualifier)       ; static, protected, local
  (edge_identifier)            ; posedge, negedge, edge
  (lifetime)                   ; static, automatic
  (module_keyword)             ; module, macromodule
  (random_qualifier)           ; rand, randc
  (unique_priority)]) @keyword ; unique, unique0, priority


;; Preprocessor directives and macro usage
(["`include" "`define" "`ifdef" "`ifndef" "`timescale" "`default_nettype"
  "`elsif" "`undef" (resetall_compiler_directive) (undefineall_compiler_directive)
  "`endif" "`else" "`unconnected_drive" (celldefine_compiler_directive)
  (endcelldefine_compiler_directive) (endkeywords_directive) "`line"
  "`begin_keywords" "`pragma" "`__FILE__" "`__LINE__"]) @string.special
(text_macro_usage
 (simple_identifier) @string.special)


;; Delimiters, operators
([";" ":" "," "::"
  "=" "?" "|=" "&=" "^="
  "|->" "|=>" "->"
  ":=" ":/" "-:" "+:"]) @punctuation.delimiter
(["(" ")"]) @punctuation.bracket
(["[" "]"]) @punctuation.bracket
(["{" "}" "'{"]) @punctuation.bracket

(["."] @operator)
(["+" "-" "*" "/" "%" "**"]) @operator
(["<" "<=" ">" ">="]) @operator
(["===" "!==" "==" "!="]) @operator
(["&&" "||" "!"]) @operator
(["~" "&" "~&" "|" "~|" "^" "~^"]) @operator
(["<<" ">>" "<<<" ">>>"]) @operator

(["@" "#" "##"]) @operator
(assignment_operator) @operator
(unary_operator) @operator
(inc_or_dec_operator) @operator
(stream_operator) @operator
(event_trigger) @operator
(["->" "->>"]) @operator


;; Declarations
;; Module/interface/program/package/class/checker
(module_nonansi_header
 name: (simple_identifier) @function)
(module_ansi_header
 name: (simple_identifier) @function)
(interface_nonansi_header
 name: (simple_identifier) @function)
(interface_ansi_header
 name: (simple_identifier) @function)
(program_nonansi_header
 name: (simple_identifier) @function)
(program_ansi_header
 name: (simple_identifier) @function)
(package_declaration
 name: (simple_identifier) @function)
(class_declaration
 name: (simple_identifier) @function)
(interface_class_declaration
 name: (simple_identifier) @function)
(checker_declaration
 name: (simple_identifier) @function)
(class_declaration
 (class_type
  (simple_identifier) @type)) ; Parent class
;; Function/task/methods
(function_body_declaration
 name: (simple_identifier) @function)
(task_body_declaration
 name: (simple_identifier) @function)
(function_prototype
 (data_type_or_void)
 name: (simple_identifier) @function)
(task_prototype
 name: (simple_identifier) @function)
(class_scope ; Definition of extern defined methods
 (class_type
  (simple_identifier)) @function)


;; Types
[(integer_vector_type) ; bit, logic, reg
  (integer_atom_type)   ; byte, shortint, int, longint, integer, time
  (non_integer_type)    ; shortreal, real, realtime
  (net_type)            ; supply0, supply1, tri, triand, trior, trireg, tri0, tri1, uwire, wire, wand, wor
  ["string" "event" "signed" "unsigned" "chandle"]] @type
(data_type_or_implicit
 (data_type
  (simple_identifier)) @type)
(data_type
 (class_type
  (simple_identifier) @type
  (parameter_value_assignment)))
(data_type
 (class_type
  (simple_identifier) @operator
  (simple_identifier) @type))
(net_port_header
 (net_port_type
  (simple_identifier) @type))
(variable_port_header
 (variable_port_type
  (data_type
   (simple_identifier) @type)))
(["void'" (data_type_or_void)]) @type ; void cast of task called as a function
(interface_port_header ; Interfaces with modports
 interface_name: (simple_identifier) @type
 modport_name: (simple_identifier) @type)
(type_assignment
 name: (simple_identifier) @type)
(net_declaration ; User type variable declaration
 (simple_identifier) @type)
(enum_base_type ; Enum base type with user type
 (simple_identifier) @type)


;; Instances
;; Module names
(module_instantiation
 instance_type: (simple_identifier) @namespace)
(interface_instantiation
 instance_type: (simple_identifier) @namespace)
(program_instantiation
 instance_type: (simple_identifier) @namespace)
(checker_instantiation
 instance_type: (simple_identifier) @namespace)
(udp_instantiation
 instance_type: (simple_identifier) @namespace)
(gate_instantiation
 [(cmos_switchtype)
  (mos_switchtype)
  (enable_gatetype)
  (n_input_gatetype)
  (n_output_gatetype)
  (pass_en_switchtype)
  (pass_switchtype)
  "pulldown" "pullup"]
 @namespace)
;; Instance names
(name_of_instance
 instance_name: (simple_identifier) @constant)
;; Instance parameters
(module_instantiation
 (parameter_value_assignment
  (list_of_parameter_value_assignments
   (named_parameter_assignment
    (simple_identifier) @constant))))
(module_instantiation
 (parameter_value_assignment
  (list_of_parameter_value_assignments
   (ordered_parameter_assignment
    (param_expression
     (data_type
      (simple_identifier) @constant))))))
;; Port names
(named_port_connection
 port_name: (simple_identifier) @constant)
(named_parameter_assignment
 (simple_identifier) @constant)
(named_checker_port_connection
 port_name: (simple_identifier) @constant)
;; Bind statements
(bind_directive
 (bind_target_scope
  (simple_identifier) @constant))


;; Numbers
(hex_number
 size: (unsigned_number) @constant.numeric
 base: (hex_base) @punctuation.delimiter)
(decimal_number
 size: (unsigned_number) @constant.numeric
 base: (decimal_base) @punctuation.delimiter)
(octal_number
 size: (unsigned_number) @constant.numeric
 base: (octal_base) @punctuation.delimiter)
(binary_number
 size: (unsigned_number) @constant.numeric
 base: (binary_base) @punctuation.delimiter)
;; Same as before but without the width (width extension)
(hex_number
 base: (hex_base) @punctuation.delimiter)
(decimal_number
 base: (decimal_base) @punctuation.delimiter)
(octal_number
 base: (octal_base) @punctuation.delimiter)
(binary_number
 base: (binary_base) @punctuation.delimiter)


;; Arrays
(unpacked_dimension
 [(constant_expression) (constant_range)] @constant.numeric)
(packed_dimension
 (constant_range) @constant.numeric)
(select
 (constant_range) @constant.numeric)
(constant_select
 (constant_range
  (constant_expression) @constant.numeric))
(constant_bit_select
 (constant_expression) @constant.numeric)
(bit_select
 (expression) @constant.numeric)
(indexed_range
 (expression) @constant.numeric
 (constant_expression) @constant.numeric)
(constant_indexed_range
 (constant_expression) @constant.numeric)
(value_range ; inside {[min_range:max_range]}, place here to apply override
 (expression) @constant)
(dynamic_array_new
 (expression) @constant)


;; Misc
;; Timeunit
((time_unit) @constant.builtin)
;; Enum labels
(enum_name_declaration
 (simple_identifier) @constant.builtin)
;; Case item label (not radix)
(case_item_expression
 (expression
  (primary
   (hierarchical_identifier
    (simple_identifier) @constant.builtin))))
;; Hierarchical references, interface signals, class members, package scope
(hierarchical_identifier
 (simple_identifier) @punctuation.delimiter
 "."
 (simple_identifier))
(method_call
 (primary) @punctuation.delimiter
 (["." "::"])
 (method_call_body))
(package_scope
 (simple_identifier) @punctuation.delimiter)
(method_call
 (primary
  (select
   (simple_identifier) @punctuation.delimiter))
 (method_call_body))
;; Attributes
(["(*" "*)"] @constant)
(attribute_instance
 (attr_spec (simple_identifier) @attribute))
;; Typedefs
(type_declaration
 (class_type (simple_identifier) @type)
 type_name: (simple_identifier) @constant)
(type_declaration
 type_name: (simple_identifier) @constant)
("typedef" "class" (simple_identifier) @constant)
;; Coverpoint & cross labels
(cover_point
 name: (simple_identifier) @constant)
(cover_cross
 name: (simple_identifier) @constant)
;; Loop variables (foreach[i])
(loop_variables
 (simple_identifier) @constant)
;; Bins values
(bins_or_options
 (expression
  (primary
   (concatenation
    (expression) @constant))))
;; Bins ranges
(covergroup_value_range
 (expression) @constant)
;; Queue dimension
(("$") @punctuation.special)
;; Parameterized classes (e.g: uvm_config_db #(axi_stream_agent_config))
(class_type
 (parameter_value_assignment
  (list_of_parameter_value_assignments) @punctuation.delimiter))


;; System-tf
([(system_tf_identifier)               ; System task/function
  "$fatal" "$error" "$warning" "$info" ; (severity_system_task)
  "$stop" "$finish" "$exit"])          ; (simulation_control_task)
@function.builtin
