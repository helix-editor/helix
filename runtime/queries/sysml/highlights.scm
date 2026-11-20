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

(metadata_usage
  type: (qualified_name (_) @attribute .))

(alias_member
  name: (_) @namespace)

(import
  (qualified_name (_) @namespace .))

; Namespaces and definitions

(package
  name: (_) @namespace)

(library_package
  name: (_) @namespace)

(namespace
  name: (_) @namespace)


(part_definition name: (_) @type)
(item_definition name: (_) @type)
(attribute_definition name: (_) @type)
(enumeration_definition name: (_) @type)
(occurrence_definition name: (_) @type)
(port_definition name: (_) @type)
(connection_definition name: (_) @type)
(interface_definition name: (_) @type)
(allocation_definition name: (_) @type)
(flow_definition name: (_) @type)
(metadata_definition name: (_) @type)
(extended_definition name: (_) @type)
(state_definition name: (_) @type)
(constraint_definition name: (_) @type)
(requirement_definition name: (_) @type)
(concern_definition name: (_) @type)
(case_definition name: (_) @type)
(analysis_case_definition name: (_) @type)
(verification_case_definition name: (_) @type)
(use_case_definition name: (_) @type)
(view_definition name: (_) @type)
(viewpoint_definition name: (_) @type)
(rendering_definition name: (_) @type)

(action_definition name: (_) @function)
(calculation_definition name: (_) @function)

(part_definition short_name: (_) @type)
(item_definition short_name: (_) @type)
(attribute_definition short_name: (_) @type)
(enumeration_definition short_name: (_) @type)
(occurrence_definition short_name: (_) @type)
(port_definition short_name: (_) @type)
(connection_definition short_name: (_) @type)
(interface_definition short_name: (_) @type)
(allocation_definition short_name: (_) @type)
(flow_definition short_name: (_) @type)
(metadata_definition short_name: (_) @type)
(extended_definition short_name: (_) @type)
(state_definition short_name: (_) @type)
(constraint_definition short_name: (_) @type)
(requirement_definition short_name: (_) @type)
(concern_definition short_name: (_) @type)
(case_definition short_name: (_) @type)
(analysis_case_definition short_name: (_) @type)
(verification_case_definition short_name: (_) @type)
(use_case_definition short_name: (_) @type)
(view_definition short_name: (_) @type)
(viewpoint_definition short_name: (_) @type)
(rendering_definition short_name: (_) @type)
(action_definition short_name: (_) @function)
(calculation_definition short_name: (_) @function)

; Usages

(action_usage name: (_) @function)
(calculation_usage name: (_) @function)
(perform_action_usage name: (_) @function)
(state_usage name: (_) @label)
(exhibit_state_usage name: (_) @label)
(transition_usage name: (_) @label)
(enumeration_usage name: (_) @constant)
(variant_member (_ name: (_) @constant))

; Type references

(typed_by type: (qualified_name (_) @type .))

(typed_by type: (feature_chain (qualified_name (_) @type .) .))

(conjugated_port_reference (qualified_name (_) @type .))

(specializes (qualified_name (_) @type .))

(subsets (qualified_name (_) @variable.other.member .))

(redefines (qualified_name (_) @variable.other.member .))

(references (qualified_name (_) @variable.other.member .))

(crosses (qualified_name (_) @variable.other.member .))

(unions (qualified_name (_) @type .))

(intersects (qualified_name (_) @type .))

(differences (qualified_name (_) @type .))

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
  "expose"
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
  "decide"
  "merge"
  "fork"
  "join"
] @keyword.control.conditional

[
  "loop"
  "while"
  "until"
] @keyword.control.repeat

[
  "return"
  "terminate"
  "done"
] @keyword.control.return

[
  "public"
  "private"
  "protected"
  "abstract"
  "derived"
  "variation"
  "variant"
  "individual"
  "constant"
  "const"
  "var"
  "ref"
  "composite"
  "portion"
  "end"
  "ordered"
  "nonunique"
  "in"
  "out"
  "inout"
  "default"
  "all"
  "parallel"
  "snapshot"
  "timeslice"
  "chain"
] @keyword.storage.modifier

[
  "def"
  "part"
  "item"
  "attribute"
  "enum"
  "occurrence"
  "port"
  "connection"
  "interface"
  "allocation"
  "flow"
  "metadata"
  "state"
  "constraint"
  "requirement"
  "concern"
  "case"
  "analysis"
  "verification"
  "use"
  "view"
  "viewpoint"
  "rendering"
  "event"
  "message"
  "succession"
  "binding"
  "expr"
] @keyword.storage.type

[
  "action"
  "calc"
  "perform"
] @keyword.function

[
  "about"
  "accept"
  "actor"
  "after"
  "allocate"
  "assert"
  "assign"
  "assume"
  "at"
  "bind"
  "by"
  "choice"
  "comment"
  "connect"
  "crosses"
  "deep"
  "defer"
  "defined"
  "dependency"
  "differences"
  "do"
  "doc"
  "entry"
  "exhibit"
  "exit"
  "filter"
  "first"
  "frame"
  "from"
  "history"
  "include"
  "intersects"
  "junction"
  "language"
  "locale"
  "new"
  "objective"
  "of"
  "redefines"
  "references"
  "render"
  "rep"
  "require"
  "satisfy"
  "send"
  "shallow"
  "specializes"
  "stakeholder"
  "subject"
  "subsets"
  "to"
  "transition"
  "typed"
  "unions"
  "verify"
  "via"
  "when"
] @keyword
