
;; Primitives
(boolean) @constant.builtin.boolean
(comment) @comment
(shebang_comment) @comment
(identifier) @variable
((identifier) @variable.builtin
  (#eq? @variable.builtin "self"))
(nil) @constant.builtin
(number) @constant.numeric
(string) @string
(table_constructor ["{" "}"] @constructor)
(varargs "..." @constant.builtin)
[ "," "." ":" ";" ] @punctuation.delimiter

(escape_sequence) @constant.character.escape
(format_specifier) @constant.character.escape

;; Basic statements/Keywords
[ "if" "then" "elseif" "else" ] @keyword.control.conditional
[ "for" "while" "repeat" "until" "do" ] @keyword.control.repeat
[ "end" ] @keyword
[ "in" ] @keyword.operator
[ "local" ] @keyword.storage.type
[ (break) (goto) ] @keyword.control
[ "return" ] @keyword.control.return
(label) @label

;; Global isn't a real keyword, but it gets special treatment in these places
(var_declaration "global" @keyword.storage.type)
(type_declaration "global" @keyword.storage.type)
(function_statement "global" @keyword.storage.type)
(record_declaration "global" @keyword.storage.type)
(interface_declaration "global" @keyword.storage.type)
(enum_declaration "global" @keyword.storage.type)

(macroexp_statement "macroexp" @keyword)

;; Ops
(bin_op (op) @operator)
(unary_op (op) @operator)
[ "=" "as" ] @operator

;; Functions
(function_statement
  "function" @keyword.function
  . name: (_) @function)
(anon_function
  "function" @keyword.function)
(function_body "end" @keyword.function)

(arg name: (identifier) @variable.parameter)

(function_signature
  (arguments
    . (arg name: (identifier) @variable.builtin))
  (#eq? @variable.builtin "self"))

(typeargs
  "<" @punctuation.bracket
  . (_) @type.parameter
  . ("," . (_) @type.parameter)*
  . ">" @punctuation.bracket)

(function_call
  (identifier) @function . (arguments))
(function_call
  (index (_) key: (identifier) @function) . (arguments))
(function_call
  (method_index (_) key: (identifier) @function) . (arguments))

;; Types

; Contextual keywords in record bodies
(record_declaration
  . [ "record" ] @keyword.storage.type
  name: (identifier) @type)
(anon_record . "record" @keyword.storage.type)
(record_body
  (record_declaration
    . [ "record" ] @keyword.storage.type
    . name: (identifier) @type))
(record_body
  (enum_declaration
    . [ "enum" ] @keyword.storage.type
    . name: (identifier) @type.enum))
(record_body
  (interface_declaration
    . [ "interface" ] @keyword.storage.type
    . name: (identifier) @type))
(record_body
  (typedef
    . "type" @keyword.storage.type
    . name: (identifier) @type . "="))
(record_body
  (macroexp_declaration
    . [ "macroexp" ] @keyword.storage.type))
(record_body (metamethod "metamethod" @keyword.storage.modifier))
(record_body (userdata) @keyword.storage.modifier)

; Contextual keywords in interface bodies
(interface_declaration
  . [ "interface" ] @keyword.storage.type
  name: (identifier) @type)
(anon_interface . "interface" @keyword.storage.type)
(interface_body
  (record_declaration
    . [ "record" ] @keyword.storage.type
    . name: (identifier) @type))
(interface_body
  (enum_declaration
    . [ "enum" ] @keyword.storage.type
    . name: (identifier) @type.enum))
(interface_body
  (interface_declaration
    . [ "interface" ] @keyword.storage.type
    . name: (identifier) @type))
(interface_body
  (typedef
    . "type" @keyword.storage.type
    . name: (identifier) @type . "="))
(interface_body
  (macroexp_declaration
    . [ "macroexp" ] @keyword.storage.type))
(interface_body (metamethod "metamethod" @keyword.storage.modifier))
(interface_body (userdata) @keyword.storage.modifier)

(enum_declaration
  "enum" @keyword.storage.type
  name: (identifier) @type.enum)

(type_declaration "type" @keyword.storage.type)
(type_declaration (identifier) @type)
(simple_type) @type
(type_index) @type
(type_union "|" @operator)
(function_type "function" @type)

;; The rest of it
(var_declaration
  declarators: (var_declarators
      (var name: (identifier) @variable)))
(var_declaration
  declarators: (var_declarators
    (var
      "<" @punctuation.bracket
      . attribute: (attribute) @attribute
      . ">" @punctuation.bracket)))
[ "(" ")" "[" "]" "{" "}" ] @punctuation.bracket

;; Only highlight format specifiers in calls to string.format
;; string.format('...')
;(function_call
;  called_object: (index
;    (identifier) @base
;    key: (identifier) @entry)
;  arguments: (arguments .
;    (string (format_specifier) @string.escape))
;
;  (#eq? @base "string")
;  (#eq? @entry "format"))

;; ('...'):format()
;(function_call
;  called_object: (method_index
;    (string (format_specifier) @string.escape)
;    key: (identifier) @func-name)
;    (#eq? @func-name "format"))


