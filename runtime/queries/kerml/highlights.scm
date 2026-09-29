; Comments and documentation

[
  (line_note)
  (block_note)
  (regular_comment)
] @comment

(documentation (regular_comment) @comment.block.documentation)

(comment (regular_comment) @comment.block.documentation)

; Literals

(string) @string

(literal_integer) @constant.numeric.integer

(literal_real) @constant.numeric.float

(literal_boolean) @constant.builtin.boolean

(literal_infinity) @constant.builtin

(null_expression) @constant.builtin

; Names

(identifier) @variable

(unrestricted_name) @variable

(qualified_name
  (_) @namespace
  "::")

(qualified_name
  "$" @variable.builtin)

(prefix_metadata
  (qualified_name (_) @attribute .))

(metadata_feature
  type: (qualified_name (_) @attribute .))

(alias_member
  name: (_) @namespace)

(import
  (qualified_name (_) @namespace .))

; Namespaces and types

(package
  name: (_) @namespace)

(library_package
  name: (_) @namespace)

(namespace
  name: (_) @namespace)


(type name: (_) @type)
(classifier name: (_) @type)
(data_type name: (_) @type)
(class name: (_) @type)
(structure name: (_) @type)
(association name: (_) @type)
(association_structure name: (_) @type)
(metaclass name: (_) @type)
(interaction name: (_) @type)
(behavior name: (_) @function)
(function name: (_) @function)
(predicate name: (_) @function)

(type short_name: (_) @type)
(classifier short_name: (_) @type)
(data_type short_name: (_) @type)
(class short_name: (_) @type)
(structure short_name: (_) @type)
(association short_name: (_) @type)
(association_structure short_name: (_) @type)
(metaclass short_name: (_) @type)
(interaction short_name: (_) @type)
(behavior short_name: (_) @function)
(function short_name: (_) @function)
(predicate short_name: (_) @function)

; Features

(step name: (_) @function)
(expression_feature name: (_) @function)
(invariant name: (_) @label)

; Type references

(typed_by type: (qualified_name (_) @type .))

(typed_by type: (feature_chain (qualified_name (_) @type .) .))

(feature_typing (qualified_name (_) @type .))

(specializes (qualified_name (_) @type .))

(specialization (qualified_name (_) @type .))

(subclassification (qualified_name (_) @type .))

(conjugates (qualified_name (_) @type .))

(conjugation (qualified_name (_) @type .))

(subsets (qualified_name (_) @variable.other.member .))

(subsetting (qualified_name (_) @variable.other.member .))

(redefines (qualified_name (_) @variable.other.member .))

(redefinition (qualified_name (_) @variable.other.member .))

(references (qualified_name (_) @variable.other.member .))

(crosses (qualified_name (_) @variable.other.member .))

(chains (qualified_name (_) @variable.other.member .))

(inverse_of (qualified_name (_) @variable.other.member .))

(featured_by (qualified_name (_) @type .))

(unions (qualified_name (_) @type .))

(intersects (qualified_name (_) @type .))

(differences (qualified_name (_) @type .))

(disjoint_from (qualified_name (_) @type .))

(classification_expression type: (qualified_name (_) @type .))

(extent_expression type: (qualified_name (_) @type .))

; Expressions

(constructor_expression
  (qualified_name (_) @type .))

(named_argument
  name: (qualified_name (_) @variable.parameter .))

(feature_chain_expression
  feature: (qualified_name (_) @variable.other.member .))

(feature_chain
  (qualified_name (_) @variable.other.member .))

; Calls

(invocation_expression
  function: (qualified_name (_) @function .))

(invocation_expression
  function: (feature_chain_expression feature: (qualified_name (_) @function .)))

(arrow_expression
  function: (qualified_name (_) @function .))

; Operators

[
  "+"
  "-"
  "*"
  "/"
  "%"
  "**"
  "^"
  "=="
  "!="
  "==="
  "!=="
  "<"
  "<="
  ">"
  ">="
  "&"
  "|"
  "??"
  "="
  ":="
  ".."
  "->"
  ".?"
  "~"
  "=>"
  ":>"
  ":>>"
  "::>"
  "@"
  "@@"
  "#"
  "?"
] @operator

[
  "and"
  "or"
  "xor"
  "not"
  "implies"
  "istype"
  "hastype"
  "as"
  "meta"
] @keyword.operator

; Punctuation

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ";"
  ","
  "."
  ":"
  "::"
  "::*"
  "::**"
] @punctuation.delimiter

; Keywords

[
  "import"
  "alias"
  "for"
] @keyword.control.import

[
  "package"
  "library"
  "namespace"
  "standard"
] @keyword

[
  "if"
  "else"
  "then"
] @keyword.control.conditional

[
  "return"
  "done"
] @keyword.control.return

[
  "public"
  "private"
  "protected"
  "abstract"
  "derived"
  "composite"
  "portion"
  "const"
  "var"
  "end"
  "ordered"
  "nonunique"
  "in"
  "out"
  "inout"
  "default"
  "all"
  "member"
  "chain"
] @keyword.storage.modifier

[
  "type"
  "classifier"
  "datatype"
  "class"
  "struct"
  "assoc"
  "metaclass"
  "interaction"
  "feature"
  "connector"
  "binding"
  "succession"
  "flow"
  "multiplicity"
  "expr"
  "bool"
  "inv"
] @keyword.storage.type

[
  "behavior"
  "function"
  "predicate"
  "step"
] @keyword.function

[
  "about"
  "by"
  "chains"
  "comment"
  "conjugate"
  "conjugates"
  "conjugation"
  "crosses"
  "defined"
  "dependency"
  "differences"
  "disjoining"
  "disjoint"
  "doc"
  "featured"
  "featuring"
  "filter"
  "first"
  "from"
  "intersects"
  "inverse"
  "inverting"
  "language"
  "locale"
  "metadata"
  "new"
  "of"
  "redefines"
  "redefinition"
  "references"
  "rep"
  "specialization"
  "specializes"
  "subclassifier"
  "subset"
  "subsets"
  "subtype"
  "to"
  "typed"
  "typing"
  "unions"
] @keyword
