(object_id) @attribute

(string) @string
(escape_sequence) @constant.character.escape

(comment) @comment

(constant) @constant.builtin
(boolean) @constant.builtin.boolean

(template) @keyword

(using) @keyword.control.import

(decorator) @attribute

(property_definition (property_name) @variable.other.member)
(property_definition
  (property_binding
    "bind" @keyword
    (property_name) @variable.other.member
    ["no-sync-create" "bidirectional" "inverted"]* @keyword))

(object) @type

(signal_binding (signal_name) @function.builtin)
(signal_binding (function (identifier)) @function)
(signal_binding "swapped" @keyword)

(styles_list "styles" @function.macro)
(layout_definition "layout" @function.macro)

(gettext_string "_" @function.builtin)

(menu_definition "menu" @keyword)
(menu_section "section" @keyword)
(menu_item "item" @function.macro)

(template_definition (template_name_qualifier) @keyword.storage.type)

(import_statement (gobject_library) @namespace)

(import_statement (version_number) @constant.numeric.float)

(float) @constant.numeric.float
(number) @constant.numeric

[
  ";"
  "."
  ","
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket
