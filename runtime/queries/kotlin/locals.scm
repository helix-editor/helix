;;; Imports

(package_header
	. (identifier) @definition.namespace)

(import_header
	(identifier
		(simple_identifier) @definition.import .)
	(import_alias
		(type_identifier) @definition.import)?)

;;; Functions

(function_declaration
	. (simple_identifier) @definition.function
	(#set! "definition.function.scope" "parent"))

(class_body
	(function_declaration
		. (simple_identifier) @definition.method)
	(#set! "definition.method.scope" "parent"))

;;; Variables

(function_declaration
	(parameter
		(simple_identifier) @definition.parameter))

(lambda_literal
	(lambda_parameters
		(variable_declaration
			(simple_identifier) @definition.parameter)))

(class_body
	(property_declaration
		(variable_declaration
			(simple_identifier) @definition.field)))

(class_declaration
	(primary_constructor
		(class_parameter
			(simple_identifier) @definition.field)))

(enum_class_body
	(enum_entry
		(simple_identifier) @definition.field))

(variable_declaration
	(simple_identifier) @definition.var)

;;; Types

(class_declaration
	(type_identifier) @definition.type
	(#set! "definition.type.scope" "parent"))

(type_alias
	(type_identifier) @definition.type
	(#set! "definition.type.scope" "parent"))

;;; Scopes

[
	(if_expression)
	(when_expression)
	(when_entry)

	(for_statement)
	(while_statement)
	(do_while_statement)

	(lambda_literal)
	(function_declaration)
	(primary_constructor)
	(secondary_constructor)
	(anonymous_initializer)

	(class_declaration)
	(enum_class_body)
	(enum_entry)

	(interpolated_expression)
] @scope
