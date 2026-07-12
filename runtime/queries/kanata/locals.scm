(source_file) @local.scope
(defcfg) @local.scope
(defalias) @local.scope
(defvar) @local.scope
(deflayer) @local.scope
(deflayermap) @local.scope

(defalias
  name: (identifier) @local.definition.function)

(defvar
  name: (identifier) @local.definition.var)

(deflayer
  name: (identifier) @local.definition.namespace)

(deflayermap
  name: (identifier) @local.definition.namespace)


(alias_reference) @local.reference

(variable_reference) @local.reference

(list
  head: (identifier) @_action
  (#any-of? @_action "layer-switch" "layer-while-held" "layer-toggle")
  body: (identifier) @local.reference)
