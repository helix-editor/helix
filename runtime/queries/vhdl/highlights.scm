[
  "alias" "package" "file" "entity" "architecture" "type" "subtype"
  "attribute" "to" "downto" "signal" "variable" "record" "array"
  "others" "process" "component" "shared" "constant" "port" "generic"
  "generate" "range" "map" "in" "inout" "of" "out" "configuration"
  "pure" "impure" "is" "begin" "end" "context" "wait" "until" "after"
  "report" "open" "exit" "assert" "next" "null" "force" "property"
  "release" "sequence" "transport" "unaffected" "select" "severity"
  "register" "reject" "postponed" "on" "new" "literal" "linkage"
  "inertial" "guarded" "group" "disconnect" "bus" "buffer" "body"
  "all" "block" "access"
] @keyword

[
  "function" "procedure"
] @keyword.function

[
  "return"
] @keyword.control.return

[
  "for" "loop" "while"
] @keyword.control.repeat

[ 
  "if" "elsif" "else" "case" "then" "when"
] @keyword.control.conditional

[ 
  "library" "use"
] @keyword.control.import

(comment) @comment

(type_mark) @type

[
  "(" ")" "[" "]"
] @punctuation.bracket

[
  "." ";" "," ":"
] @punctuation.delimiter

[
  "=>" "<=" "+" ":=" "=" "/=" "<" ">" "-" "*"
  "**" "/" "?>" "?<" "?<=" "?>=" "?=" "?/="
; "?/" errors, maybe due to escape character
  (attribute_name "'")
  (index_subtype_definition (any))
] @operator

[
  "not" "xor" "xnor" "and" "nand" "or" "nor" "mod" "rem"
  (attribute_name "'")
  (index_subtype_definition (any))
] @keyword.operator

[
  (real_decimal)
  (integer_decimal)
] @constant.numeric

(character_literal) @constant.character

[
  (string_literal)
  (bit_string_literal)
] @string

(physical_literal
  unit: (simple_name) @attribute)

(attribute_name
  prefix: (_) @variable
  designator: (_) @attribute)

((simple_name) @variable.builtin (#any-of? @variable.builtin
  "true" "false" "now"))

(severity_expression) @constant.builtin

(procedure_call_statement
  procedure: (simple_name) @function)

(ambiguous_name
  prefix: (simple_name) @function.builtin (#any-of? @function.builtin
    "rising_edge" "falling_edge" "find_rightmost" "find_leftmost"
    "maximum" "minimum" "shift_left" "shift_right" "rotate_left"
    "rotate_right" "sll" "srl" "rol" "ror" "sla" "sra" "resize"
    "mod" "rem" "abs" "saturate"
    "to_sfixed" "to_ufixed" "to_signed" "to_unsigned" "to_real"
    "to_integer" "sfixed_low" "ufixed_low" "sfixed_high"
    "ufixed_high" "to_slv" "to_stdulogicvector" "to_sulv"
    "to_float" "std_logic" "std_logic_vector" "integer" "signed"
    "unsigned" "real" "std_ulogic_vector"
    "std_ulogic" "x01" "x01z" "ux01" "ux01Z"
;math_real
    "sign" "ceil" "floor" "round" "fmax" "fmin" "uniform" "srand"
    "rand" "get_rand_max" "sqrt" "cbrt" "exp" "log" "log2" "log10"
    "sin" "cos" "tan" "asin" "acos" "atan" "atan2" "sinh" "cosh"
    "tanh" "asinh" "acosh" "atanh" "realmax" "realmin" "trunc"
    "conj" "arg" "polar_to_complex" "complex_to_polar"
    "get_principal_value" "cmplx"
;std_textio
    "read" "write" "hread" "hwrite" "to_hstring" "to_string"
    "from_hstring" "from_string"
    "justify" "readline" "sread" "string_read" " bread"
    "binary_read" "oread" "octal_read" "hex_read"
    "writeline" "swrite" "string_write" "bwrite"
    "binary_write" "owrite" "octal_write" "hex_write"
    "synthesis_return"
;std_logic_1164
    "resolved" "logic_type_encoding" "is_signed" "to_bit"
    "to_bitvector" "to_stdulogic" "to_stdlogicvector"
    "to_bit_vector" "to_bv" "to_std_logic_vector"
    "to_std_ulogic_vector" "to_01" "to_x01" "to_x01z" "to_ux01"
    "is_x" "to_bstring" "to_binary_string" "to_ostring"
    "to_octal_string" "to_hex_string"
;float_pkg
    "add" "subtract" "multiply" "divide" "remainder" "modulo"
    "reciprocal" "dividebyp2" "mac" "eq" "ne" "lt" "gt" "le" "ge"
    "to_float32" "to_float64" "to_float128" "realtobits" "bitstoreal"
    "break_number" "normalize" "copysign" "scalb" "logb" "nextafter"
    "unordered" "finite" "isnan" "zerofp" "nanfp" "qnanfp"
    "pos_inffp" "neg_inffp" "neg_zerofp" "from_bstring"
    "from_binary_string" "from_ostring" "from_octal_string"
    "from_hex_string"
;fixed_pkg
    "add_carry" "to_ufix" "to_sfix" "ufix_high"
    "ufix_low" "sfix_high" "sfix_low"
))

