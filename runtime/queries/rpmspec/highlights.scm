(variable_name) @variable

(macro_definition) @keyword.directive.define
(macro_invocation) @keyword.function
(macro_expansion) @keyword

[
  (tag)
  (dependency_tag)
] @type.definition

[
  (integer)
  (float)
] @number

(comment) @comment
(string) @string

(if_statement) @keyword

[
  "%description"
  "%package"
  (files)
  (changelog)
] @type.definition

[
  (prep_scriptlet)
  (generate_buildrequires)
  (conf_scriptlet)
  (build_scriptlet)
  (install_scriptlet)
  (check_scriptlet)
  (clean_scriptlet)
] @function.builtin

[
  "%artifact"
  "%attr"
  "%config"
  "%dir"
  "%doc"
  "%docdir"
  "%ghost"
  "%license"
  "%missingok"
  "%readme"
] @keyword.type

;[
;  "!="
;  "<"
;  "<="
;  "=="
;  ">"
;  ">="
;  "&&"
;  "||"
;] @operator

[
  "%if"
  "%ifarch"
  "%ifos"
  "%ifnarch"
  "%ifnos"
  "%elif"
  "%elifarch"
  "%elifos"
  "%else"
  "%endif"
] @keyword.conditional
