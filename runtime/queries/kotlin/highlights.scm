;;; Identifiers
(simple_identifier) @variable

; `field` keyword inside property getter/setter
; FIXME: This will highlight the keyword outside of getters and setters
;        since tree-sitter does not allow us to check for arbitrary nestation
((simple_identifier) @variable.builtin
(#eq? @variable.builtin "field"))

; `it` keyword inside lambdas
; FIXME: This will highlight the keyword outside of lambdas since tree-sitter
;        does not allow us to check for arbitrary nestation
((simple_identifier) @variable.builtin
(#eq? @variable.builtin "it"))


;;; Operators & Punctuation

[
	"."
	","
	";"
	":"
	"::"
] @punctuation.delimiter

[
	"(" ")"
	"[" "]"
	"{" "}"
] @punctuation.bracket

[
	"!"
	"!="
	"!=="
	"="
	"=="
	"==="
	">"
	">="
	"<"
	"<="
	"||"
	"&&"
	"+"
	"++"
	"+="
	"-"
	"--"
	"-="
	"*"
	"*="
	"/"
	"/="
	"%"
	"%="
	"?."
	"?:"
	"!!"
	"is"
	"!is"
	"in"
	"!in"
	"as"
	"as?"
	".."
	"->"
] @operator

(string_literal
	"$" @punctuation
  (interpolated_identifier) @none)
(string_literal
	"${" @punctuation
	(interpolated_expression) @none
	"}" @punctuation)

;;; Keywords

(type_alias "typealias" @keyword)
[
	(class_modifier)
	(member_modifier)
	(function_modifier)
	(property_modifier)
	(platform_modifier)
	(variance_modifier)
	(parameter_modifier)
	(visibility_modifier)
	(reification_modifier)
	(inheritance_modifier)
]@keyword

[
	"val"
	"var"
	"enum"
	"class"
	"object"
	"interface"
	"companion"
;	"typeof" ; NOTE: It is reserved for future use
] @keyword

("fun") @keyword.function

(jump_expression) @keyword.control.return

[
	"if"
	"else"
	"when"
] @keyword.control.conditional

[
	"for"
	"do"
	"while"
] @keyword.control.repeat

[
	"try"
	"catch"
	"throw"
	"finally"
] @keyword.control.exception

(annotation
	"@" @attribute (use_site_target)? @attribute)
(annotation
	(user_type
		(type_identifier) @attribute))
(annotation
	(constructor_invocation
		(user_type
			(type_identifier) @attribute)))

(file_annotation
	"@" @attribute "file" @attribute ":" @attribute)
(file_annotation
	(user_type
		(type_identifier) @attribute))
(file_annotation
	(constructor_invocation
		(user_type
			(type_identifier) @attribute)))

;;; Literals
; NOTE: Escapes not allowed in multi-line strings
(character_literal (character_escape_seq) @constant.character.escape)

(string_literal) @string

(character_literal) @constant.character

[
	(null_literal) ; should be highlighted the same as booleans
	(boolean_literal)
] @constant.builtin.boolean

(real_literal) @constant.numeric.float
[
	(integer_literal)
	(long_literal)
	(hex_literal)
	(bin_literal)
	(unsigned_literal)
] @constant.numeric.integer

[
	(line_comment)
	(multiline_comment)
	(shebang_line)
] @comment

;;; Function calls

(call_expression
	. (simple_identifier) @function.builtin
    (#match? @function.builtin "^(arrayOf|arrayOfNulls|byteArrayOf|shortArrayOf|intArrayOf|longArrayOf|ubyteArrayOf|ushortArrayOf|uintArrayOf|ulongArrayOf|floatArrayOf|doubleArrayOf|booleanArrayOf|charArrayOf|emptyArray|mapOf|setOf|listOf|emptyMap|emptySet|emptyList|mutableMapOf|mutableSetOf|mutableListOf|print|println|error|TODO|run|runCatching|repeat|lazy|lazyOf|enumValues|enumValueOf|assert|check|checkNotNull|require|requireNotNull|with|suspend|synchronized)$"))

; object.function() or object.property.function()
(call_expression
	(navigation_expression
		(navigation_suffix
			(simple_identifier) @function) . ))

; function()
(call_expression
	. (simple_identifier) @function)

;;; Function definitions

; lambda parameters
(lambda_literal
	(lambda_parameters
		(variable_declaration
			(simple_identifier) @variable.parameter)))
			
(parameter_with_optional_type
	(simple_identifier) @variable.parameter)
			
(parameter
	(simple_identifier) @variable.parameter)
			
(anonymous_initializer
	("init") @constructor)

(constructor_invocation
	(user_type
		(type_identifier) @constructor))
			
(secondary_constructor
	("constructor") @constructor)
(primary_constructor) @constructor
			
(getter
	("get") @function.builtin)
(setter
	("set") @function.builtin)

(function_declaration
	. (simple_identifier) @function)

; TODO: Separate labeled returns/breaks/continue/super/this
;       Must be implemented in the parser first
(label) @label

(import_header
	(identifier
		(simple_identifier) @function @_import .)
	(import_alias
		(type_identifier) @function)?
		(#match? @_import "^[a-z]"))

; The last `simple_identifier` in a `import_header` will always either be a function
; or a type. Classes can appear anywhere in the import path, unlike functions
(import_header
	(identifier
		(simple_identifier) @type @_import)
	(import_alias
		(type_identifier) @type)?
		(#match? @_import "^[A-Z]"))

(import_header
	"import" @keyword.control.import)

(package_header
	. (identifier)) @namespace

((type_identifier) @type.builtin
	(#match? @type.builtin "^(Byte|Short|Int|Long|UByte|UShort|UInt|ULong|Float|Double|Boolean|Char|String|Array|ByteArray|ShortArray|IntArray|LongArray|UByteArray|UShortArray|UIntArray|ULongArray|FloatArray|DoubleArray|BooleanArray|CharArray|Map|Set|List|EmptyMap|EmptySet|EmptyList|MutableMap|MutableSet|MutableList)$"))

(type_parameter
  (type_identifier) @type.parameter)

(type_identifier) @type

(enum_entry
	(simple_identifier) @constant)

(_
	(navigation_suffix
		(simple_identifier) @constant
		(#match? @constant "^[A-Z][A-Z0-9_]*$")))

; SCREAMING CASE identifiers are assumed to be constants
((simple_identifier) @constant
(#match? @constant "^[A-Z][A-Z0-9_]*$"))

; id_1.id_2.id_3: `id_2` and `id_3` are assumed as object properties
(_
	(navigation_suffix
		(simple_identifier) @variable.other.member))

(class_body
	(property_declaration
		(variable_declaration
			(simple_identifier) @variable.other.member)))

(class_parameter
	(simple_identifier) @variable.other.member)

; `super` keyword inside classes
(super_expression) @variable.builtin

; `this` this keyword inside classes
(this_expression) @variable.builtin
