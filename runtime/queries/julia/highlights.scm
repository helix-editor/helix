; ------------
; Variables identifiers
; ------------

(identifier) @variable

; Remaining identifiers that start with capital letters should be types (PascalCase)
(
  (identifier) @type
  (#match? @type "^[A-Z]"))

; SCREAMING_SNAKE_CASE
(
  (identifier) @constant
  (#match? @constant "^[A-Z][A-Z0-9_]*$"))

(const_statement
  (assignment
    . (identifier) @constant))

; Field expressions are either module content or struct fields.
; Module types and constants should already be captured, so this
; assumes the remaining identifiers to be struct fields.
(field_expression
  (_)
  (identifier) @variable.other.member)

(quote_expression
  ":" @string.special.symbol
  [
    (identifier)
    (operator)
  ] @string.special.symbol)

; ------
; Macros
; ------

(macro_definition
  name: (identifier) @function.macro)

(macro_identifier
  "@" @function.macro
  (identifier) @function.macro)

; -------------------
; Modules and Imports
; -------------------

(module_definition
  name: (identifier) @namespace)
  
(import_statement
  (identifier) @namespace)
  
(selected_import
  . (identifier) @namespace)

(scoped_identifier
  (identifier) @namespace)

; -------------------
; Function definition
; -------------------

(
  (function_definition
    name: [
      (identifier) @function
      (scoped_identifier
        (identifier) @namespace
        (identifier) @function)
    ])
  ; prevent constructors (PascalCase) to be highlighted as functions
  (#match? @function "^[^A-Z]"))

(
  (short_function_definition
    name: [
      (identifier) @function
      (scoped_identifier
        (identifier) @namespace
        (identifier) @function)
    ])
  ; prevent constructors (PascalCase) to be highlighted as functions
  (#match? @function "^[^A-Z]"))

; ---------------
; Functions calls
; ---------------

(
  (call_expression
    (identifier) @function)
  ; prevent constructors (PascalCase) to be highlighted as functions
  (#match? @function "^[^A-Z]"))

(
  (call_expression
    (field_expression (identifier) @function .))
  (#match? @function "^[^A-Z]"))

(
  (broadcast_call_expression
    (identifier) @function)
  (#match? @function "^[^A-Z]"))

(
  (broadcast_call_expression
    (field_expression (identifier) @function .))
  (#match? @function "^[^A-Z]"))


; -------------------
; Functions builtins
; -------------------

((identifier) @function.builtin
  (#any-of? @function.builtin
    "_abstracttype" "_apply_iterate" "_apply_pure" "_call_in_world" "_call_in_world_total"
    "_call_latest" "_equiv_typedef" "_expr" "_primitivetype" "_setsuper!" "_structtype" "_typebody!"
    "_typevar" "applicable" "apply_type" "arrayref" "arrayset" "arraysize" "const_arrayref"
    "donotdelete" "fieldtype" "get_binding_type" "getfield" "ifelse" "invoke" "isa" "isdefined"
    "modifyfield!" "nfields" "replacefield!" "set_binding_type!" "setfield!" "sizeof" "svec"
    "swapfield!" "throw" "tuple" "typeassert" "typeof"))

; -----------
; Parameters
; -----------

(parameter_list
  (identifier) @variable.parameter)

(optional_parameter
  . (identifier) @variable.parameter)

(slurp_parameter
  (identifier) @variable.parameter)

(typed_parameter
  parameter: (identifier)? @variable.parameter
  type: (_) @type)

(function_expression
  . (identifier) @variable.parameter) ; Single parameter arrow functions

; -----
; Types
; -----

; Definitions
(abstract_definition
  name: (identifier) @type.definition) @keyword

(primitive_definition
  name: (identifier) @type.definition) @keyword

(struct_definition
  name: (identifier) @type)

(struct_definition
  . (_)
    (identifier) @variable.other.member)

(struct_definition
  . (_)
  (typed_expression
    . (identifier) @variable.other.member))

(type_clause
  [
    (identifier) @type
    (field_expression
      (identifier) @type .)
  ])

; Annotations
(parametrized_type_expression
  (_) @type
  (curly_expression
    (_) @type))

(type_parameter_list
  (identifier) @type)

(typed_expression
  (identifier) @type . )

(function_definition
  return_type: (identifier) @type)

(short_function_definition
  return_type: (identifier) @type)

(where_clause
  (identifier) @type)

(where_clause
  (curly_expression
    (_) @type))

; ---------
; Builtins
; ---------

; This list was generated with:
;
;  istype(x) = typeof(x) === DataType || typeof(x) === UnionAll
;  get_types(m) = filter(x -> istype(Base.eval(m, x)), names(m))
;  type_names = sort(union(get_types(Core), get_types(Base)))
;
((identifier) @type.builtin
  (#any-of? @type.builtin
    "AbstractArray" "AbstractChannel" "AbstractChar" "AbstractDict" "AbstractDisplay"
    "AbstractFloat" "AbstractIrrational" "AbstractLock" "AbstractMatch" "AbstractMatrix"
    "AbstractPattern" "AbstractRange" "AbstractSet" "AbstractSlices" "AbstractString"
    "AbstractUnitRange" "AbstractVecOrMat" "AbstractVector" "Any" "ArgumentError" "Array"
    "AssertionError" "Atomic" "BigFloat" "BigInt" "BitArray" "BitMatrix" "BitSet" "BitVector" "Bool"
    "BoundsError" "By" "CanonicalIndexError" "CapturedException" "CartesianIndex" "CartesianIndices"
    "Cchar" "Cdouble" "Cfloat" "Channel" "Char" "Cint" "Cintmax_t" "Clong" "Clonglong" "Cmd" "Colon"
    "ColumnSlices" "Complex" "ComplexF16" "ComplexF32" "ComplexF64" "ComposedFunction"
    "CompositeException" "ConcurrencyViolationError" "Condition" "Cptrdiff_t" "Cshort" "Csize_t"
    "Cssize_t" "Cstring" "Cuchar" "Cuint" "Cuintmax_t" "Culong" "Culonglong" "Cushort" "Cvoid"
    "Cwchar_t" "Cwstring" "DataType" "DenseArray" "DenseMatrix" "DenseVecOrMat" "DenseVector" "Dict"
    "DimensionMismatch" "Dims" "DivideError" "DomainError" "EOFError" "Enum" "ErrorException"
    "Exception" "ExponentialBackOff" "Expr" "Float16" "Float32" "Float64" "Function" "GlobalRef"
    "HTML" "IO" "IOBuffer" "IOContext" "IOStream" "IdDict" "IndexCartesian" "IndexLinear"
    "IndexStyle" "InexactError" "InitError" "Int" "Int128" "Int16" "Int32" "Int64" "Int8" "Integer"
    "InterruptException" "InvalidStateException" "Irrational" "KeyError" "LazyString" "LinRange"
    "LineNumberNode" "LinearIndices" "LoadError" "Lt" "MIME" "Matrix" "Method" "MethodError"
    "Missing" "MissingException" "Module" "NTuple" "NamedTuple" "Nothing" "Number" "Ordering"
    "OrdinalRange" "OutOfMemoryError" "OverflowError" "Pair" "ParseError" "PartialQuickSort" "Perm"
    "PermutedDimsArray" "Pipe" "ProcessFailedException" "Ptr" "QuoteNode" "Rational" "RawFD"
    "ReadOnlyMemoryError" "Real" "ReentrantLock" "Ref" "Regex" "RegexMatch" "Returns"
    "ReverseOrdering" "RoundingMode" "RowSlices" "SegmentationFault" "Set" "Signed" "Slices" "Some"
    "SpinLock" "StackFrame" "StackOverflowError" "StackTrace" "Stateful" "StepRange" "StepRangeLen"
    "StridedArray" "StridedMatrix" "StridedVecOrMat" "StridedVector" "String" "StringIndexError"
    "SubArray" "SubString" "SubstitutionString" "Symbol" "SystemError" "Task" "TaskFailedException"
    "Text" "TextDisplay" "Timer" "Tmstruct" "Tuple" "Type" "TypeError" "TypeVar" "UInt" "UInt128"
    "UInt16" "UInt32" "UInt64" "UInt8" "UndefInitializer" "UndefKeywordError" "UndefRefError"
    "UndefVarError" "Union" "UnionAll" "UnitRange" "Unsigned" "Val" "VecElement" "VecOrMat" "Vector"
    "VersionNumber" "WeakKeyDict" "WeakRef"))

((identifier) @variable.builtin
  (#any-of? @variable.builtin "begin" "end"))

((identifier) @variable.builtin
  (#any-of? @variable.builtin "begin" "end"))


; --------
; Keywords
; --------

[
  "global"
  "local"
] @keyword

(compound_statement
  [
    "begin"
    "end"
  ] @keyword)

(quote_statement
  [
    "quote"
    "end"
  ] @keyword)

(let_statement
  [
    "let"
    "end"
  ] @keyword)

(if_statement
  [
    "if"
    "end"
  ] @keyword.control.conditional)

(elseif_clause
  "elseif" @keyword.control.conditional)

(else_clause
  "else" @keyword.control.conditional)

(if_clause
  "if" @keyword.control.conditional) ; `if` clause in comprehensions

(ternary_expression
  [
    "?"
    ":"
  ] @keyword.control.conditional)

(try_statement
  [
    "try"
    "end"
  ] @keyword.control.exception)

(finally_clause
  "finally" @keyword.control.exception)

(catch_clause
  "catch" @keyword.control.exception)

(for_statement
  [
    "for"
    "end"
  ] @keyword.control.repeat)

(while_statement
  [
    "while"
    "end"
  ] @keyword.control.repeat)

(for_clause
  "for" @keyword.control.repeat)

[
  (break_statement)
  (continue_statement)
] @keyword.control.repeat

(module_definition
  [
    "module"
    "baremodule"
    "end"
  ] @keyword.control.import)

(import_statement
  [
    "import"
    "using"
  ] @keyword.control.import)

(import_alias
  "as" @keyword.control.import)

(export_statement
  "export" @keyword.control.import)

(selected_import
  ":" @punctuation.delimiter)

(struct_definition
  [
    "struct"
    "end"
  ] @keyword)

(macro_definition
  [
    "macro"
    "end"
  ] @keyword)

(function_definition
  [
    "function"
    "end"
  ] @keyword.function)

(do_clause
  [
    "do"
    "end"
  ] @keyword.function)

(return_statement
  "return" @keyword.control.return)

[
  "const"
  "mutable"
] @keyword.storage.modifier

; ---------
; Operators
; ---------

[
  (operator)
  "="
  "∈"
] @operator

(adjoint_expression
  "'" @operator)

(range_expression
  ":" @operator)

((operator) @keyword.operator
  (#any-of? @keyword.operator "in" "isa"))

(for_binding
  "in" @keyword.operator)

(where_clause
  "where" @keyword.operator)

(where_expression
  "where" @keyword.operator)

(binary_expression
  (_)
  (operator) @operator
  (identifier) @function
  (#any-of? @operator "|>" ".|>"))

; ------------
; Punctuations
; ------------

[
  "."
  "," 
  ";"
  "::"
  "->"
] @punctuation.delimiter

"..." @punctuation.special

[
  "("
  ")" 
  "["
  "]"
  "{" 
  "}"
] @punctuation.bracket

; ---------
; Literals
; ---------

(boolean_literal) @constant.builtin.boolean

(integer_literal) @constant.numeric.integer

(float_literal) @constant.numeric.float

(
  ((identifier) @constant.numeric.float)
  (#match? @constant.numeric.float "^((Inf|NaN)(16|32|64)?)$"))

(
  ((identifier) @constant.builtin)
  (#match? @constant.builtin "^(nothing|missing|undef)$"))

(character_literal) @constant.character

(escape_sequence) @constant.character.escape

(string_literal) @string

(prefixed_string_literal
  prefix: (identifier) @function.macro) @string

(command_literal) @string

(prefixed_command_literal
  prefix: (identifier) @function.macro) @string

; ---------
; Comments
; ---------

[
  (line_comment)
  (block_comment)
] @comment
