;; highlight queries.
;; See the syntax at https://tree-sitter.github.io/tree-sitter/using-parsers#pattern-matching-with-queries
;; See also https://github.com/nvim-treesitter/nvim-treesitter/blob/master/CONTRIBUTING.md#parser-configurations
;; for a list of recommended @ tags, though not all of them have matching
;; highlights in neovim.

[
   "abort"
   "abs"
   "abstract"
   "accept"
   "access"
   "all"
   "array"
   "at"
   "begin"
   "declare"
   "delay"
   "delta"
   "digits"
   "do"
   "end"
   "entry"
   "exit"
   "generic"
   "interface"
   "is"
   "limited"
   "of"
   "others"
   "out"
   "pragma"
   "private"
   "range"
   "synchronized"
   "tagged"
   "task"
   "terminate"
   "until"
   "when"
] @keyword
[
   "null"
] @constant.builtin
[
   "aliased"
   "constant"
   "renames"
] @keyword.storage
[
   "mod"
   "new"
   "protected"
   "record"
   "subtype"
   "type"
] @type.builtin
[
   "with"
   "use"
] @keyword.control.import
[
   "body"
   "function"
   "overriding"
   "procedure"
   "package"
   "separate"
] @keyword.function
[
   "and"
   "in"
   "not"
   "or"
   "xor"
] @operator
[
   "while"
   "loop"
   "for"
   "parallel"
   "reverse"
   "some"
] @kewyord.control.repeat
[
   "return"
] @keyword.control.return
[
   "case"
   "if"
   "else"
   "then"
   "elsif"
   "select"
] @keyword.control.conditional
[
   "exception"
   "raise"
] @keyword.control.exception
(comment)         @comment
(string_literal)  @string
(character_literal) @string
(numeric_literal) @constant.numeric

;; Highlight the name of subprograms
(procedure_specification name: (_) @function.builtin)
(function_specification name: (_) @function.builtin)
(package_declaration name: (_) @function.builtin)
(package_body name: (_) @function.builtin)
(generic_instantiation name: (_) @function.builtin)
(entry_declaration . (identifier) @function.builtin)

;; Some keywords should take different categories depending on the context
(use_clause "use"  @keyword.control.import "type" @keyword.control.import)
(with_clause "private" @keyword.control.import)
(with_clause "limited" @keyword.control.import)
(use_clause (_) @namespace)
(with_clause (_) @namespace)

(loop_statement "end" @keyword.control.repeat)
(if_statement "end" @keyword.control.conditional)
(loop_parameter_specification "in" @keyword.control.repeat)
(loop_parameter_specification "in" @keyword.control.repeat)
(iterator_specification ["in" "of"] @keyword.control.repeat)
(range_attribute_designator "range" @keyword.control.repeat)

(raise_statement "with" @keyword.control.exception)

(gnatprep_declarative_if_statement)  @keyword.directive
(gnatprep_if_statement)              @keyword.directive
(gnatprep_identifier)                @keyword.directive

(subprogram_declaration "is" @keyword.function "abstract"  @keyword.function)
(aspect_specification "with" @keyword.function)

(full_type_declaration "is" @type.builtin)
(subtype_declaration "is" @type.builtin)
(record_definition "end" @type.builtin)
(full_type_declaration (_ "access" @type.builtin))
(array_type_definition "array" @type.builtin "of" @type.builtin)
(access_to_object_definition "access" @type.builtin)
(access_to_object_definition "access" @type.builtin
   [
      (general_access_modifier "constant" @type.builtin)
      (general_access_modifier "all" @type.builtin)
   ]
)
(range_constraint "range" @type.builtin)
(signed_integer_type_definition "range" @type.builtin)
(index_subtype_definition "range" @type.builtin)
(record_type_definition "abstract" @type.builtin)
(record_type_definition "tagged" @type.builtin)
(record_type_definition "limited" @type.builtin)
(record_type_definition (record_definition "null" @type.builtin))
(private_type_declaration "is" @type.builtin "private" @type.builtin)
(private_type_declaration "tagged" @type.builtin)
(private_type_declaration "limited" @type.builtin)
(task_type_declaration "task" @type.builtin "is" @type.builtin)

;; Gray the body of expression functions
(expression_function_declaration
   (function_specification)
   "is"
   (_) @attribute
)
(subprogram_declaration (aspect_specification) @attribute)

;; Highlight full subprogram specifications
; (subprogram_body
;     [
;        (procedure_specification)
;        (function_specification)
;     ] @function.builtin.spec
; )


