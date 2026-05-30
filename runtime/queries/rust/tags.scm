; Definitions

(struct_item
  name: (type_identifier) @name) @definition.struct

(field_declaration
  name: (field_identifier) @name) @definition.field

(const_item
  name: (identifier) @name) @definition.constant

(trait_item
  name: (type_identifier) @name) @definition.interface

(function_item
  name: (identifier) @name) @definition.function

(function_signature_item
  name: (identifier) @name) @definition.function

(enum_item
  name: (type_identifier) @name) @definition.enum

(enum_variant
  name: (identifier) @name) @definition.struct

(type_item
  name: (type_identifier) @name) @definition.type

(mod_item
  name: (identifier) @name) @definition.module

(macro_definition
  name: (identifier) @name) @definition.macro

; References

; Function and method calls: foo(), self.foo(), Foo::foo()
(call_expression
  function: [
    (identifier) @name
    (scoped_identifier name: (identifier) @name)
    (field_expression field: (field_identifier) @name)
  ]) @reference.function

; Generic function calls: foo::<T>(), self.foo::<T>()
(generic_function
  function: [
    (identifier) @name
    (scoped_identifier name: (identifier) @name)
    (field_expression field: (field_identifier) @name)
  ]) @reference.function

; Macro invocations: foo!()
(macro_invocation
  macro: [
    (identifier) @name
    (scoped_identifier name: (identifier) @name)
  ]) @reference.macro

; Struct construction: Foo { ... }
(struct_expression
  name: [
    (type_identifier) @name
    (scoped_type_identifier name: (type_identifier) @name)
  ]) @reference.struct

; Struct/enum patterns in match: Foo { .. }, Foo::Bar { .. }
(struct_pattern
  type: [
    (type_identifier) @name
    (scoped_type_identifier name: (type_identifier) @name)
  ]) @reference.struct

; impl blocks: impl Foo { ... }
(impl_item
  type: (type_identifier) @name) @reference.struct

; Trait being implemented: impl Trait for Type
(impl_item
  trait: [
    (type_identifier) @name
    (scoped_type_identifier name: (type_identifier) @name)
  ]) @reference.interface

; Field access: foo.bar
(field_expression
  field: (field_identifier) @name) @reference.field

; Explicit field init: Foo { bar: val }
(field_initializer
  (field_identifier) @name) @reference.field

; Shorthand field init: Foo { bar }
(shorthand_field_initializer
  (identifier) @name) @reference.field
