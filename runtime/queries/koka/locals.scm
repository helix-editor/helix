(modulebody) @local.scope

(block) @local.scope

(pparameter
  (pattern
    (identifier
      (varid) @local.definition.variable.parameter)))

(puredecl
  (funid
    (identifier
      (varid) @local.definition.function)))

(puredecl
  (binder
    (identifier
      (varid) @local.definition.function)))

(decl
  (binder
    (identifier
      (varid) @local.definition.function)))

(identifier (varid) @local.reference)
