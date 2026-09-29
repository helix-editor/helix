; Several X.680 lexical items are string aliases in grammar.js, so they are
; anonymous nodes. Those must be matched with "name", not (name).

; --- Comments ---

(line_comment) @comment.line
(block_comment) @comment.block

; --- Builtin types and useful object classes ---

[
  (BOOLEAN)
  (INTEGER)
  (ENUMERATED)
  (REAL)
  (SEQUENCE)
  (SET)
  (CHOICE)
  (NULL)
  (RELATIVE_OID)
  (OID_IRI)
  (RELATIVE_OID_IRI)
  (EXTERNAL)
  (TIME)
  (DATE)
  (DATE_TIME)
  (DURATION)
  (TIME_OF_DAY)
  (BMPString)
  (GeneralString)
  (GraphicString)
  (IA5String)
  (ISO646String)
  (NumericString)
  (PrintableString)
  (TeletexString)
  (T61String)
  (UniversalString)
  (UTF8String)
  (VideotexString)
  (VisibleString)
  (ANY)
  (TYPE_IDENTIFIER)
  (ABSTRACT_SYNTAX)

; Types using two keywords, except BIT STRING, because it can have named bits.

  (OctetStringType)
  (ObjectIdentifierType)
  (EmbeddedPDVType)
  (UnrestrictedCharacterStringType)

] @type.builtin

; --- Constants / booleans ---

[
  (TRUE)
  (FALSE)
  (true)
  (false)
  (xmltrue)
  (xmlfalse)
] @constant.builtin.boolean

[
  (PLUS_INFINITY)
  (MINUS_INFINITY)
  (NOT_A_NUMBER)
  (INF)
  (NaN)
  (xmlplusinfinity)
  (xmlminusinfinity)
  (xmlnotanumber)
] @constant.numeric.float

[
  (MIN)
  (MAX)
] @constant.builtin

; --- Keywords ---

[
  (DEFINITIONS)
  (BEGIN)
  (END)
  (EXPORTS)
  (IMPORTS)
  (FROM)
  (CLASS)
  (WITH)
  (SYNTAX)
  (SUCCESSORS)
  (DESCENDANTS)
  (INSTRUCTIONS)
  (ENCODING_CONTROL)
  (AUTOMATIC)
  (IMPLIED)
  (EXPLICIT)
  (IMPLICIT)
  (ALL)
  (TAGS)
  (EXTENSIBILITY)
  (UNIQUE)
  (CONTAINING)
  (OF)
  (DEFAULT)
  (OPTIONAL)
  (COMPONENTS)
  (COMPONENT)
  (EXCEPT)
  (INTERSECTION)
  (INCLUDES)
  (INSTANCE)
  (UNION)
  (PRESENT)
  (ABSENT)
  (PATTERN)
  (SETTINGS)
  (ENCODED)
  (BY)
  (CONSTRAINED)
  (SIZE)
  (UNIVERSAL)
  (APPLICATION)
  (PRIVATE)
  (DEFINED)
] @keyword

; WITH SYNTAX words (IDENTIFIED, PARMS, …)

"word" @keyword

; --- Names ---
;
; Types / object classes / object sets are uppercase; values / objects /
; identifiers are lowercase (X.680 12.2–12.5, X.681).

"typereference" @type
(objectsetreference) @type
"objectclassreference" @type
(yellcased_identifier) @type
(uppercased_identifier) @type

(modulereference) @namespace
(encodingreference) @namespace

(valuereference) @variable
(objectreference) @variable
(identifier) @variable
(lowercased_identifier) @variable
(any_identifier) @variable
"DummyGovernor" @variable.parameter

; IMPORTS/EXPORTS and dummy parameters use a single Reference token.
((Reference) @type
  (#match? @type "^[A-Z]"))
((Reference) @variable
  (#match? @variable "^[a-z]"))
(DummyReference
  (Reference) @variable.parameter)

; Information-object field names (&Type, &id, …)

[
  "typefieldreference"
  "valuefieldreference"
  "valuesetfieldreference"
  "objectfieldreference"
  "objectsetfieldreference"
  "PrimitiveFieldName"
] @variable.other.member

(anycased_field_ref) @variable.other.member

; Encoding-control notation that is not an X.680 lexical item

(foreign_lexical_item) @variable.builtin

; --- Literals ---

(number) @constant.numeric.integer
(realnumber) @constant.numeric.float

[
  (cstring)
  (tstring)
  (xmlcstring)
  (xmltstring)
] @string

[
  (bstring)
  (hstring)
  (xmlbstring)
  (xmlhstring)
] @string.special

(IRIValue) @string

; --- Operators and punctuation ---

[
  "::="
  "|"
  "^"
  ".."
  "..."
  "!"
  "="
  "@"
] @operator

[
  ","
  "."
  ":"
  ";"
] @punctuation.delimiter

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
  "[["
  "]]"
  "<"
  ">"
  "</"
  "/>"
] @punctuation.bracket
