(line_comment) @comment.line

(block_comment) @comment.block

(identifier) @variable

[
  "access"
  "after"
  "alias"
  "architecture"
  "array"
  "attribute"
  "block"
  "body"
  "component"
  "configuration"
  "context"
  "disconnect"
  "entity"
  "file"
  "force"
  "generate"
  "generic"
  "group"
  "label"
  "literal"
  "map"
  "new"
  "package"
  "parameter"
  "port"
  "property"
  "range"
  "reject"
  "release"
  "sequence"
  "transport"
  "unaffected"
  "view"
  "vunit"
] @keyword

[
  (ALL)
  (OTHERS)
  "<>"
  (DEFAULT)
  (OPEN)
] @constant.builtin

[
  "is"
  "begin"
  "end"
] @keyword

(parameter_specification
  "in" @keyword)

[
  "process"
  "wait"
  "on"
  "until"
] @keyword

(timeout_clause
  "for" @keyword)

[
  "function"
  "procedure"
] @keyword.function

[
  "to"
  "downto"
  "of"
] @keyword.operator

[
  "library"
  "use"
] @keyword.control.import

[
  "subtype"
  "type"
  "record"
  "units"
  "constant"
  "signal"
  "variable"
] @keyword.storage.type

[
  "protected"
  "private"
  "pure"
  "impure"
  "inertial"
  "postponed"
  "guarded"
  "out"
  "inout"
  "linkage"
  "buffer"
  "register"
  "bus"
  "shared"
] @keyword.storage.modifier

(mode
  "in" @keyword.storage.modifier)

(force_mode
  "in" @keyword.storage.modifier)

[
  "while"
  "loop"
  "next"
  "exit"
] @keyword.control.repeat

(for_loop
  "for" @keyword.control.repeat)

(block_configuration
  "for" @keyword)

(configuration_specification
  "for" @keyword)

(component_configuration
  "for" @keyword)

(end_for
  "for" @keyword)

"return" @keyword.control.return

[
  "assert"
  "report"
  "severity"
] @keyword

[
  "if"
  "then"
  "elsif"
  "case"
] @keyword.control.conditional

(when_element
  "when" @keyword.control.conditional)

(case_generate_alternative
  "when" @keyword.control.conditional)

(else_statement
  "else" @keyword.control.conditional)

(else_generate
  "else" @keyword.control.conditional)

[
  "with"
  "select"
] @keyword.control.conditional

(when_expression
  "when" @keyword.control.conditional)

(else_expression
  "else" @keyword.control.conditional)

(else_waveform
  "else" @keyword.control.conditional)

(else_expression_or_unaffected
  "else" @keyword.control.conditional)

"null" @constant.builtin

(user_directive) @keyword.directive

(protect_directive) @keyword.directive

(warning_directive) @keyword.directive

(error_directive) @keyword.directive

(if_conditional_analysis
  "if" @keyword.directive)

(if_conditional_analysis
  "then" @keyword.directive)

(elsif_conditional_analysis
  "elsif" @keyword.directive)

(else_conditional_analysis
  "else" @keyword.directive)

(end_conditional_analysis
  "end" @keyword.directive)

(end_conditional_analysis
  "if" @keyword.directive)

(directive_body) @keyword.directive

(directive_constant_builtin) @constant.builtin

(directive_error) @keyword.directive

(directive_protect) @keyword.directive

(directive_warning) @keyword.directive

[
  (condition_conversion)
  (relational_operator)
  (sign)
  (adding_operator)
  (exponentiate)
  (variable_assignment)
  (signal_assignment)
  "*"
  "/"
  ":"
  "=>"
] @operator

[
  (unary_operator)
  (logical_operator)
  (shift_operator)
  "mod"
  "not"
  "rem"
] @keyword.operator

[
  "'"
  ","
  "."
  ";"
] @punctuation.delimiters

[
  "("
  ")"
  "["
  "]"
  "<<"
  ">>"
] @punctuation.bracket

"@" @punctuation.special

[
  (decimal_integer)
  (string_literal_std_logic)
] @constant.numeric.integer

(decimal_float) @constant.numeric.float

(bit_string_length) @type.parameter

(bit_string_base) @type.builtin

(bit_string_value) @constant.numeric.integer

(based_literal
  (based_base) @type.builtin
  (based_integer) @constant.numeric.integer)

(based_literal
  (based_base) @type.builtin
  (based_float) @constant.numeric.float)

(string_literal) @string

(character_literal) @constant.character

(library_constant_std_logic) @constant.builtin

(library_constant) @constant.builtin

(library_function) @function.builtin

(library_constant_boolean) @constant.builtin.boolean

(library_constant_character) @constant.character

(unit) @keyword.storage.modifier

(library_constant_unit) @keyword.storage.modifier

(label) @label

(generic_map_aspect
  "generic" @constructor
  "map" @constructor)

(port_map_aspect
  "port" @constructor
  "map" @constructor)

(selection
  (identifier) @variable.other.member)

(_
  view: (_) @type)

(_
  type: (_) @type)

(_
  library: (_) @namespace)

(_
  package: (_) @namespace)

(_
  entity: (_) @namespace)

(_
  component: (_) @namespace)

(_
  configuration: (_) @type.parameter)

(_
  architecture: (_) @type.parameter)

(_
  function: (_) @function)

(_
  procedure: (_) @function.method)

(_
  attribute: (_) @attribute)

(_
  constant: (_) @constant)

(_
  generic: (_) @variable.parameter)

(_
  view: (name
    (_)) @type)

(_
  type: (name
    (_)) @type)

(_
  entity: (name
    (_)) @namespace)

(_
  component: (name
    (_)) @namespace)

(_
  configuration: (name
    (_)) @namespace)

(library_type) @type.builtin

